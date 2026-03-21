use std::path::{Path, PathBuf};

use oxrdf::{BlankNode, NamedNode};
use sparkdown_core::parser::SparkdownParser;
use sparkdown_ontology::registry::ThemeRegistry;
use sparkdown_overlay::anchor::AnchorStatus;
use sparkdown_overlay::graph::SemanticGraph;
use sparkdown_overlay::mapping::MappingIndex;
use sparkdown_overlay::sidecar;
use sparkdown_overlay::sync;
use tauri::AppHandle;
use tokio::sync::mpsc;

use crate::events;
use crate::registry::SessionCommand;
use crate::types::{
    byte_to_char_offset, char_to_byte_offset, DocId, EntityDetailDto, EntityDto, EntityStatus,
    IncomingRelation, PropertyDto, Relation, SidecarStatus,
};

/// Owns all state for a single open document. Runs as a tokio task.
pub struct DocumentSession {
    doc_id: DocId,
    file_path: PathBuf,
    source: String,
    graph: SemanticGraph,
    index: MappingIndex,
    parser: SparkdownParser,
    _registry: ThemeRegistry,
    app: AppHandle,
    /// Monotonic counter for generating unique entity IDs.
    next_entity_id: u32,
    /// Tracks the file's mtime at last load or save, for external modification detection.
    last_known_mtime: Option<std::time::SystemTime>,
}

impl DocumentSession {
    /// Spawn a new session for the given document path.
    /// Returns the mpsc sender for sending commands to this session.
    pub async fn open(
        app: AppHandle,
        file_path: PathBuf,
    ) -> Result<(DocId, mpsc::Sender<SessionCommand>), String> {
        let doc_id: DocId = file_path
            .canonicalize()
            .map_err(|e| format!("Cannot resolve path: {e}"))?
            .to_string_lossy()
            .into_owned();

        // Read source
        let source = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Cannot read file: {e}"))?;

        // Parse source
        let parser = SparkdownParser::new();
        if let Err(e) = parser.parse(&source) {
            events::emit_parse_error(
                &app,
                events::ErrorPayload {
                    doc_id: doc_id.clone(),
                    message: e.to_string(),
                },
            );
        }

        // Load sidecar
        let sidecar_path = sidecar_path_for(&file_path);
        let graph = if sidecar_path.exists() {
            match tokio::fs::read_to_string(&sidecar_path).await {
                Ok(content) => match sidecar::parse(&content) {
                    Ok(g) => g,
                    Err(e) => {
                        events::emit_sidecar_error(
                            &app,
                            events::ErrorPayload {
                                doc_id: doc_id.clone(),
                                message: e.to_string(),
                            },
                        );
                        SemanticGraph::new([0u8; 32])
                    }
                },
                Err(e) => {
                    events::emit_sidecar_error(
                        &app,
                        events::ErrorPayload {
                            doc_id: doc_id.clone(),
                            message: e.to_string(),
                        },
                    );
                    SemanticGraph::new([0u8; 32])
                }
            }
        } else {
            SemanticGraph::new([0u8; 32])
        };

        let index = MappingIndex::build(&graph);
        let registry = ThemeRegistry::with_builtins();

        // Determine next entity ID from existing graph
        let next_entity_id = graph
            .entities
            .iter()
            .filter_map(|e| {
                e.id.as_str()
                    .strip_prefix("e")
                    .and_then(|n| n.parse::<u32>().ok())
            })
            .max()
            .map(|m| m + 1)
            .unwrap_or(1);

        // Record file mtime for modification detection
        let last_known_mtime = tokio::fs::metadata(&file_path)
            .await
            .ok()
            .and_then(|m| m.modified().ok());

        let (tx, rx) = mpsc::channel::<SessionCommand>(32);

        let session = DocumentSession {
            doc_id: doc_id.clone(),
            file_path,
            source,
            graph,
            index,
            parser,
            _registry: registry,
            app: app.clone(),
            next_entity_id,
            last_known_mtime,
        };

        // Emit initial state
        let entities = session.build_entity_dtos();
        let sidecar_status = session.build_sidecar_status();
        events::emit_document_opened(
            &app,
            events::DocumentOpenedPayload {
                doc_id: doc_id.clone(),
                entities,
                sidecar_status,
            },
        );

        // Spawn actor task
        tokio::spawn(session.run(rx));

        Ok((doc_id, tx))
    }

    async fn run(mut self, mut rx: mpsc::Receiver<SessionCommand>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                SessionCommand::UpdateSource { new_source, reply } => {
                    self.handle_update_source(new_source);
                    let _ = reply.send(());
                }
                SessionCommand::GetEntitiesAt { start, end, reply } => {
                    let entities = self.get_entities_at(start, end);
                    let _ = reply.send(entities);
                }
                SessionCommand::ExportAs { format, reply } => {
                    let result = self.handle_export(format);
                    let _ = reply.send(result);
                }
                SessionCommand::Save { reply } => {
                    let result = self.handle_save().await;
                    let _ = reply.send(result);
                }
                SessionCommand::Close => break,

                // Phase 2 commands
                SessionCommand::CreateEntity {
                    span_start,
                    span_end,
                    type_iri,
                    reply,
                } => {
                    let result = self.handle_create_entity(span_start, span_end, &type_iri);
                    let _ = reply.send(result);
                }
                SessionCommand::UpdateStaleAnchor { entity_id, reply } => {
                    let result = self.handle_update_stale_anchor(&entity_id);
                    let _ = reply.send(result);
                }
                SessionCommand::DismissSuggestion { entity_id, reply } => {
                    let result = self.handle_dismiss_suggestion(&entity_id);
                    let _ = reply.send(result);
                }
                SessionCommand::GetAllEntities { reply } => {
                    let entities = self.build_entity_dtos();
                    let _ = reply.send(entities);
                }
                SessionCommand::GetEntityDetails { entity_id, reply } => {
                    let result = self.handle_get_entity_details(&entity_id);
                    let _ = reply.send(result);
                }
                SessionCommand::AddTriple {
                    subject_id,
                    predicate_iri,
                    object_value,
                    object_is_entity,
                    reply,
                } => {
                    let result = self.handle_add_triple(
                        &subject_id,
                        &predicate_iri,
                        &object_value,
                        object_is_entity,
                    );
                    let _ = reply.send(result);
                }
                SessionCommand::CheckFileModified { reply } => {
                    let result = self.handle_check_file_modified().await;
                    let _ = reply.send(result);
                }
            }
        }
    }

    fn handle_update_source(&mut self, new_source: String) {
        // Reparse
        if let Err(e) = self.parser.parse(&new_source) {
            events::emit_parse_error(
                &self.app,
                events::ErrorPayload {
                    doc_id: self.doc_id.clone(),
                    message: e.to_string(),
                },
            );
            return; // Keep old state on parse error
        }

        // Sync graph
        sync::sync_graph(&mut self.graph, &self.source, &new_source);

        // Update source and rebuild index
        self.source = new_source;
        self.index = MappingIndex::build(&self.graph);

        // Emit updated entities
        let entities = self.build_entity_dtos();
        events::emit_entities_updated(
            &self.app,
            events::EntitiesUpdatedPayload {
                doc_id: self.doc_id.clone(),
                entities,
            },
        );

        // Emit sidecar status
        let status = self.build_sidecar_status();
        events::emit_sidecar_status(
            &self.app,
            events::SidecarStatusPayload {
                doc_id: self.doc_id.clone(),
                status,
            },
        );

        // Emit stale anchors if any
        let stale: Vec<_> = self
            .graph
            .entities
            .iter()
            .filter(|e| e.status == AnchorStatus::Stale)
            .map(|e| {
                let span_start = byte_to_char_offset(&self.source, e.anchor.span.start);
                let span_end = byte_to_char_offset(
                    &self.source,
                    e.anchor.span.end.min(self.source.len()),
                );
                crate::types::StaleAnchor {
                    entity_id: e.id.as_str().to_string(),
                    old_snippet: e.anchor.snippet.clone(),
                    new_text: self
                        .source
                        .get(e.anchor.span.start..e.anchor.span.end.min(self.source.len()))
                        .unwrap_or("")
                        .chars()
                        .take(40)
                        .collect(),
                    span_start,
                    span_end,
                }
            })
            .collect();

        if !stale.is_empty() {
            events::emit_stale_anchors(
                &self.app,
                events::StaleAnchorsPayload {
                    doc_id: self.doc_id.clone(),
                    anchors: stale,
                },
            );
        }
    }

    fn get_entities_at(&self, start: usize, end: usize) -> Vec<EntityDto> {
        let blank_nodes = self.index.entities_at(start..end);
        blank_nodes
            .iter()
            .filter_map(|bn| {
                let entity = self.graph.entity_by_id(bn)?;
                Some(self.entity_to_dto(entity))
            })
            .collect()
    }

    fn handle_export(&self, format: crate::types::RenderFormat) -> Result<String, String> {
        use sparkdown_render::traits::OutputRenderer;

        let doc = self
            .parser
            .parse(&self.source)
            .map_err(|e| e.to_string())?;

        let mut buf = Vec::new();
        match format {
            crate::types::RenderFormat::HtmlRdfa => {
                sparkdown_render::html_rdfa::HtmlRdfaRenderer::new()
                    .render(&doc, &mut buf)
                    .map_err(|e| e.to_string())?;
            }
            crate::types::RenderFormat::JsonLd => {
                sparkdown_render::jsonld::JsonLdRenderer::new()
                    .render(&doc, &mut buf)
                    .map_err(|e| e.to_string())?;
            }
            crate::types::RenderFormat::Turtle => {
                sparkdown_render::turtle::TurtleRenderer::new()
                    .render(&doc, &mut buf)
                    .map_err(|e| e.to_string())?;
            }
        }

        String::from_utf8(buf).map_err(|e| e.to_string())
    }

    // --- Phase 2 handlers ---

    fn handle_create_entity(
        &mut self,
        span_start: usize,
        span_end: usize,
        type_iri: &str,
    ) -> Result<EntityDto, String> {
        use sparkdown_overlay::anchor::Anchor;
        use sparkdown_overlay::graph::SemanticEntity;

        // Convert char offsets from CM to byte offsets
        let byte_start = char_to_byte_offset(&self.source, span_start);
        let byte_end = char_to_byte_offset(&self.source, span_end);

        let snippet: String = self
            .source
            .get(byte_start..byte_end.min(self.source.len()))
            .unwrap_or("")
            .chars()
            .take(40)
            .collect();

        let entity_id_str = format!("e{}", self.next_entity_id);
        self.next_entity_id += 1;

        let blank_node = BlankNode::new(&entity_id_str)
            .map_err(|e| format!("Invalid blank node: {e}"))?;

        let named_node = NamedNode::new(type_iri)
            .map_err(|e| format!("Invalid IRI: {e}"))?;

        let entity = SemanticEntity {
            id: blank_node,
            anchor: Anchor::new(byte_start..byte_end, &snippet),
            types: vec![named_node],
            status: AnchorStatus::Synced,
        };

        self.graph.entities.push(entity);
        self.index = MappingIndex::build(&self.graph);

        // Emit updated state
        let entities = self.build_entity_dtos();
        events::emit_entities_updated(
            &self.app,
            events::EntitiesUpdatedPayload {
                doc_id: self.doc_id.clone(),
                entities: entities.clone(),
            },
        );
        let status = self.build_sidecar_status();
        events::emit_sidecar_status(
            &self.app,
            events::SidecarStatusPayload {
                doc_id: self.doc_id.clone(),
                status,
            },
        );

        // Return the newly created entity DTO
        let new_entity = self
            .graph
            .entities
            .last()
            .map(|e| self.entity_to_dto(e))
            .ok_or("Entity creation failed")?;

        Ok(new_entity)
    }

    fn handle_update_stale_anchor(&mut self, entity_id: &str) -> Result<(), String> {
        let blank_node = BlankNode::new(entity_id)
            .map_err(|e| format!("Invalid entity ID: {e}"))?;

        let entity = self
            .graph
            .entity_by_id_mut(&blank_node)
            .ok_or_else(|| format!("Entity not found: {entity_id}"))?;

        if entity.status != AnchorStatus::Stale {
            return Err("Entity is not stale".into());
        }

        // Update the snippet to the current text at the anchor span
        let new_snippet: String = self
            .source
            .get(entity.anchor.span.start..entity.anchor.span.end.min(self.source.len()))
            .unwrap_or("")
            .chars()
            .take(40)
            .collect();
        entity.anchor.snippet = new_snippet;
        entity.status = AnchorStatus::Synced;

        self.index = MappingIndex::build(&self.graph);

        // Emit updated state
        let entities = self.build_entity_dtos();
        events::emit_entities_updated(
            &self.app,
            events::EntitiesUpdatedPayload {
                doc_id: self.doc_id.clone(),
                entities,
            },
        );
        let status = self.build_sidecar_status();
        events::emit_sidecar_status(
            &self.app,
            events::SidecarStatusPayload {
                doc_id: self.doc_id.clone(),
                status,
            },
        );

        Ok(())
    }

    fn handle_dismiss_suggestion(&mut self, entity_id: &str) -> Result<(), String> {
        let blank_node = BlankNode::new(entity_id)
            .map_err(|e| format!("Invalid entity ID: {e}"))?;

        // Remove entity and its triples
        self.graph
            .entities
            .retain(|e| e.id.as_str() != blank_node.as_str());
        self.graph
            .triples
            .retain(|t| t.subject.as_str() != blank_node.as_str());

        self.index = MappingIndex::build(&self.graph);

        let entities = self.build_entity_dtos();
        events::emit_entities_updated(
            &self.app,
            events::EntitiesUpdatedPayload {
                doc_id: self.doc_id.clone(),
                entities,
            },
        );
        let status = self.build_sidecar_status();
        events::emit_sidecar_status(
            &self.app,
            events::SidecarStatusPayload {
                doc_id: self.doc_id.clone(),
                status,
            },
        );

        Ok(())
    }

    fn handle_get_entity_details(&self, entity_id: &str) -> Result<EntityDetailDto, String> {
        let blank_node = BlankNode::new(entity_id)
            .map_err(|e| format!("Invalid entity ID: {e}"))?;

        let entity = self
            .graph
            .entity_by_id(&blank_node)
            .ok_or_else(|| format!("Entity not found: {entity_id}"))?;

        let span_start = byte_to_char_offset(&self.source, entity.anchor.span.start);
        let span_end = byte_to_char_offset(
            &self.source,
            entity.anchor.span.end.min(self.source.len()),
        );

        let type_iris: Vec<String> = entity.types.iter().map(|t| t.as_str().to_string()).collect();
        let type_prefix = entity
            .types
            .first()
            .map(|t| iri_to_curie(t.as_str()))
            .unwrap_or_default();
        let status = match entity.status {
            AnchorStatus::Synced => EntityStatus::Synced,
            AnchorStatus::Stale => EntityStatus::Stale,
            AnchorStatus::Detached => EntityStatus::Detached,
        };

        // All triples where this entity is the subject
        let subject_triples = self.graph.triples_for_subject(&blank_node);
        let mut properties = Vec::new();

        for t in &subject_triples {
            match &t.object {
                sparkdown_overlay::graph::TripleObject::Literal { value, .. } => {
                    properties.push(PropertyDto {
                        predicate_label: iri_local_name(t.predicate.as_str()),
                        predicate_iri: t.predicate.as_str().to_string(),
                        value: value.clone(),
                    });
                }
                _ => {}
            }
        }

        // All triples where this entity is referenced as object
        let mut incoming_relations = Vec::new();
        for t in &self.graph.triples {
            if let sparkdown_overlay::graph::TripleObject::Entity(ref obj_bn) = t.object {
                if obj_bn.as_str() == entity_id {
                    let subject_label = self
                        .graph
                        .entity_by_id(&t.subject)
                        .map(|e| e.anchor.snippet.clone())
                        .unwrap_or_else(|| t.subject.as_str().to_string());
                    incoming_relations.push(IncomingRelation {
                        subject_id: t.subject.as_str().to_string(),
                        subject_label,
                        predicate_label: iri_local_name(t.predicate.as_str()),
                    });
                }
            }
        }

        Ok(EntityDetailDto {
            id: entity.id.as_str().to_string(),
            label: entity.anchor.snippet.clone(),
            type_iris,
            type_prefix,
            span_start,
            span_end,
            status,
            properties,
            incoming_relations,
        })
    }

    fn handle_add_triple(
        &mut self,
        subject_id: &str,
        predicate_iri: &str,
        object_value: &str,
        object_is_entity: bool,
    ) -> Result<(), String> {
        use sparkdown_overlay::graph::{Triple, TripleObject};

        let subject = BlankNode::new(subject_id)
            .map_err(|e| format!("Invalid subject ID: {e}"))?;
        let predicate = NamedNode::new(predicate_iri)
            .map_err(|e| format!("Invalid predicate IRI: {e}"))?;

        // Verify subject entity exists
        if self.graph.entity_by_id(&subject).is_none() {
            return Err(format!("Subject entity not found: {subject_id}"));
        }

        let object = if object_is_entity {
            let obj_bn = BlankNode::new(object_value)
                .map_err(|e| format!("Invalid object entity ID: {e}"))?;
            TripleObject::Entity(obj_bn)
        } else {
            TripleObject::Literal {
                value: object_value.to_string(),
                datatype: None,
            }
        };

        self.graph.triples.push(Triple {
            subject,
            predicate,
            object,
        });

        // Emit updated state
        let entities = self.build_entity_dtos();
        events::emit_entities_updated(
            &self.app,
            events::EntitiesUpdatedPayload {
                doc_id: self.doc_id.clone(),
                entities,
            },
        );
        let status = self.build_sidecar_status();
        events::emit_sidecar_status(
            &self.app,
            events::SidecarStatusPayload {
                doc_id: self.doc_id.clone(),
                status,
            },
        );

        Ok(())
    }

    async fn handle_check_file_modified(&self) -> Result<bool, String> {
        let meta = tokio::fs::metadata(&self.file_path)
            .await
            .map_err(|e| format!("Cannot stat file: {e}"))?;

        let current_mtime = meta.modified().map_err(|e| format!("No mtime: {e}"))?;

        Ok(self
            .last_known_mtime
            .map(|known| current_mtime != known)
            .unwrap_or(false))
    }

    // --- End Phase 2 handlers ---

    async fn handle_save(&mut self) -> Result<(), String> {
        // Write source
        tokio::fs::write(&self.file_path, &self.source)
            .await
            .map_err(|e| format!("Failed to save source: {e}"))?;

        // Write sidecar
        let sidecar_content = sidecar::serialize(&self.graph);
        let sidecar_path = sidecar_path_for(&self.file_path);
        tokio::fs::write(&sidecar_path, &sidecar_content)
            .await
            .map_err(|e| format!("Failed to save sidecar: {e}"))?;

        // Update tracked mtime
        self.last_known_mtime = tokio::fs::metadata(&self.file_path)
            .await
            .ok()
            .and_then(|m| m.modified().ok());

        Ok(())
    }

    fn build_entity_dtos(&self) -> Vec<EntityDto> {
        self.graph
            .entities
            .iter()
            .map(|entity| self.entity_to_dto(entity))
            .collect()
    }

    fn entity_to_dto(&self, entity: &sparkdown_overlay::graph::SemanticEntity) -> EntityDto {
        let span_start = byte_to_char_offset(&self.source, entity.anchor.span.start);
        let span_end = byte_to_char_offset(
            &self.source,
            entity.anchor.span.end.min(self.source.len()),
        );

        let type_iris: Vec<String> = entity.types.iter().map(|t| t.as_str().to_string()).collect();

        let type_prefix = entity
            .types
            .first()
            .map(|t| iri_to_curie(t.as_str()))
            .unwrap_or_default();

        let status = match entity.status {
            AnchorStatus::Synced => EntityStatus::Synced,
            AnchorStatus::Stale => EntityStatus::Stale,
            AnchorStatus::Detached => EntityStatus::Detached,
        };

        // Build top 2 relations
        let triples = self.graph.triples_for_subject(&entity.id);
        let top_relations: Vec<Relation> = triples
            .iter()
            .take(2)
            .filter_map(|t| {
                let predicate_label = iri_local_name(t.predicate.as_str());
                let (target_label, target_id) = match &t.object {
                    sparkdown_overlay::graph::TripleObject::Entity(bn) => {
                        let label = self
                            .graph
                            .entity_by_id(bn)
                            .map(|e| e.anchor.snippet.clone())
                            .unwrap_or_else(|| bn.as_str().to_string());
                        (label, bn.as_str().to_string())
                    }
                    sparkdown_overlay::graph::TripleObject::Literal { value, .. } => {
                        (value.clone(), String::new())
                    }
                };
                Some(Relation {
                    predicate_label,
                    target_label,
                    target_id,
                })
            })
            .collect();

        EntityDto {
            id: entity.id.as_str().to_string(),
            label: entity.anchor.snippet.clone(),
            type_iris,
            type_prefix,
            span_start,
            span_end,
            status,
            top_relations,
        }
    }

    fn build_sidecar_status(&self) -> SidecarStatus {
        let synced = self
            .graph
            .entities
            .iter()
            .filter(|e| e.status == AnchorStatus::Synced)
            .count();
        let stale = self
            .graph
            .entities
            .iter()
            .filter(|e| e.status == AnchorStatus::Stale)
            .count();
        let detached = self
            .graph
            .entities
            .iter()
            .filter(|e| e.status == AnchorStatus::Detached)
            .count();
        SidecarStatus {
            synced,
            stale,
            detached,
            total_triples: self.graph.triples.len(),
        }
    }
}

/// Get the sidecar path for a .md file: same name with .sparkdown-sem extension.
fn sidecar_path_for(md_path: &Path) -> PathBuf {
    md_path.with_extension("sparkdown-sem")
}

/// Convert a full IRI to a CURIE-like display string.
/// e.g. "http://schema.org/Person" -> "schema:Person"
fn iri_to_curie(iri: &str) -> String {
    let known = [
        ("http://schema.org/", "schema:"),
        ("http://purl.org/dc/terms/", "dc:"),
        ("http://xmlns.com/foaf/0.1/", "foaf:"),
    ];
    for (base, prefix) in &known {
        if let Some(local) = iri.strip_prefix(base) {
            return format!("{prefix}{local}");
        }
    }
    iri.to_string()
}

/// Extract the local name from an IRI (everything after last / or #).
fn iri_local_name(iri: &str) -> String {
    iri.rsplit_once('#')
        .or_else(|| iri.rsplit_once('/'))
        .map(|(_, local)| local.to_string())
        .unwrap_or_else(|| iri.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iri_to_curie_known_prefix() {
        assert_eq!(iri_to_curie("http://schema.org/Person"), "schema:Person");
        assert_eq!(
            iri_to_curie("http://purl.org/dc/terms/title"),
            "dc:title"
        );
    }

    #[test]
    fn iri_to_curie_unknown_returns_raw() {
        assert_eq!(
            iri_to_curie("http://example.org/Custom"),
            "http://example.org/Custom"
        );
    }

    #[test]
    fn iri_local_name_extracts_after_slash() {
        assert_eq!(iri_local_name("http://schema.org/Person"), "Person");
    }

    #[test]
    fn iri_local_name_extracts_after_hash() {
        assert_eq!(
            iri_local_name("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
            "type"
        );
    }

    #[test]
    fn sidecar_path_replaces_extension() {
        let p = sidecar_path_for(Path::new("/notes/meeting.md"));
        assert_eq!(p, PathBuf::from("/notes/meeting.sparkdown-sem"));
    }
}
