use std::io::Write;

use sparkdown_core::annotation::AnnotationKind;
use sparkdown_core::ast::{NodeKind, SemanticNode, SparkdownDocument};

use crate::traits::{OutputRenderer, RenderError};

pub struct TurtleRenderer;

impl TurtleRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TurtleRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for TurtleRenderer {
    fn render(
        &self,
        doc: &SparkdownDocument,
        out: &mut dyn Write,
    ) -> Result<(), RenderError> {
        // Emit prefix declarations
        for (prefix, iri) in doc.prefixes.iter() {
            writeln!(out, "@prefix {prefix}: <{iri}> .")?;
        }
        writeln!(out)?;

        // Collect triples from the AST
        let mut triples = Vec::new();
        collect_triples(&doc.nodes, &mut triples);

        // Also emit document-level triples from frontmatter
        if let Some(ref doc_type) = doc.frontmatter.doc_type {
            let subject = "<>".to_string(); // document itself
            triples.push(Triple {
                subject: subject.clone(),
                predicate: "a".to_string(),
                object: resolve_or_curie(doc_type),
            });
            if let Some(ref title) = doc.frontmatter.title {
                triples.push(Triple {
                    subject,
                    predicate: "schema:name".to_string(),
                    object: format!("\"{}\"", escape_turtle(title)),
                });
            }
        }

        // Group triples by subject
        let mut subjects: Vec<String> = Vec::new();
        let mut grouped: std::collections::HashMap<String, Vec<(String, String)>> =
            std::collections::HashMap::new();

        for triple in &triples {
            if !grouped.contains_key(&triple.subject) {
                subjects.push(triple.subject.clone());
            }
            grouped
                .entry(triple.subject.clone())
                .or_default()
                .push((triple.predicate.clone(), triple.object.clone()));
        }

        for subject in &subjects {
            let predicates = &grouped[subject];
            write!(out, "{subject}")?;
            for (i, (pred, obj)) in predicates.iter().enumerate() {
                if i == 0 {
                    write!(out, " {pred} {obj}")?;
                } else {
                    write!(out, " ;\n    {pred} {obj}")?;
                }
            }
            writeln!(out, " .")?;
            writeln!(out)?;
        }

        Ok(())
    }

    fn content_type(&self) -> &str {
        "text/turtle"
    }

    fn file_extension(&self) -> &str {
        "ttl"
    }
}

struct Triple {
    subject: String,
    predicate: String,
    object: String,
}

fn collect_triples(nodes: &[SemanticNode], triples: &mut Vec<Triple>) {
    for node in nodes {
        let has_type = node
            .annotations
            .iter()
            .any(|a| matches!(&a.kind, AnnotationKind::TypeAssignment { .. }));

        if has_type {
            // Determine subject
            let subject = match &node.kind {
                NodeKind::Section { id: Some(id), .. } => format!("<#{id}>"),
                NodeKind::Section { .. } => {
                    let text = node.text_content().trim().to_string();
                    let slug = slugify(&text);
                    format!("<#{slug}>")
                }
                NodeKind::InlineDirective { .. } => {
                    // Check for external ID to use as subject
                    let ext_id = node.annotations.iter().find_map(|a| {
                        if let AnnotationKind::ExternalId {
                            system,
                            identifier,
                        } = &a.kind
                        {
                            match system.as_str() {
                                "wikidata" => Some(format!(
                                    "<http://www.wikidata.org/entity/{identifier}>"
                                )),
                                "doi" => Some(format!("<https://doi.org/{identifier}>")),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    });

                    ext_id.unwrap_or_else(|| {
                        let text = node.text_content().trim().to_string();
                        let slug = slugify(&text);
                        format!("<#{slug}>")
                    })
                }
                _ => "_:b0".to_string(),
            };

            for ann in &node.annotations {
                match &ann.kind {
                    AnnotationKind::TypeAssignment { resolved_iri, raw } => {
                        let obj = if let Some(iri) = resolved_iri {
                            format!("<{}>", iri.as_str())
                        } else {
                            resolve_or_curie(raw)
                        };
                        triples.push(Triple {
                            subject: subject.clone(),
                            predicate: "a".to_string(),
                            object: obj,
                        });
                    }
                    AnnotationKind::Property {
                        resolved_iri,
                        raw_key,
                        value,
                    } => {
                        let pred = if let Some(iri) = resolved_iri {
                            format!("<{}>", iri.as_str())
                        } else {
                            resolve_or_curie(raw_key)
                        };
                        triples.push(Triple {
                            subject: subject.clone(),
                            predicate: pred,
                            object: format!("\"{}\"", escape_turtle(value)),
                        });
                    }
                    AnnotationKind::ExternalId {
                        system,
                        identifier,
                    } => {
                        let obj = match system.as_str() {
                            "wikidata" => {
                                format!("<http://www.wikidata.org/entity/{identifier}>")
                            }
                            "doi" => format!("<https://doi.org/{identifier}>"),
                            "orcid" => format!("<https://orcid.org/{identifier}>"),
                            _ => format!("\"{}\"", escape_turtle(identifier)),
                        };
                        triples.push(Triple {
                            subject: subject.clone(),
                            predicate: "<http://www.w3.org/2002/07/owl#sameAs>".to_string(),
                            object: obj,
                        });
                    }
                    _ => {}
                }
            }

            // Add rdfs:label from text content
            let text = node.text_content().trim().to_string();
            if !text.is_empty() {
                triples.push(Triple {
                    subject: subject.clone(),
                    predicate: "<http://www.w3.org/2000/01/rdf-schema#label>".to_string(),
                    object: format!("\"{}\"", escape_turtle(&text)),
                });
            }
        }

        collect_triples(&node.children, triples);
    }
}

fn resolve_or_curie(curie: &str) -> String {
    if curie.starts_with("http://") || curie.starts_with("https://") {
        format!("<{curie}>")
    } else {
        curie.to_string()
    }
}

fn escape_turtle(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
