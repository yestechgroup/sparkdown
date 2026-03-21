use std::path::{Path, PathBuf};

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
    byte_to_char_offset, DocId, EntityDto, EntityStatus, Relation, SidecarStatus,
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

    async fn handle_save(&self) -> Result<(), String> {
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
