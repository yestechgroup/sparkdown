use anyhow::{Context, Result};
use sparkdown_overlay::anchor::AnchorStatus;
use sparkdown_overlay::graph::SemanticGraph;
use sparkdown_overlay::sidecar;
use std::fs;
use std::path::Path;

/// Create an empty `.sparkdown-sem` sidecar for a `.md` file.
pub fn init(input: &str) -> Result<()> {
    let md_path = Path::new(input);
    let sidecar_path = sidecar_path_for(md_path);

    if sidecar_path.exists() {
        anyhow::bail!(
            "Sidecar already exists: {}",
            sidecar_path.display()
        );
    }

    let source = fs::read_to_string(md_path)
        .with_context(|| format!("Failed to read {input}"))?;

    // Compute a simple hash of the source
    let hash = compute_hash(&source);
    let graph = SemanticGraph::new(hash);
    let content = sidecar::serialize(&graph);

    fs::write(&sidecar_path, content)
        .with_context(|| format!("Failed to write {}", sidecar_path.display()))?;

    println!("Created {}", sidecar_path.display());
    Ok(())
}

/// Run the sync engine after markdown edits.
pub fn sync(input: &str) -> Result<()> {
    let md_path = Path::new(input);
    let sidecar_path = sidecar_path_for(md_path);

    let new_source = fs::read_to_string(md_path)
        .with_context(|| format!("Failed to read {input}"))?;
    let sidecar_content = fs::read_to_string(&sidecar_path)
        .with_context(|| format!("Failed to read {}", sidecar_path.display()))?;

    let mut graph = sidecar::parse(&sidecar_content)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Try to get old source from git
    let old_source = get_git_source(md_path);

    match old_source {
        Some(old) => {
            sparkdown_overlay::sync::sync_graph(&mut graph, &old, &new_source);
        }
        None => {
            eprintln!("Warning: git source unavailable, marking all anchors stale");
            sparkdown_overlay::sync::mark_all_stale(&mut graph);
        }
    }

    // Update source hash
    graph.source_hash = compute_hash(&new_source);

    let content = sidecar::serialize(&graph);
    fs::write(&sidecar_path, content)
        .with_context(|| format!("Failed to write {}", sidecar_path.display()))?;

    let stale_count = graph.entities_with_status(AnchorStatus::Stale).len();
    let detached_count = graph.entities_with_status(AnchorStatus::Detached).len();
    println!("Sync complete. Stale: {stale_count}, Detached: {detached_count}");

    Ok(())
}

/// Show stale/detached entities.
pub fn status(input: &str) -> Result<()> {
    let md_path = Path::new(input);
    let sidecar_path = sidecar_path_for(md_path);

    let sidecar_content = fs::read_to_string(&sidecar_path)
        .with_context(|| format!("Failed to read {}", sidecar_path.display()))?;

    let graph = sidecar::parse(&sidecar_content)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut synced = 0;
    let mut stale = 0;
    let mut detached = 0;

    for entity in &graph.entities {
        let id = entity.id.as_str();
        let types: Vec<_> = entity.types.iter().map(|t| t.as_str()).collect();
        let type_str = types.join(", ");

        match entity.status {
            AnchorStatus::Synced => {
                synced += 1;
                println!("  [synced]   _:{id} ({type_str})");
            }
            AnchorStatus::Stale => {
                stale += 1;
                println!("  [stale]    _:{id} ({type_str})");
            }
            AnchorStatus::Detached => {
                detached += 1;
                println!("  [detached] _:{id} ({type_str})");
            }
        }
    }

    println!();
    println!("Total: {} entities ({synced} synced, {stale} stale, {detached} detached)",
        graph.entities.len());

    Ok(())
}

/// Strip anchors and produce valid Turtle output.
pub fn export(input: &str) -> Result<()> {
    let md_path = Path::new(input);
    let sidecar_path = sidecar_path_for(md_path);

    let sidecar_content = fs::read_to_string(&sidecar_path)
        .with_context(|| format!("Failed to read {}", sidecar_path.display()))?;

    let graph = sidecar::parse(&sidecar_content)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    // Output valid Turtle (no anchor syntax)
    let mut out = String::new();

    // Prefixes
    for (prefix, iri) in graph.prefixes.iter() {
        if ["rdf", "rdfs", "owl", "xsd", "wikidata", "skos", "foaf"].contains(&prefix) {
            continue;
        }
        out.push_str(&format!("@prefix {prefix}: <{iri}> .\n"));
    }
    out.push('\n');

    // Entities as standard Turtle
    for entity in &graph.entities {
        let id = format!("_:{}", entity.id.as_str());
        let mut first = true;
        for ty in &entity.types {
            if first {
                out.push_str(&format!("{id} a <{}> ", ty.as_str()));
                first = false;
            } else {
                out.push_str(&format!(";\n    a <{}> ", ty.as_str()));
            }
        }
        // Property triples
        for triple in graph.triples_for_subject(&entity.id) {
            let obj_str = match &triple.object {
                sparkdown_overlay::graph::TripleObject::Entity(bn) => {
                    format!("_:{}", bn.as_str())
                }
                sparkdown_overlay::graph::TripleObject::Literal { value, .. } => {
                    format!("\"{}\"", value.replace('"', "\\\""))
                }
            };
            if first {
                out.push_str(&format!("{id} <{}> {obj_str}", triple.predicate.as_str()));
                first = false;
            } else {
                out.push_str(&format!(" ;\n    <{}> {obj_str}", triple.predicate.as_str()));
            }
        }
        if !first {
            out.push_str(" .\n\n");
        }
    }

    print!("{out}");
    Ok(())
}

/// Combined view: markdown + inline annotations for debugging.
pub fn merge(input: &str) -> Result<()> {
    let md_path = Path::new(input);
    let sidecar_path = sidecar_path_for(md_path);

    let source = fs::read_to_string(md_path)
        .with_context(|| format!("Failed to read {input}"))?;
    let sidecar_content = fs::read_to_string(&sidecar_path)
        .with_context(|| format!("Failed to read {}", sidecar_path.display()))?;

    let graph = sidecar::parse(&sidecar_content)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let index = sparkdown_overlay::mapping::MappingIndex::build(&graph);

    // Walk through the source and annotate
    let mut annotations: Vec<(usize, String)> = Vec::new();

    for entity in &graph.entities {
        if entity.anchor.is_open_ended() {
            continue;
        }
        let types: Vec<_> = entity.types.iter().map(|t| {
            let s = t.as_str();
            // Try to shorten to CURIE
            for (prefix, base) in graph.prefixes.iter() {
                if let Some(local) = s.strip_prefix(base) {
                    return format!("{prefix}:{local}");
                }
            }
            s.to_string()
        }).collect();
        annotations.push((entity.anchor.span.start, format!("{{.{}}}", types.join(" "))));
    }
    annotations.sort_by_key(|(pos, _)| *pos);

    // Simple output: print source with annotations as comments
    println!("# Merged view (markdown + semantic overlay)");
    println!("# {} entities indexed", index.len());
    println!();
    print!("{source}");
    if !annotations.is_empty() {
        println!();
        println!("# --- Semantic Overlay ---");
        for entity in &graph.entities {
            let id = entity.id.as_str();
            let types: Vec<_> = entity.types.iter().map(|t| t.as_str()).collect();
            let span = if entity.anchor.is_open_ended() {
                format!("[{}..]", entity.anchor.span.start)
            } else {
                format!("[{}..{}]", entity.anchor.span.start, entity.anchor.span.end)
            };
            println!("# _:{id} {span} : {}", types.join(", "));
        }
    }

    let _ = &annotations;

    Ok(())
}

/// Extract inline annotations from a legacy `.md` file into a sidecar.
pub fn import(input: &str) -> Result<()> {
    let md_path = Path::new(input);
    let sidecar_path = sidecar_path_for(md_path);

    if sidecar_path.exists() {
        anyhow::bail!(
            "Sidecar already exists: {}. Remove it first to re-import.",
            sidecar_path.display()
        );
    }

    let source = fs::read_to_string(md_path)
        .with_context(|| format!("Failed to read {input}"))?;

    // Parse the document to extract inline annotations
    let parser = sparkdown_core::parser::SparkdownParser::new();
    let doc = parser
        .parse(&source)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let hash = compute_hash(&source);
    let mut graph = SemanticGraph::new(hash);
    graph.prefixes = doc.prefixes.clone();

    // Walk the AST and extract entities from inline directives
    let mut entity_counter = 0;
    extract_entities_from_nodes(&doc.nodes, &mut graph, &mut entity_counter, &doc.prefixes);

    let content = sidecar::serialize(&graph);
    fs::write(&sidecar_path, &content)
        .with_context(|| format!("Failed to write {}", sidecar_path.display()))?;

    println!("Imported {} entities to {}", graph.entities.len(), sidecar_path.display());

    // Note: cleaning inline annotations from the .md is left to the user
    // to avoid accidental data loss. Use the sidecar as the new source of truth.
    println!("Tip: Review the sidecar, then manually remove inline annotations from the .md file.");

    Ok(())
}

fn extract_entities_from_nodes(
    nodes: &[sparkdown_core::ast::SemanticNode],
    graph: &mut SemanticGraph,
    counter: &mut usize,
    prefixes: &sparkdown_core::prefix::PrefixMap,
) {
    for node in nodes {
        if !node.annotations.is_empty() {
            *counter += 1;
            let id = sparkdown_overlay::graph::blank_node(&format!("e{counter}"));
            let snippet = node.text_content();
            let snippet = if snippet.len() > 40 {
                snippet[..40].to_string()
            } else {
                snippet
            };

            let mut types = Vec::new();
            for ann in &node.annotations {
                match &ann.kind {
                    sparkdown_core::annotation::AnnotationKind::TypeAssignment {
                        resolved_iri,
                        ..
                    } => {
                        if let Some(iri) = resolved_iri {
                            types.push(iri.clone());
                        }
                    }
                    sparkdown_core::annotation::AnnotationKind::Property {
                        resolved_iri,
                        value,
                        ..
                    } => {
                        if let Some(pred_iri) = resolved_iri {
                            graph.triples.push(sparkdown_overlay::graph::Triple {
                                subject: id.clone(),
                                predicate: pred_iri.clone(),
                                object: sparkdown_overlay::graph::TripleObject::Literal {
                                    value: value.clone(),
                                    datatype: None,
                                },
                            });
                        }
                    }
                    _ => {}
                }
            }

            graph.entities.push(sparkdown_overlay::graph::SemanticEntity {
                id,
                anchor: sparkdown_overlay::anchor::Anchor::new(node.span.clone(), snippet),
                types,
                status: AnchorStatus::Synced,
            });
        }

        extract_entities_from_nodes(&node.children, graph, counter, prefixes);
    }
}

// --- Helpers ---

fn sidecar_path_for(md_path: &Path) -> std::path::PathBuf {
    let name = md_path.file_name().unwrap_or_default().to_string_lossy();
    md_path.with_file_name(format!("{name}.sparkdown-sem"))
}

fn compute_hash(source: &str) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    let h = hasher.finish().to_le_bytes();
    let mut hash = [0u8; 32];
    hash[..8].copy_from_slice(&h);
    hash
}

fn get_git_source(md_path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("HEAD:{}", md_path.display())])
        .output()
        .ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout).ok()
    } else {
        None
    }
}
