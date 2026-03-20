use anyhow::{Context, Result};
use sparkdown_core::annotation::AnnotationKind;
use sparkdown_core::ast::{NodeKind, SemanticNode};
use sparkdown_core::parser::SparkdownParser;
use sparkdown_ontology::registry::ThemeRegistry;
use std::fs;

pub fn run(input: &str, level: &str) -> Result<()> {
    let source = fs::read_to_string(input).with_context(|| format!("Failed to read {input}"))?;

    let parser = SparkdownParser::new();
    let doc = parser
        .parse(&source)
        .with_context(|| format!("Failed to parse {input}"))?;

    let registry = ThemeRegistry::with_builtins();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    validate_nodes(&doc.nodes, &registry, &mut warnings, &mut errors);

    for w in &warnings {
        eprintln!("warning: {w}");
    }
    for e in &errors {
        eprintln!("error: {e}");
    }

    let issue_count = if level == "error" {
        errors.len()
    } else {
        warnings.len() + errors.len()
    };

    if issue_count == 0 {
        println!("Valid: no issues found in {input}");
    } else {
        println!(
            "{} warning(s), {} error(s) in {input}",
            warnings.len(),
            errors.len()
        );
    }

    if !errors.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

fn validate_nodes(
    nodes: &[SemanticNode],
    registry: &ThemeRegistry,
    warnings: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    for node in nodes {
        // Check type annotations resolve
        for ann in &node.annotations {
            match &ann.kind {
                AnnotationKind::TypeAssignment { resolved_iri, raw } => {
                    if resolved_iri.is_none() && raw.contains(':') {
                        warnings.push(format!(
                            "Could not resolve type CURIE: {raw}"
                        ));
                    }
                    // Check type exists in registry
                    if let Some(iri) = resolved_iri {
                        let iri_str = iri.as_str();
                        let found = registry.prefixes().iter().any(|(_, base)| {
                            if iri_str.starts_with(base) {
                                let local = &iri_str[base.len()..];
                                let prefix = registry
                                    .prefixes()
                                    .iter()
                                    .find(|(_, b)| *b == *base)
                                    .map(|(p, _)| *p)
                                    .unwrap_or("");
                                registry.lookup_type(prefix, local).is_some()
                            } else {
                                false
                            }
                        });
                        if !found {
                            warnings.push(format!(
                                "Type not found in registered ontologies: {raw}"
                            ));
                        }
                    }
                }
                AnnotationKind::Property {
                    resolved_iri,
                    raw_key,
                    ..
                } => {
                    if resolved_iri.is_none() && raw_key.contains(':') {
                        warnings.push(format!(
                            "Could not resolve property CURIE: {raw_key}"
                        ));
                    }
                }
                _ => {}
            }
        }

        // Check for sections/directives without any annotations
        match &node.kind {
            NodeKind::InlineDirective { name } if node.annotations.is_empty() => {
                warnings.push(format!(
                    "Inline directive '{name}' has no semantic annotations"
                ));
            }
            NodeKind::DirectiveBlock { name } if node.annotations.is_empty() => {
                warnings.push(format!(
                    "Block directive '{name}' has no semantic annotations"
                ));
            }
            _ => {}
        }

        validate_nodes(&node.children, registry, warnings, errors);
    }
}
