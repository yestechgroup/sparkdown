use std::io::Write;

use serde_json::{json, Map, Value};
use sparkdown_core::annotation::AnnotationKind;
use sparkdown_core::ast::{NodeKind, SemanticNode, SparkdownDocument};

use crate::traits::{OutputRenderer, RenderError};

pub struct JsonLdRenderer;

impl JsonLdRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonLdRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for JsonLdRenderer {
    fn render(
        &self,
        doc: &SparkdownDocument,
        out: &mut dyn Write,
    ) -> Result<(), RenderError> {
        // Build @context
        let mut context = Map::new();
        for (prefix, iri) in doc.prefixes.iter() {
            context.insert(prefix.to_string(), Value::String(iri.to_string()));
        }

        // Collect entities from the AST
        let mut entities = Vec::new();

        // Document-level entity from frontmatter
        if doc.frontmatter.doc_type.is_some() || doc.frontmatter.title.is_some() {
            let mut entity = Map::new();
            if let Some(ref doc_type) = doc.frontmatter.doc_type {
                entity.insert("@type".to_string(), Value::String(doc_type.clone()));
            }
            if let Some(ref title) = doc.frontmatter.title {
                entity.insert("schema:name".to_string(), Value::String(title.clone()));
            }
            entities.push(Value::Object(entity));
        }

        // Walk the AST for annotated nodes
        collect_entities(&doc.nodes, &mut entities);

        let output = if entities.len() == 1 && doc.frontmatter.doc_type.is_some() {
            // Single entity — flatten
            let mut root = entities.into_iter().next().unwrap();
            if let Value::Object(ref mut obj) = root {
                obj.insert("@context".to_string(), Value::Object(context));
            }
            root
        } else {
            // Multiple entities — use @graph
            json!({
                "@context": context,
                "@graph": entities,
            })
        };

        let json_str = serde_json::to_string_pretty(&output)
            .map_err(|e| RenderError::Other(e.to_string()))?;
        write!(out, "{json_str}")?;

        Ok(())
    }

    fn content_type(&self) -> &str {
        "application/ld+json"
    }

    fn file_extension(&self) -> &str {
        "jsonld"
    }
}

fn collect_entities(nodes: &[SemanticNode], entities: &mut Vec<Value>) {
    for node in nodes {
        let has_type = node
            .annotations
            .iter()
            .any(|a| matches!(&a.kind, AnnotationKind::TypeAssignment { .. }));

        if has_type {
            let mut entity = Map::new();

            for ann in &node.annotations {
                match &ann.kind {
                    AnnotationKind::TypeAssignment { resolved_iri, raw } => {
                        let type_str = resolved_iri
                            .as_ref()
                            .map(|n| n.as_str().to_string())
                            .unwrap_or_else(|| raw.clone());
                        entity.insert("@type".to_string(), Value::String(type_str));
                    }
                    AnnotationKind::Property {
                        resolved_iri,
                        raw_key,
                        value,
                    } => {
                        let key = resolved_iri
                            .as_ref()
                            .map(|n| n.as_str().to_string())
                            .unwrap_or_else(|| raw_key.clone());
                        entity.insert(key, Value::String(value.clone()));
                    }
                    AnnotationKind::ExternalId {
                        system,
                        identifier,
                    } => {
                        let same_as = match system.as_str() {
                            "wikidata" => {
                                format!("http://www.wikidata.org/entity/{identifier}")
                            }
                            "doi" => format!("https://doi.org/{identifier}"),
                            "orcid" => format!("https://orcid.org/{identifier}"),
                            _ => identifier.clone(),
                        };
                        entity.insert(
                            "owl:sameAs".to_string(),
                            Value::String(same_as),
                        );
                    }
                    _ => {}
                }
            }

            // Add name from text content if available
            match &node.kind {
                NodeKind::Section { id, .. } => {
                    let text = node.text_content();
                    if !text.is_empty() && !entity.contains_key("schema:name") {
                        entity.insert(
                            "schema:name".to_string(),
                            Value::String(text.trim().to_string()),
                        );
                    }
                    if let Some(id) = id {
                        entity.insert("@id".to_string(), Value::String(format!("#{id}")));
                    }
                }
                NodeKind::InlineDirective { .. } => {
                    let text = node.text_content();
                    if !text.is_empty() && !entity.contains_key("schema:name") {
                        entity.insert(
                            "schema:name".to_string(),
                            Value::String(text.trim().to_string()),
                        );
                    }
                }
                _ => {}
            }

            entities.push(Value::Object(entity));
        }

        // Recurse into children
        collect_entities(&node.children, entities);
    }
}
