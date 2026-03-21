# Sparkdown Studio Phase 1.5: Knowledge Authoring — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add entity creation, stale anchor resolution, Knowledge Panel, and an ontology pack system to Sparkdown Studio.

**Architecture:** Extends the existing Tauri 2 actor-based backend with new SessionCommand variants for graph mutation (create, update, delete) and a shared ThemeRegistry loaded from TOML ontology packs. The Svelte 5 frontend gains three new components (EntityCreationPopup, stale anchor widget, KnowledgePanel) connected via Svelte 5 runes and Tauri event listeners.

**Tech Stack:** Rust (Tauri 2, tokio, serde, oxrdf), Svelte 5 (runes), CodeMirror 6, TOML (ontology packs)

**Spec:** `docs/superpowers/specs/2026-03-21-sparkdown-studio-phase2-design.md`

---

## File Structure

### New files to create

| File | Responsibility |
|------|---------------|
| `crates/sparkdown-ontology/src/pack.rs` | Pack TOML parsing, OntologyPack struct, TomlProvider impl |
| `studio/src-tauri/src/pack_types.rs` | TypeCategoryDto, TypeOptionDto DTOs for IPC |
| `studio/src/lib/components/EntityCreationPopup.svelte` | Cmd+E type picker popup |
| `studio/src/lib/components/KnowledgePanel.svelte` | Right panel with overview/detail views |
| `studio/src/lib/components/EntityList.svelte` | Reusable entity list grouped by type |
| `studio/src/lib/components/EntityDetail.svelte` | Entity detail view within Knowledge Panel |
| `studio/src/lib/editor/stale-anchor-widget.ts` | CM extension for inline stale resolution |

### Files to modify

| File | Changes |
|------|---------|
| `crates/sparkdown-ontology/src/lib.rs` | Add `pub mod pack;` |
| `crates/sparkdown-ontology/src/registry.rs:64-168` | Add `load_packs()`, `all_type_categories()`, `search_types()` methods |
| `studio/src-tauri/Cargo.toml:10-22` | Add `toml` dependency |
| `studio/src-tauri/src/lib.rs:9-28` | Register new commands, manage ThemeRegistryState |
| `studio/src-tauri/src/registry.rs:8-26` | Add new SessionCommand variants |
| `studio/src-tauri/src/session.rs:20-29` | Add `next_entity_id` counter, shared registry reference; add command handlers |
| `studio/src-tauri/src/commands.rs:11-136` | Add 7 new Tauri command functions |
| `studio/src-tauri/src/types.rs:75-77` | Add `char_to_byte_offset()`, DocumentOverviewDto, EntityDetailDto |
| `studio/src/lib/tauri/commands.ts:46-76` | Add new command wrappers and type interfaces |
| `studio/src/lib/stores/document.svelte.ts:3-19` | Add selectedEntityId, entityDetail, dismissedStaleIds state |
| `studio/src/lib/stores/events.ts:33-60` | No changes needed (existing events carry updated data) |
| `studio/src/lib/components/CodeMirrorEditor.svelte:35-88` | Add Cmd+E keybinding, stale anchor widget extension |
| `studio/src/lib/components/EditorPane.svelte:1-42` | Mount EntityCreationPopup |
| `studio/src/routes/+page.svelte:31-42` | Add KnowledgePanel to layout |

---

## Task 1: Ontology Pack Loader (sparkdown-ontology)

**Files:**
- Create: `crates/sparkdown-ontology/src/pack.rs`
- Modify: `crates/sparkdown-ontology/src/lib.rs:1-2`
- Modify: `crates/sparkdown-ontology/src/registry.rs:64-168`
- Modify: `crates/sparkdown-ontology/Cargo.toml` (add `toml` and `serde` deps)

- [ ] **Step 1: Add toml and serde dependencies to sparkdown-ontology**

In `crates/sparkdown-ontology/Cargo.toml`, add:
```toml
toml = "0.8"
serde = { version = "1", features = ["derive"] }
```

- [ ] **Step 2: Write the failing test for pack TOML parsing**

Create `crates/sparkdown-ontology/src/pack.rs` with the test:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pack_metadata() {
        let toml_str = r#"
[pack]
name = "test-ontology"
version = "1.0.0"
description = "A test ontology pack"

[prefixes]
test = "http://example.org/test/"

[categories]
people = { label = "People", types = ["test:Person", "test:Agent"] }
"#;
        let pack = parse_pack_toml(toml_str).unwrap();
        assert_eq!(pack.metadata.name, "test-ontology");
        assert_eq!(pack.metadata.version, "1.0.0");
        assert_eq!(pack.prefixes.get("test").unwrap(), "http://example.org/test/");
        assert_eq!(pack.categories.len(), 1);
        assert_eq!(pack.categories[0].label, "People");
        assert_eq!(pack.categories[0].types, vec!["test:Person", "test:Agent"]);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p sparkdown-ontology parse_pack_metadata -- --nocapture`
Expected: FAIL — `parse_pack_toml` not defined

- [ ] **Step 4: Implement pack TOML parsing structs and parser**

In `crates/sparkdown-ontology/src/pack.rs`, add above the test module:

```rust
use serde::Deserialize;
use std::collections::HashMap;

/// Raw TOML structure for pack.toml
#[derive(Debug, Deserialize)]
struct PackToml {
    pack: PackSection,
    prefixes: Option<HashMap<String, String>>,
    categories: Option<HashMap<String, CategorySection>>,
}

#[derive(Debug, Deserialize)]
struct PackSection {
    name: String,
    version: String,
    description: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CategorySection {
    label: String,
    types: Vec<String>,
}

/// Parsed ontology pack metadata.
#[derive(Debug, Clone)]
pub struct OntologyPackMeta {
    pub metadata: PackMetadata,
    pub prefixes: HashMap<String, String>,
    pub categories: Vec<TypeCategory>,
}

#[derive(Debug, Clone)]
pub struct PackMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TypeCategory {
    pub key: String,
    pub label: String,
    pub types: Vec<String>,
}

/// Parse a pack.toml string into an OntologyPackMeta.
pub fn parse_pack_toml(input: &str) -> Result<OntologyPackMeta, String> {
    let raw: PackToml = toml::from_str(input).map_err(|e| format!("TOML parse error: {e}"))?;

    let categories = match raw.categories {
        Some(cats) => cats
            .into_iter()
            .map(|(key, section)| TypeCategory {
                key,
                label: section.label,
                types: section.types,
            })
            .collect(),
        None => vec![],
    };

    Ok(OntologyPackMeta {
        metadata: PackMetadata {
            name: raw.pack.name,
            version: raw.pack.version,
            description: raw.pack.description,
            source: raw.pack.source,
        },
        prefixes: raw.prefixes.unwrap_or_default(),
        categories,
    })
}
```

- [ ] **Step 5: Export the pack module from lib.rs**

In `crates/sparkdown-ontology/src/lib.rs`, add:
```rust
pub mod pack;
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test -p sparkdown-ontology parse_pack_metadata -- --nocapture`
Expected: PASS

- [ ] **Step 7: Write the failing test for types.toml parsing**

Add to `pack.rs` tests:

```rust
#[test]
fn parse_types_toml() {
    let toml_str = r#"
[[types]]
iri = "http://example.org/test/Person"
curie = "test:Person"
label = "Person"
description = "A human being"
suggested_properties = ["test:name", "test:age"]

[[types]]
iri = "http://example.org/test/Agent"
curie = "test:Agent"
label = "Agent"
"#;
    let types = parse_types_toml(toml_str).unwrap();
    assert_eq!(types.len(), 2);
    assert_eq!(types[0].label, "Person");
    assert_eq!(types[0].iri, "http://example.org/test/Person");
    assert_eq!(types[0].suggested_properties, vec!["test:name", "test:age"]);
    assert_eq!(types[1].label, "Agent");
    assert!(types[1].suggested_properties.is_empty());
}
```

- [ ] **Step 8: Implement types.toml parsing**

Add to `pack.rs`:

```rust
#[derive(Debug, Deserialize)]
struct TypesToml {
    types: Vec<TypeEntry>,
}

#[derive(Debug, Deserialize)]
struct TypeEntry {
    iri: String,
    curie: String,
    label: String,
    description: Option<String>,
    parent: Option<String>,
    #[serde(default)]
    suggested_properties: Vec<String>,
}

/// A type definition from a pack's types.toml.
#[derive(Debug, Clone)]
pub struct PackTypeDef {
    pub iri: String,
    pub curie: String,
    pub label: String,
    pub description: Option<String>,
    pub parent: Option<String>,
    pub suggested_properties: Vec<String>,
}

pub fn parse_types_toml(input: &str) -> Result<Vec<PackTypeDef>, String> {
    let raw: TypesToml = toml::from_str(input).map_err(|e| format!("TOML parse error: {e}"))?;
    Ok(raw.types.into_iter().map(|t| PackTypeDef {
        iri: t.iri,
        curie: t.curie,
        label: t.label,
        description: t.description,
        parent: t.parent,
        suggested_properties: t.suggested_properties,
    }).collect())
}
```

- [ ] **Step 9: Run test to verify it passes**

Run: `cargo test -p sparkdown-ontology parse_types_toml -- --nocapture`
Expected: PASS

- [ ] **Step 10: Write the failing test for properties.toml parsing**

Add to `pack.rs` tests:

```rust
#[test]
fn parse_properties_toml() {
    let toml_str = r#"
[[properties]]
iri = "http://example.org/test/name"
curie = "test:name"
label = "Name"
expected_type = "Text"

[[properties]]
iri = "http://example.org/test/knows"
curie = "test:knows"
label = "Knows"
expected_type = "Entity"
"#;
    let props = parse_properties_toml(toml_str).unwrap();
    assert_eq!(props.len(), 2);
    assert_eq!(props[0].label, "Name");
    assert_eq!(props[0].expected_type, "Text");
    assert_eq!(props[1].expected_type, "Entity");
}
```

- [ ] **Step 11: Implement properties.toml parsing**

Add to `pack.rs`:

```rust
#[derive(Debug, Deserialize)]
struct PropertiesToml {
    properties: Vec<PropertyEntry>,
}

#[derive(Debug, Deserialize)]
struct PropertyEntry {
    iri: String,
    curie: String,
    label: String,
    expected_type: String,
    description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PackPropertyDef {
    pub iri: String,
    pub curie: String,
    pub label: String,
    pub expected_type: String,
    pub description: Option<String>,
}

pub fn parse_properties_toml(input: &str) -> Result<Vec<PackPropertyDef>, String> {
    let raw: PropertiesToml = toml::from_str(input).map_err(|e| format!("TOML parse error: {e}"))?;
    Ok(raw.properties.into_iter().map(|p| PackPropertyDef {
        iri: p.iri,
        curie: p.curie,
        label: p.label,
        expected_type: p.expected_type,
        description: p.description,
    }).collect())
}
```

- [ ] **Step 12: Run test to verify it passes**

Run: `cargo test -p sparkdown-ontology parse_properties_toml -- --nocapture`
Expected: PASS

- [ ] **Step 13: Write the failing test for TomlOntologyProvider**

Add to `pack.rs` tests:

```rust
use crate::registry::OntologyProvider;

#[test]
fn toml_provider_lookups() {
    let types = vec![PackTypeDef {
        iri: "http://example.org/test/Person".to_string(),
        curie: "test:Person".to_string(),
        label: "Person".to_string(),
        description: Some("A human".to_string()),
        parent: None,
        suggested_properties: vec![],
    }];
    let props = vec![PackPropertyDef {
        iri: "http://example.org/test/name".to_string(),
        curie: "test:name".to_string(),
        label: "Name".to_string(),
        expected_type: "Text".to_string(),
        description: None,
    }];
    let provider = TomlOntologyProvider::new("test", "http://example.org/test/", types, props);

    assert_eq!(provider.prefix(), "test");
    assert_eq!(provider.base_iri(), "http://example.org/test/");
    assert!(provider.lookup_type("Person").is_some());
    assert!(provider.lookup_type("Unknown").is_none());
    assert!(provider.lookup_property("name").is_some());
    assert_eq!(provider.all_types().len(), 1);
    assert_eq!(provider.all_properties().len(), 1);
}
```

- [ ] **Step 14: Implement TomlOntologyProvider**

Add to `pack.rs`:

```rust
use oxrdf::NamedNode;
use std::collections::HashMap;
use crate::registry::{OntologyProvider, TypeDef, PropertyDef, ExpectedType};

/// An OntologyProvider backed by TOML pack definitions.
pub struct TomlOntologyProvider {
    prefix_str: String,
    base: String,
    types: HashMap<String, TypeDef>,
    properties: HashMap<String, PropertyDef>,
}

impl TomlOntologyProvider {
    pub fn new(
        prefix: &str,
        base_iri: &str,
        type_defs: Vec<PackTypeDef>,
        prop_defs: Vec<PackPropertyDef>,
    ) -> Self {
        let mut types = HashMap::new();
        for t in type_defs {
            let local = t.iri.strip_prefix(base_iri).unwrap_or(&t.label).to_string();
            types.insert(local, TypeDef {
                iri: NamedNode::new_unchecked(&t.iri),
                label: t.label,
                parent_types: t.parent.iter().map(|p| NamedNode::new_unchecked(p)).collect(),
                properties: t.suggested_properties.iter().map(|p| NamedNode::new_unchecked(p)).collect(),
                comment: t.description,
            });
        }
        let mut properties = HashMap::new();
        for p in prop_defs {
            let local = p.iri.strip_prefix(base_iri).unwrap_or(&p.label).to_string();
            let expected = match p.expected_type.as_str() {
                "Text" => ExpectedType::Text,
                "Date" => ExpectedType::Date,
                "DateTime" => ExpectedType::DateTime,
                "Integer" => ExpectedType::Integer,
                "Float" => ExpectedType::Float,
                "Boolean" => ExpectedType::Boolean,
                "Url" => ExpectedType::Url,
                "Entity" => ExpectedType::Entity(NamedNode::new_unchecked("http://www.w3.org/2002/07/owl#Thing")),
                _ => ExpectedType::Text, // default fallback
            };
            properties.insert(local, PropertyDef {
                iri: NamedNode::new_unchecked(&p.iri),
                label: p.label,
                expected_type: expected,
                comment: p.description,
            });
        }
        Self { prefix_str: prefix.to_string(), base: base_iri.to_string(), types, properties }
    }
}

impl OntologyProvider for TomlOntologyProvider {
    fn prefix(&self) -> &str { &self.prefix_str }
    fn base_iri(&self) -> &str { &self.base }
    fn lookup_type(&self, local_name: &str) -> Option<&TypeDef> { self.types.get(local_name) }
    fn lookup_property(&self, local_name: &str) -> Option<&PropertyDef> { self.properties.get(local_name) }
    fn all_types(&self) -> Vec<&TypeDef> { self.types.values().collect() }
    fn all_properties(&self) -> Vec<&PropertyDef> { self.properties.values().collect() }
}
```

- [ ] **Step 15: Run test to verify it passes**

Run: `cargo test -p sparkdown-ontology toml_provider_lookups -- --nocapture`
Expected: PASS

- [ ] **Step 16: Add ThemeRegistry methods for pack support**

In `crates/sparkdown-ontology/src/registry.rs`, add after `prefixes()` method (line ~168):

```rust
/// Returns all type categories across all registered providers,
/// structured for UI consumption. Returns owned Strings (no leaks).
pub fn all_type_categories(&self) -> Vec<(String, String, Vec<(String, String, &TypeDef)>)> {
    // Returns (prefix, base_iri, Vec<(curie, local_name, &TypeDef)>) per provider
    self.providers.iter().map(|(prefix, provider)| {
        let types: Vec<_> = provider.all_types().into_iter().map(|t| {
            let local = t.iri.as_str().strip_prefix(provider.base_iri()).unwrap_or(t.iri.as_str()).to_string();
            let curie = format!("{}:{}", prefix, local);
            (curie, local, t)
        }).collect();
        (prefix.clone(), provider.base_iri().to_string(), types)
    }).collect()
}

/// Search types by query string across all providers. Returns up to `limit` results.
pub fn search_types(&self, query: &str, limit: usize) -> Vec<(&str, &TypeDef)> {
    let query_lower = query.to_lowercase();
    let mut results = vec![];
    for (prefix, provider) in &self.providers {
        for t in provider.all_types() {
            if results.len() >= limit { return results; }
            if t.label.to_lowercase().contains(&query_lower)
                || t.iri.as_str().to_lowercase().contains(&query_lower) {
                results.push((prefix.as_str(), t));
            }
        }
    }
    results
}
```

- [ ] **Step 17: Run all ontology tests**

Run: `cargo test -p sparkdown-ontology -- --nocapture`
Expected: All tests PASS

- [ ] **Step 18: Commit**

```bash
git add crates/sparkdown-ontology/
git commit -m "feat(ontology): add TOML ontology pack loader and ThemeRegistry search"
```

---

## Task 2: Backend — char_to_byte_offset and new DTOs (studio types)

**Files:**
- Modify: `studio/src-tauri/src/types.rs:75-129`
- Create: `studio/src-tauri/src/pack_types.rs`

- [ ] **Step 1: Write the failing test for char_to_byte_offset**

In `studio/src-tauri/src/types.rs`, add to the existing tests module:

```rust
#[test]
fn char_to_byte_offset_ascii() {
    let source = "Hello, World!";
    assert_eq!(char_to_byte_offset(source, 0), 0);
    assert_eq!(char_to_byte_offset(source, 5), 5);
    assert_eq!(char_to_byte_offset(source, 13), 13);
}

#[test]
fn char_to_byte_offset_multibyte() {
    let source = "café"; // é is 2 bytes UTF-8, 1 code unit UTF-16
    assert_eq!(char_to_byte_offset(source, 0), 0);
    assert_eq!(char_to_byte_offset(source, 3), 3);  // 'f' is at byte 3
    assert_eq!(char_to_byte_offset(source, 4), 5);  // é is 2 bytes
}

#[test]
fn char_to_byte_offset_emoji() {
    let source = "hi 👋 there"; // 👋 is 4 bytes UTF-8, 2 code units UTF-16
    assert_eq!(char_to_byte_offset(source, 3), 3);  // start of emoji
    assert_eq!(char_to_byte_offset(source, 5), 7);  // after emoji (2 UTF-16 units)
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p sparkdown-studio char_to_byte_offset -- --nocapture`
Expected: FAIL — function not defined

- [ ] **Step 3: Implement char_to_byte_offset**

In `studio/src-tauri/src/types.rs`, add near the existing `byte_to_char_offset` (line ~75):

```rust
/// Maps UTF-16 character offset (CodeMirror) to byte offset in the source string.
pub fn char_to_byte_offset(source: &str, char_offset: usize) -> usize {
    let mut byte_pos = 0;
    let mut char_count = 0;
    for ch in source.chars() {
        if char_count >= char_offset {
            break;
        }
        char_count += ch.len_utf16();
        byte_pos += ch.len_utf8();
    }
    byte_pos
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p sparkdown-studio char_to_byte_offset -- --nocapture`
Expected: PASS

- [ ] **Step 5: Add DocumentOverviewDto and EntityDetailDto**

In `studio/src-tauri/src/types.rs`, add after the existing structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentOverviewDto {
    pub title: Option<String>,
    pub entities: Vec<EntityDto>,
    pub sidecar_status: SidecarStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDetailDto {
    pub entity: EntityDto,
    pub all_relations: Vec<Relation>,
    pub incoming_relations: Vec<Relation>,
    pub anchor_snippet: String,
    pub anchor_line: usize,
}
```

- [ ] **Step 6: Create pack_types.rs with TypeCategoryDto and TypeOptionDto**

Create `studio/src-tauri/src/pack_types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCategoryDto {
    pub pack_name: String,
    pub category_label: String,
    pub types: Vec<TypeOptionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeOptionDto {
    pub iri: String,
    pub curie: String,
    pub label: String,
    pub description: Option<String>,
}
```

- [ ] **Step 7: Add `mod pack_types;` to lib.rs**

In `studio/src-tauri/src/lib.rs`, add: `mod pack_types;`

- [ ] **Step 8: Run all studio tests**

Run: `cargo test -p sparkdown-studio -- --nocapture`
Expected: All tests PASS

- [ ] **Step 9: Commit**

```bash
git add studio/src-tauri/src/types.rs studio/src-tauri/src/pack_types.rs studio/src-tauri/src/lib.rs
git commit -m "feat(studio): add char_to_byte_offset, DocumentOverviewDto, EntityDetailDto, pack type DTOs"
```

---

## Task 3: Backend — New SessionCommand Variants

**Files:**
- Modify: `studio/src-tauri/src/registry.rs:8-26`

- [ ] **Step 1: Add new SessionCommand variants**

In `studio/src-tauri/src/registry.rs`, expand the `SessionCommand` enum (currently lines 8-26) to add after the `Close` variant:

```rust
// Phase 1.5: Entity creation and management
CreateEntity {
    span_start: usize,
    span_end: usize,
    type_iri: String,
    reply: tokio::sync::oneshot::Sender<Result<crate::types::EntityDto, String>>,
},
UpdateStaleAnchor {
    entity_id: String,
    reply: tokio::sync::oneshot::Sender<Result<(), String>>,
},
GetDocumentOverview {
    reply: tokio::sync::oneshot::Sender<crate::types::DocumentOverviewDto>,
},
GetEntityDetail {
    entity_id: String,
    reply: tokio::sync::oneshot::Sender<Result<crate::types::EntityDetailDto, String>>,
},
DeleteEntity {
    entity_id: String,
    reply: tokio::sync::oneshot::Sender<Result<(), String>>,
},
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p sparkdown-studio`
Expected: Warnings about unused variants are OK. No errors. (The session run loop won't handle them yet — that's Task 4.)

- [ ] **Step 3: Commit**

```bash
git add studio/src-tauri/src/registry.rs
git commit -m "feat(studio): add SessionCommand variants for entity creation and management"
```

---

## Task 4: Backend — CreateEntity Handler

**Files:**
- Modify: `studio/src-tauri/src/session.rs:20-29` (struct fields), `127-149` (run loop), add new handler method

- [ ] **Step 1: Add `next_entity_id` counter to DocumentSession struct**

In `studio/src-tauri/src/session.rs`, add a field to the `DocumentSession` struct (line ~29):

```rust
next_entity_id: usize,
```

Initialize it in `DocumentSession::open()` to `graph.entities.len() + 1` (after graph is loaded, around line ~100).

- [ ] **Step 2: Write the failing test for CreateEntity**

Add to the test module in `session.rs`:

```rust
#[tokio::test]
async fn create_entity_adds_to_graph() {
    use sparkdown_overlay::graph::SemanticGraph;
    use sparkdown_overlay::anchor::AnchorStatus;

    // Create a minimal session state
    let source = "Dr. Sarah Chen is a researcher.";
    let mut graph = SemanticGraph::new([0u8; 32]);
    assert_eq!(graph.entities.len(), 0);

    // Simulate CreateEntity: span covers "Dr. Sarah Chen" (bytes 0..14)
    let span_start = 0;
    let span_end = 14;
    let snippet = source[span_start..span_end].to_string();
    let entity_id = "e1";
    let type_iri = "http://schema.org/Person";

    let entity = sparkdown_overlay::graph::SemanticEntity {
        id: sparkdown_overlay::graph::blank_node(entity_id),
        anchor: sparkdown_overlay::anchor::Anchor::new(span_start..span_end, &snippet[..std::cmp::min(snippet.len(), 40)]),
        types: vec![oxrdf::NamedNode::new_unchecked(type_iri)],
        status: AnchorStatus::Synced,
    };
    graph.entities.push(entity);

    assert_eq!(graph.entities.len(), 1);
    assert_eq!(graph.entities[0].id.as_str(), entity_id);
    assert_eq!(graph.entities[0].types[0].as_str(), type_iri);
    assert_eq!(graph.entities[0].anchor.span, 0..14);
}
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test -p sparkdown-studio create_entity_adds -- --nocapture`
Expected: PASS (this tests the graph mutation logic directly)

- [ ] **Step 4: Implement handle_create_entity on DocumentSession**

First, add the required imports at the top of `session.rs`:
```rust
use sparkdown_overlay::graph::{SemanticEntity, blank_node, TripleObject};
use sparkdown_overlay::anchor::Anchor;
use crate::types::char_to_byte_offset;
```

Add a new method to `DocumentSession` impl:

```rust
fn handle_create_entity(
    &mut self,
    char_start: usize,
    char_end: usize,
    type_iri: String,
) -> Result<EntityDto, String> {
    // Validate char offsets
    if char_start >= char_end {
        return Err("Selection must not be empty".into());
    }

    // Convert UTF-16 character offsets (from CodeMirror) to byte offsets
    let span_start = char_to_byte_offset(&self.source, char_start);
    let span_end = char_to_byte_offset(&self.source, char_end);

    if span_end > self.source.len() {
        return Err("Selection range exceeds document length".into());
    }

    // Extract snippet (truncated to 40 chars for anchor storage)
    let raw_snippet = &self.source[span_start..span_end];
    let snippet: String = raw_snippet.chars().take(40).collect();

    // Generate unique blank node ID
    let entity_id = format!("e{}", self.next_entity_id);
    self.next_entity_id += 1;

    // Create the entity
    let iri = oxrdf::NamedNode::new(&type_iri)
        .map_err(|e| format!("Invalid type IRI: {e}"))?;

    let entity = SemanticEntity {
        id: blank_node(&entity_id),
        anchor: Anchor::new(span_start..span_end, snippet),
        types: vec![iri],
        status: AnchorStatus::Synced,
    };

    self.graph.entities.push(entity);
    self.index = MappingIndex::build(&self.graph);

    // Build DTO for the newly created entity
    let dto = self.entity_to_dto(self.graph.entities.last().unwrap());
    Ok(dto)
}
```

- [ ] **Step 5: Wire CreateEntity in the run loop**

In `DocumentSession::run()` (line ~127-149), add a match arm for `CreateEntity`:

```rust
SessionCommand::CreateEntity { span_start, span_end, type_iri, reply } => {
    let result = self.handle_create_entity(span_start, span_end, type_iri);
    if result.is_ok() {
        let dtos = self.build_entity_dtos();
        let status = self.build_sidecar_status();
        events::emit_entities_updated(&self.app, events::EntitiesUpdatedPayload {
            doc_id: self.doc_id.clone(),
            entities: dtos,
        });
        events::emit_sidecar_status(&self.app, events::SidecarStatusPayload {
            doc_id: self.doc_id.clone(),
            status,
        });
    }
    let _ = reply.send(result);
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p sparkdown-studio`
Expected: No errors (warnings about unhandled new variants are expected until Task 6)

- [ ] **Step 7: Commit**

```bash
git add studio/src-tauri/src/session.rs
git commit -m "feat(studio): implement CreateEntity session command handler"
```

---

## Task 5: Backend — UpdateStaleAnchor, GetDocumentOverview, GetEntityDetail, DeleteEntity Handlers

**Files:**
- Modify: `studio/src-tauri/src/session.rs` (add handler methods and wire into run loop)

- [ ] **Step 1: Implement handle_update_stale_anchor**

```rust
fn handle_update_stale_anchor(&mut self, entity_id: &str) -> Result<(), String> {
    let entity = self.graph.entities.iter_mut()
        .find(|e| e.id.as_str() == entity_id)
        .ok_or_else(|| format!("Entity not found: {entity_id}"))?;

    // Update snippet from current source
    let start = entity.anchor.span.start;
    let end = entity.anchor.span.end.min(self.source.len());
    let new_snippet: String = self.source[start..end].chars().take(40).collect();
    entity.anchor.snippet = new_snippet;
    entity.status = AnchorStatus::Synced;

    self.index = MappingIndex::build(&self.graph);
    Ok(())
}
```

- [ ] **Step 2: Implement handle_get_document_overview**

```rust
fn handle_get_document_overview(&self) -> DocumentOverviewDto {
    let title = None; // TODO: extract from frontmatter when parser exposes it
    let entities = self.build_entity_dtos();
    let sidecar_status = self.build_sidecar_status();
    DocumentOverviewDto { title, entities, sidecar_status }
}
```

- [ ] **Step 3: Implement handle_get_entity_detail**

```rust
fn handle_get_entity_detail(&self, entity_id: &str) -> Result<EntityDetailDto, String> {
    let entity = self.graph.entities.iter()
        .find(|e| e.id.as_str() == entity_id)
        .ok_or_else(|| format!("Entity not found: {entity_id}"))?;

    let dto = self.entity_to_dto(entity);

    // All outgoing relations (no cap)
    let all_relations: Vec<Relation> = self.graph.triples_for_subject(&entity.id)
        .iter()
        .map(|t| {
            let pred_label = iri_local_name(t.predicate.as_str());
            let (target_label, target_id) = match &t.object {
                TripleObject::Entity(id) => {
                    let label = self.graph.entity_by_id(id)
                        .map(|e| e.anchor.snippet.clone())
                        .unwrap_or_else(|| id.as_str().to_string());
                    (label, id.as_str().to_string())
                }
                TripleObject::Literal { value, .. } => (value.clone(), String::new()),
            };
            Relation { predicate_label: pred_label, target_label, target_id }
        })
        .collect();

    // Incoming relations
    let incoming_relations: Vec<Relation> = self.graph.triples.iter()
        .filter(|t| match &t.object {
            TripleObject::Entity(id) => id.as_str() == entity_id,
            _ => false,
        })
        .map(|t| {
            let pred_label = iri_local_name(t.predicate.as_str());
            let source_label = self.graph.entity_by_id(&t.subject)
                .map(|e| e.anchor.snippet.clone())
                .unwrap_or_else(|| t.subject.as_str().to_string());
            Relation {
                predicate_label: pred_label,
                target_label: source_label,
                target_id: t.subject.as_str().to_string(),
            }
        })
        .collect();

    // Compute line number from byte offset
    let anchor_line = self.source[..entity.anchor.span.start]
        .chars().filter(|c| *c == '\n').count() + 1;

    Ok(EntityDetailDto {
        entity: dto,
        all_relations,
        incoming_relations,
        anchor_snippet: entity.anchor.snippet.clone(),
        anchor_line,
    })
}
```

- [ ] **Step 4: Implement handle_delete_entity**

```rust
fn handle_delete_entity(&mut self, entity_id: &str) -> Result<(), String> {
    let existed = self.graph.entities.len();
    self.graph.entities.retain(|e| e.id.as_str() != entity_id);
    if self.graph.entities.len() == existed {
        return Err(format!("Entity not found: {entity_id}"));
    }

    // Remove all triples referencing this entity (as subject or object)
    self.graph.triples.retain(|t| {
        let subject_match = t.subject.as_str() == entity_id;
        let object_match = match &t.object {
            TripleObject::Entity(id) => id.as_str() == entity_id,
            _ => false,
        };
        !subject_match && !object_match
    });

    self.index = MappingIndex::build(&self.graph);
    Ok(())
}
```

- [ ] **Step 5: Wire all new commands into the run loop**

Add match arms in `DocumentSession::run()`:

```rust
SessionCommand::UpdateStaleAnchor { entity_id, reply } => {
    let result = self.handle_update_stale_anchor(&entity_id);
    if result.is_ok() {
        let dtos = self.build_entity_dtos();
        let status = self.build_sidecar_status();
        events::emit_entities_updated(&self.app, events::EntitiesUpdatedPayload {
            doc_id: self.doc_id.clone(),
            entities: dtos,
        });
        events::emit_sidecar_status(&self.app, events::SidecarStatusPayload {
            doc_id: self.doc_id.clone(),
            status,
        });
        // Re-emit stale anchors with this entity removed
        let stale: Vec<_> = self.graph.entities.iter()
            .filter(|e| e.status == AnchorStatus::Stale)
            .map(|e| self.build_stale_anchor(e))
            .collect();
        events::emit_stale_anchors(&self.app, events::StaleAnchorsPayload {
            doc_id: self.doc_id.clone(),
            anchors: stale,
        });
    }
    let _ = reply.send(result);
}
SessionCommand::GetDocumentOverview { reply } => {
    let overview = self.handle_get_document_overview();
    let _ = reply.send(overview);
}
SessionCommand::GetEntityDetail { entity_id, reply } => {
    let result = self.handle_get_entity_detail(&entity_id);
    let _ = reply.send(result);
}
SessionCommand::DeleteEntity { entity_id, reply } => {
    let result = self.handle_delete_entity(&entity_id);
    if result.is_ok() {
        let dtos = self.build_entity_dtos();
        let status = self.build_sidecar_status();
        events::emit_entities_updated(&self.app, events::EntitiesUpdatedPayload {
            doc_id: self.doc_id.clone(),
            entities: dtos,
        });
        events::emit_sidecar_status(&self.app, events::SidecarStatusPayload {
            doc_id: self.doc_id.clone(),
            status,
        });
    }
    let _ = reply.send(result);
}
```

- [ ] **Step 6: Run all studio tests**

Run: `cargo test -p sparkdown-studio -- --nocapture`
Expected: All tests PASS

- [ ] **Step 7: Commit**

```bash
git add studio/src-tauri/src/session.rs
git commit -m "feat(studio): implement UpdateStaleAnchor, GetDocumentOverview, GetEntityDetail, DeleteEntity handlers"
```

---

## Task 6: Backend — New Tauri Commands and ThemeRegistry State

**Files:**
- Modify: `studio/src-tauri/src/commands.rs:11-176`
- Modify: `studio/src-tauri/src/lib.rs:9-28`
- Modify: `studio/src-tauri/Cargo.toml`

- [ ] **Step 1: Add ThemeRegistryState to lib.rs**

In `studio/src-tauri/src/lib.rs`, add:
```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use sparkdown_ontology::registry::ThemeRegistry;

type ThemeRegistryState = Arc<RwLock<ThemeRegistry>>;
```

In the `run()` function, before the Tauri builder, initialize the registry:
```rust
let registry = Arc::new(RwLock::new(ThemeRegistry::with_builtins()));
```

Add it as Tauri managed state:
```rust
.manage(registry)
```

Register the new commands in the `invoke_handler`:
```rust
.invoke_handler(tauri::generate_handler![
    // existing commands...
    commands::create_entity,
    commands::update_stale_anchor,
    commands::get_document_overview,
    commands::get_entity_detail,
    commands::delete_entity,
    commands::list_available_types,
    commands::search_types,
])
```

- [ ] **Step 3: Implement new Tauri commands**

In `studio/src-tauri/src/commands.rs`, add:

Note: Follow the existing command pattern from `commands.rs` — the `SessionRegistry` handles its own internal locking via `async fn get()`, so use `registry.get(&doc_id).await` directly (not `registry.read().await`). The state type is `Arc<SessionRegistry>`.

The session handles char→byte conversion internally by adding a `GetSource` step, or the `CreateEntity` command is extended to accept char offsets and the session converts using its own source. For Phase 1.5, the session's `handle_create_entity` receives char offsets and calls `char_to_byte_offset(&self.source, char_start)` internally before creating the anchor.

```rust
use crate::types::{char_to_byte_offset, DocumentOverviewDto, EntityDetailDto, EntityDto};
use crate::pack_types::{TypeCategoryDto, TypeOptionDto};
use crate::registry::SessionCommand;

#[tauri::command]
pub async fn create_entity(
    doc_id: String,
    char_start: usize,
    char_end: usize,
    type_iri: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<EntityDto, String> {
    if char_start >= char_end {
        return Err("Selection must not be empty".into());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    sender.send(SessionCommand::CreateEntity {
        span_start: char_start,  // Session converts to byte offsets using its source
        span_end: char_end,
        type_iri,
        reply: tx,
    }).await.map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn update_stale_anchor(
    doc_id: String,
    entity_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    sender.send(SessionCommand::UpdateStaleAnchor { entity_id, reply: tx })
        .await.map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn get_document_overview(
    doc_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<DocumentOverviewDto, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    sender.send(SessionCommand::GetDocumentOverview { reply: tx })
        .await.map_err(|_| "Session closed".to_string())?;
    Ok(rx.await.map_err(|_| "Session dropped".to_string())?)
}

#[tauri::command]
pub async fn get_entity_detail(
    doc_id: String,
    entity_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<EntityDetailDto, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    sender.send(SessionCommand::GetEntityDetail { entity_id, reply: tx })
        .await.map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn delete_entity(
    doc_id: String,
    entity_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    sender.send(SessionCommand::DeleteEntity { entity_id, reply: tx })
        .await.map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn list_available_types(
    theme_registry: tauri::State<'_, crate::ThemeRegistryState>,
) -> Result<Vec<TypeCategoryDto>, String> {
    let reg = theme_registry.read().await;
    let mut categories = vec![];
    for (prefix, base_iri, types) in reg.all_type_categories() {
        let type_options: Vec<TypeOptionDto> = types.into_iter().map(|(curie, _local, tdef)| {
            TypeOptionDto {
                iri: tdef.iri.as_str().to_string(),
                curie: curie.to_string(),
                label: tdef.label.clone(),
                description: tdef.comment.clone(),
            }
        }).collect();
        categories.push(TypeCategoryDto {
            pack_name: prefix,
            category_label: base_iri,
            types: type_options,
        });
    }
    Ok(categories)
}

#[tauri::command]
pub async fn search_types(
    query: String,
    limit: Option<usize>,
    theme_registry: tauri::State<'_, crate::ThemeRegistryState>,
) -> Result<Vec<TypeOptionDto>, String> {
    let reg = theme_registry.read().await;
    let results = reg.search_types(&query, limit.unwrap_or(50));
    Ok(results.into_iter().map(|(prefix, tdef)| {
        let local = tdef.iri.as_str().rsplit('/').next().unwrap_or(tdef.iri.as_str());
        TypeOptionDto {
            iri: tdef.iri.as_str().to_string(),
            curie: format!("{prefix}:{local}"),
            label: tdef.label.clone(),
            description: tdef.comment.clone(),
        }
    }).collect())
}
```

- [ ] **Step 4: Verify full build**

Run: `cargo build -p sparkdown-studio`
Expected: BUILD SUCCESS

- [ ] **Step 5: Run all tests**

Run: `cargo test -p sparkdown-studio -- --nocapture`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add studio/src-tauri/
git commit -m "feat(studio): add Tauri commands for entity CRUD, type listing, and ThemeRegistry state"
```

---

## Task 7: Frontend — TypeScript Command Wrappers and State

**Files:**
- Modify: `studio/src/lib/tauri/commands.ts:3-76`
- Modify: `studio/src/lib/stores/document.svelte.ts:3-19`

- [ ] **Step 1: Add new TypeScript interfaces to commands.ts**

In `studio/src/lib/tauri/commands.ts`, add after the existing interfaces:

```typescript
export interface DocumentOverviewDto {
    title: string | null;
    entities: EntityDto[];
    sidecar_status: SidecarStatus;
}

export interface EntityDetailDto {
    entity: EntityDto;
    all_relations: Relation[];
    incoming_relations: Relation[];
    anchor_snippet: string;
    anchor_line: number;
}

export interface TypeCategoryDto {
    pack_name: string;
    category_label: string;
    types: TypeOptionDto[];
}

export interface TypeOptionDto {
    iri: string;
    curie: string;
    label: string;
    description?: string;
}
```

- [ ] **Step 2: Add new command wrapper functions**

```typescript
export async function createEntity(docId: string, charStart: number, charEnd: number, typeIri: string): Promise<EntityDto> {
    return invoke('create_entity', { docId, charStart, charEnd, typeIri });
}

export async function updateStaleAnchor(docId: string, entityId: string): Promise<void> {
    return invoke('update_stale_anchor', { docId, entityId });
}

export async function getDocumentOverview(docId: string): Promise<DocumentOverviewDto> {
    return invoke('get_document_overview', { docId });
}

export async function getEntityDetail(docId: string, entityId: string): Promise<EntityDetailDto> {
    return invoke('get_entity_detail', { docId, entityId });
}

export async function deleteEntity(docId: string, entityId: string): Promise<void> {
    return invoke('delete_entity', { docId, entityId });
}

export async function listAvailableTypes(): Promise<TypeCategoryDto[]> {
    return invoke('list_available_types');
}

export async function searchTypes(query: string, limit?: number): Promise<TypeOptionDto[]> {
    return invoke('search_types', { query, limit });
}
```

- [ ] **Step 3: Extend document store with new state**

In `studio/src/lib/stores/document.svelte.ts`, add:

```typescript
let selectedEntityId = $state<string | null>(null);
let entityDetail = $state<EntityDetailDto | null>(null);
let documentOverview = $state<DocumentOverviewDto | null>(null);
let dismissedStaleIds = $state<Set<string>>(new Set());

export function getSelectedEntityId() { return selectedEntityId; }
export function setSelectedEntityId(id: string | null) { selectedEntityId = id; }
export function getEntityDetail() { return entityDetail; }
export function setEntityDetail(detail: EntityDetailDto | null) { entityDetail = detail; }
export function getDocumentOverview() { return documentOverview; }
export function setDocumentOverview(overview: DocumentOverviewDto | null) { documentOverview = overview; }
export function getDismissedStaleIds() { return dismissedStaleIds; }
export function dismissStaleAnchor(id: string) { dismissedStaleIds = new Set([...dismissedStaleIds, id]); }
export function getVisibleStaleAnchors() { return staleAnchors.filter(a => !dismissedStaleIds.has(a.entity_id)); }
```

Update `clearDocumentState()` to also clear the new state:
```typescript
export function clearDocumentState() {
    entities = [];
    sidecarStatus = { synced: 0, stale: 0, detached: 0, total_triples: 0 };
    staleAnchors = [];
    selectedEntityId = null;
    entityDetail = null;
    documentOverview = null;
    dismissedStaleIds = new Set();
}
```

- [ ] **Step 4: Commit**

```bash
git add studio/src/lib/tauri/commands.ts studio/src/lib/stores/document.svelte.ts
git commit -m "feat(studio): add frontend command wrappers and document state for Phase 1.5 features"
```

---

## Task 8: Frontend — Entity Creation Popup

**Files:**
- Create: `studio/src/lib/components/EntityCreationPopup.svelte`
- Modify: `studio/src/lib/components/CodeMirrorEditor.svelte:35-88` (add Cmd+E keybinding)
- Modify: `studio/src/lib/components/EditorPane.svelte:1-42` (mount popup)

- [ ] **Step 1: Create EntityCreationPopup.svelte**

Create `studio/src/lib/components/EntityCreationPopup.svelte`:

```svelte
<script lang="ts">
    import { createEntity, listAvailableTypes, type TypeCategoryDto, type TypeOptionDto } from '$lib/tauri/commands';
    import { getActiveDocId } from '$lib/stores/workspace.svelte';
    import { entityColor } from '$lib/theme/colors';

    let { show = $bindable(false), charStart = 0, charEnd = 0, selectedText = '' } = $props();

    let categories = $state<TypeCategoryDto[]>([]);
    let searchQuery = $state('');
    let selectedIndex = $state(0);
    let loading = $state(false);
    let loaded = $state(false);

    let filteredTypes = $derived.by(() => {
        const query = searchQuery.toLowerCase();
        if (!query) return categories;
        return categories.map(cat => ({
            ...cat,
            types: cat.types.filter(t =>
                t.label.toLowerCase().includes(query) ||
                t.curie.toLowerCase().includes(query)
            )
        })).filter(cat => cat.types.length > 0);
    });

    let flatTypes = $derived(filteredTypes.flatMap(c => c.types));

    async function loadTypes() {
        if (loaded) return;
        loading = true;
        try {
            categories = await listAvailableTypes();
            loaded = true;
        } finally {
            loading = false;
        }
    }

    async function confirm(type_option: TypeOptionDto) {
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            await createEntity(docId, charStart, charEnd, type_option.iri);
            show = false;
        } catch (e) {
            console.error('Failed to create entity:', e);
        }
    }

    function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
            show = false;
        } else if (e.key === 'ArrowDown') {
            e.preventDefault();
            selectedIndex = Math.min(selectedIndex + 1, flatTypes.length - 1);
        } else if (e.key === 'ArrowUp') {
            e.preventDefault();
            selectedIndex = Math.max(selectedIndex - 1, 0);
        } else if (e.key === 'Enter' && flatTypes[selectedIndex]) {
            e.preventDefault();
            confirm(flatTypes[selectedIndex]);
        }
    }

    $effect(() => {
        if (show) {
            loadTypes();
            searchQuery = '';
            selectedIndex = 0;
        }
    });
</script>

{#if show}
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="popup-overlay" onkeydown={handleKeydown}>
    <div class="popup">
        <div class="popup-header">"{selectedText}"</div>
        <input
            class="search-input"
            bind:value={searchQuery}
            placeholder="Search types..."
            autofocus
        />
        {#if loading}
            <div class="loading">Loading types...</div>
        {:else}
            <div class="type-list">
                {#each filteredTypes as category}
                    <div class="category-label">{category.pack_name}</div>
                    {#each category.types as typeOpt, i}
                        {@const globalIdx = flatTypes.indexOf(typeOpt)}
                        <button
                            class="type-option"
                            class:selected={globalIdx === selectedIndex}
                            onclick={() => confirm(typeOpt)}
                        >
                            <span class="type-dot" style="background: {entityColor(typeOpt.curie)}"></span>
                            <span class="type-label">{typeOpt.label}</span>
                            <span class="type-curie">{typeOpt.curie}</span>
                        </button>
                    {/each}
                {/each}
            </div>
        {/if}
    </div>
</div>
{/if}

<style>
    .popup-overlay {
        position: fixed; inset: 0; z-index: 100;
    }
    .popup {
        position: absolute; top: 50%; left: 50%;
        transform: translate(-50%, -50%);
        background: #1a1a1a; border: 1px solid #333; border-radius: 8px;
        width: 320px; max-height: 400px; overflow: hidden;
        display: flex; flex-direction: column;
    }
    .popup-header {
        padding: 12px 16px 4px; color: #ccc; font-size: 13px;
        font-style: italic;
    }
    .search-input {
        margin: 8px 16px; padding: 6px 10px;
        background: #0f0f0f; border: 1px solid #444; border-radius: 4px;
        color: #eee; font-size: 13px; outline: none;
    }
    .search-input:focus { border-color: #666; }
    .loading { padding: 16px; color: #888; text-align: center; }
    .type-list { overflow-y: auto; max-height: 280px; padding-bottom: 8px; }
    .category-label {
        padding: 8px 16px 4px; color: #666; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
    }
    .type-option {
        display: flex; align-items: center; gap: 8px;
        width: 100%; padding: 6px 16px;
        background: none; border: none; color: #ddd;
        font-size: 13px; cursor: pointer; text-align: left;
    }
    .type-option:hover, .type-option.selected { background: #2a2a2a; }
    .type-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
    .type-curie { color: #666; font-size: 11px; margin-left: auto; }
</style>
```

- [ ] **Step 2: Add Cmd+E keybinding to CodeMirrorEditor.svelte**

In `studio/src/lib/components/CodeMirrorEditor.svelte`, add a callback prop (Svelte 5 pattern — no `createEventDispatcher`):

```typescript
// In the $props() declaration at the top of the script:
let { onCreateEntity }: { onCreateEntity?: (from: number, to: number, text: string) => void } = $props();
```

Then in the keymap setup (around line 42-50 where Mod+S is defined), add alongside it:

```typescript
{
    key: 'Mod-e',
    run: (view) => {
        const sel = view.state.selection.main;
        if (sel.from === sel.to) return false;
        const text = view.state.sliceDoc(sel.from, sel.to);
        onCreateEntity?.(sel.from, sel.to, text);
        return true;
    }
}
```

The `onCreateEntity` callback is passed as a prop from EditorPane.

- [ ] **Step 3: Mount EntityCreationPopup in EditorPane.svelte**

In `studio/src/lib/components/EditorPane.svelte`, import and mount using Svelte 5 callback props:

```svelte
<script lang="ts">
    import EntityCreationPopup from './EntityCreationPopup.svelte';

    let showEntityPopup = $state(false);
    let entityPopupStart = $state(0);
    let entityPopupEnd = $state(0);
    let entityPopupText = $state('');

    function handleCreateEntity(from: number, to: number, text: string) {
        entityPopupStart = from;
        entityPopupEnd = to;
        entityPopupText = text;
        showEntityPopup = true;
    }
</script>

<!-- Pass callback prop to CodeMirrorEditor -->
<CodeMirrorEditor onCreateEntity={handleCreateEntity} ... />

<!-- Entity creation popup -->
<EntityCreationPopup
    bind:show={showEntityPopup}
    charStart={entityPopupStart}
    charEnd={entityPopupEnd}
    selectedText={entityPopupText}
/>
```

- [ ] **Step 4: Commit**

```bash
git add studio/src/lib/components/EntityCreationPopup.svelte studio/src/lib/components/CodeMirrorEditor.svelte studio/src/lib/components/EditorPane.svelte
git commit -m "feat(studio): add Cmd+E entity creation popup with type picker"
```

---

## Task 9: Frontend — Stale Anchor Widget

**Files:**
- Create: `studio/src/lib/editor/stale-anchor-widget.ts`
- Modify: `studio/src/lib/components/CodeMirrorEditor.svelte:35-88` (add extension)

- [ ] **Step 1: Create stale-anchor-widget.ts**

Create `studio/src/lib/editor/stale-anchor-widget.ts`:

```typescript
import { EditorView, Decoration, WidgetType } from '@codemirror/view';
import { StateField, StateEffect } from '@codemirror/state';
import type { StaleAnchor } from '$lib/tauri/commands';

export const setStaleAnchorsEffect = StateEffect.define<StaleAnchor[]>();

class StaleAnchorWidget extends WidgetType {
    constructor(
        private staleAnchor: StaleAnchor,
        private onAccept: (entityId: string) => void,
        private onDismiss: (entityId: string) => void,
    ) { super(); }

    toDOM() {
        const wrap = document.createElement('div');
        wrap.className = 'stale-anchor-widget';
        wrap.innerHTML = `
            <span class="stale-label">↑ "${this.staleAnchor.old_snippet}" → "${this.staleAnchor.new_text}" — update?</span>
        `;

        const yBtn = document.createElement('button');
        yBtn.textContent = 'y';
        yBtn.className = 'stale-btn stale-accept';
        yBtn.onclick = () => this.onAccept(this.staleAnchor.entity_id);

        const nBtn = document.createElement('button');
        nBtn.textContent = 'n';
        nBtn.className = 'stale-btn stale-dismiss';
        nBtn.onclick = () => this.onDismiss(this.staleAnchor.entity_id);

        wrap.appendChild(yBtn);
        wrap.appendChild(nBtn);

        // Keyboard support
        wrap.addEventListener('keydown', (e) => {
            if (e.key === 'y') this.onAccept(this.staleAnchor.entity_id);
            if (e.key === 'n') this.onDismiss(this.staleAnchor.entity_id);
        });

        return wrap;
    }

    ignoreEvent() { return false; }
}

export function staleAnchorWidgets(
    onAccept: (entityId: string) => void,
    onDismiss: (entityId: string) => void,
) {
    return StateField.define({
        create() { return Decoration.none; },
        update(decos, tr) {
            for (const effect of tr.effects) {
                if (effect.is(setStaleAnchorsEffect)) {
                    const anchors: StaleAnchor[] = effect.value;
                    const widgets = anchors.map(sa =>
                        Decoration.widget({
                            widget: new StaleAnchorWidget(sa, onAccept, onDismiss),
                            block: true,
                        }).range(sa.span_end)
                    );
                    return Decoration.set(widgets, true);
                }
            }
            return decos.map(tr.changes);
        },
        provide: f => EditorView.decorations.from(f),
    });
}
```

- [ ] **Step 2: Wire into CodeMirrorEditor.svelte**

In `CodeMirrorEditor.svelte`, import and add the extension alongside existing ones:

```typescript
import { staleAnchorWidgets, setStaleAnchorsEffect } from '$lib/editor/stale-anchor-widget';
import { updateStaleAnchor } from '$lib/tauri/commands';
import { getActiveDocId } from '$lib/stores/workspace.svelte';
import { dismissStaleAnchor } from '$lib/stores/document.svelte';
```

Add to extensions array:
```typescript
staleAnchorWidgets(
    async (entityId) => {
        const docId = getActiveDocId();
        if (docId) await updateStaleAnchor(docId, entityId);
    },
    (entityId) => {
        dismissStaleAnchor(entityId);
    }
)
```

Add an `$effect` to dispatch stale anchor updates to the editor:
```typescript
$effect(() => {
    const anchors = getVisibleStaleAnchors(); // filtered by dismissedStaleIds
    if (editorView) {
        editorView.dispatch({
            effects: setStaleAnchorsEffect.of(anchors)
        });
    }
});
```

- [ ] **Step 3: Add CSS for stale anchor widgets**

Add to the component's `<style>` or a global CSS file:

```css
:global(.stale-anchor-widget) {
    display: flex; align-items: center; gap: 8px;
    padding: 4px 12px; margin: 2px 0;
    background: rgba(245, 158, 11, 0.1);
    border-left: 2px solid #F59E0B;
    font-size: 12px; color: #aaa;
}
:global(.stale-btn) {
    padding: 2px 8px; border: 1px solid #444;
    border-radius: 3px; background: #222; color: #ddd;
    cursor: pointer; font-size: 11px;
}
:global(.stale-btn:hover) { background: #333; }
:global(.stale-accept:hover) { border-color: #22C55E; }
:global(.stale-dismiss:hover) { border-color: #F43F5E; }
```

- [ ] **Step 4: Commit**

```bash
git add studio/src/lib/editor/stale-anchor-widget.ts studio/src/lib/components/CodeMirrorEditor.svelte
git commit -m "feat(studio): add stale anchor resolution widget with accept/dismiss buttons"
```

---

## Task 10: Frontend — Knowledge Panel

**Files:**
- Create: `studio/src/lib/components/KnowledgePanel.svelte`
- Create: `studio/src/lib/components/EntityList.svelte`
- Create: `studio/src/lib/components/EntityDetail.svelte`
- Modify: `studio/src/routes/+page.svelte:31-42` (add panel to layout)

- [ ] **Step 1: Create EntityList.svelte**

Create `studio/src/lib/components/EntityList.svelte`:

```svelte
<script lang="ts">
    import type { EntityDto } from '$lib/tauri/commands';
    import { entityColor } from '$lib/theme/colors';

    let { entities = [], onSelect }: { entities: EntityDto[], onSelect: (id: string) => void } = $props();

    // Group entities by type_prefix
    let grouped = $derived.by(() => {
        const groups = new Map<string, EntityDto[]>();
        for (const e of entities) {
            const key = e.type_prefix || 'Unknown';
            if (!groups.has(key)) groups.set(key, []);
            groups.get(key)!.push(e);
        }
        return groups;
    });
</script>

<div class="entity-list">
    {#each [...grouped.entries()] as [type_prefix, group]}
        <div class="type-group-label">{type_prefix}</div>
        {#each group as entity}
            <button class="entity-row" onclick={() => onSelect(entity.id)}>
                <span class="dot" style="background: {entityColor(entity.type_prefix)}"></span>
                <span class="label">{entity.label}</span>
                <span class="status status-{entity.status}">{entity.status}</span>
            </button>
        {/each}
    {/each}
    {#if entities.length === 0}
        <div class="empty">No entities yet. Select text and press Cmd+E to create one.</div>
    {/if}
</div>

<style>
    .entity-list { display: flex; flex-direction: column; }
    .type-group-label {
        padding: 8px 12px 4px; color: #666; font-size: 11px;
        text-transform: uppercase; letter-spacing: 0.5px;
    }
    .entity-row {
        display: flex; align-items: center; gap: 8px;
        padding: 6px 12px; background: none; border: none;
        color: #ddd; font-size: 13px; cursor: pointer; text-align: left;
    }
    .entity-row:hover { background: #2a2a2a; }
    .dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
    .label { flex: 1; }
    .status { font-size: 10px; color: #666; }
    .status-stale { color: #F59E0B; }
    .status-detached { color: #F43F5E; }
    .empty { padding: 16px 12px; color: #555; font-size: 13px; }
</style>
```

- [ ] **Step 2: Create EntityDetail.svelte**

Create `studio/src/lib/components/EntityDetail.svelte`:

```svelte
<script lang="ts">
    import type { EntityDetailDto } from '$lib/tauri/commands';
    import { deleteEntity } from '$lib/tauri/commands';
    import { getActiveDocId } from '$lib/stores/workspace.svelte';
    import { entityColor } from '$lib/theme/colors';

    let { detail, onBack, onNavigate }:
        { detail: EntityDetailDto, onBack: () => void, onNavigate: (entityId: string) => void } = $props();

    async function handleDelete() {
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            await deleteEntity(docId, detail.entity.id);
            onBack();
        } catch (e) {
            console.error('Failed to delete entity:', e);
        }
    }
</script>

<div class="entity-detail">
    <button class="back-btn" onclick={onBack}>← Back</button>

    <div class="header">
        <span class="dot" style="background: {entityColor(detail.entity.type_prefix)}"></span>
        <div>
            <div class="name">{detail.entity.label}</div>
            <div class="type">{detail.entity.type_prefix}</div>
        </div>
        <span class="status status-{detail.entity.status}">{detail.entity.status}</span>
    </div>

    <div class="section">
        <div class="section-label">Anchor</div>
        <div class="snippet">"{detail.anchor_snippet}" — line {detail.anchor_line}</div>
    </div>

    {#if detail.all_relations.length > 0}
        <div class="section">
            <div class="section-label">Relations</div>
            {#each detail.all_relations as rel}
                <div class="relation">
                    <span class="pred">{rel.predicate_label}</span> →
                    {#if rel.target_id}
                        <button class="link" onclick={() => onNavigate(rel.target_id)}>{rel.target_label}</button>
                    {:else}
                        <span class="literal">{rel.target_label}</span>
                    {/if}
                </div>
            {/each}
        </div>
    {/if}

    {#if detail.incoming_relations.length > 0}
        <div class="section">
            <div class="section-label">Referenced by</div>
            {#each detail.incoming_relations as rel}
                <div class="relation">
                    <span class="pred">{rel.predicate_label}</span> ←
                    <button class="link" onclick={() => onNavigate(rel.target_id)}>{rel.target_label}</button>
                </div>
            {/each}
        </div>
    {/if}

    <div class="actions">
        <button class="delete-btn" onclick={handleDelete}>Delete entity</button>
    </div>
</div>

<style>
    .entity-detail { display: flex; flex-direction: column; gap: 12px; padding: 12px; }
    .back-btn { background: none; border: none; color: #888; cursor: pointer; text-align: left; padding: 0; font-size: 12px; }
    .back-btn:hover { color: #ddd; }
    .header { display: flex; align-items: center; gap: 8px; }
    .dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
    .name { color: #eee; font-size: 14px; font-weight: 500; }
    .type { color: #888; font-size: 12px; }
    .status { font-size: 10px; color: #666; margin-left: auto; }
    .status-stale { color: #F59E0B; }
    .status-detached { color: #F43F5E; }
    .section-label { color: #555; font-size: 11px; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px; }
    .snippet { color: #aaa; font-size: 12px; font-style: italic; }
    .relation { font-size: 13px; color: #ccc; padding: 2px 0; }
    .pred { color: #888; }
    .link { background: none; border: none; color: #8B5CF6; cursor: pointer; padding: 0; font-size: 13px; }
    .link:hover { text-decoration: underline; }
    .literal { color: #aaa; }
    .actions { margin-top: 12px; padding-top: 12px; border-top: 1px solid #333; }
    .delete-btn {
        background: none; border: 1px solid #F43F5E33; color: #F43F5E; border-radius: 4px;
        padding: 4px 12px; font-size: 12px; cursor: pointer;
    }
    .delete-btn:hover { background: #F43F5E11; }
</style>
```

- [ ] **Step 3: Create KnowledgePanel.svelte**

Create `studio/src/lib/components/KnowledgePanel.svelte`:

```svelte
<script lang="ts">
    import { getDocumentOverview, getEntityDetail, type DocumentOverviewDto, type EntityDetailDto } from '$lib/tauri/commands';
    import { getActiveDocId } from '$lib/stores/workspace.svelte';
    import { getEntities, getSelectedEntityId, setSelectedEntityId, setEntityDetail, getEntityDetail as getEntityDetailState } from '$lib/stores/document.svelte';
    import EntityList from './EntityList.svelte';
    import EntityDetail from './EntityDetail.svelte';

    let collapsed = $state(false);
    let overview = $state<DocumentOverviewDto | null>(null);
    let detail = $derived(getEntityDetailState());
    let selectedId = $derived(getSelectedEntityId());
    let entities = $derived(getEntities());

    async function loadOverview() {
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            overview = await getDocumentOverview(docId);
        } catch (e) {
            console.error('Failed to load overview:', e);
        }
    }

    async function selectEntity(id: string) {
        setSelectedEntityId(id);
        const docId = getActiveDocId();
        if (!docId) return;
        try {
            const detailData = await getEntityDetail(docId, id);
            setEntityDetail(detailData);
        } catch (e) {
            console.error('Failed to load entity detail:', e);
        }
    }

    function goBack() {
        setSelectedEntityId(null);
        setEntityDetail(null);
    }

    // Refresh when entities change
    $effect(() => {
        entities; // subscribe to entity changes
        loadOverview();
    });
</script>

<div class="knowledge-panel" class:collapsed>
    <button class="collapse-toggle" onclick={() => collapsed = !collapsed}>
        {collapsed ? '◀' : '▶'}
    </button>

    {#if !collapsed}
        <div class="panel-content">
            <div class="panel-header">Knowledge</div>

            {#if selectedId && detail}
                <EntityDetail
                    {detail}
                    onBack={goBack}
                    onNavigate={selectEntity}
                />
            {:else}
                <EntityList
                    entities={overview?.entities ?? []}
                    onSelect={selectEntity}
                />
                {#if overview}
                    <div class="summary">
                        {overview.sidecar_status.total_triples} triples ·
                        {overview.sidecar_status.synced} synced
                        {#if overview.sidecar_status.stale > 0}
                            · {overview.sidecar_status.stale} stale
                        {/if}
                    </div>
                {/if}
            {/if}
        </div>
    {:else}
        <div class="collapsed-label">
            {entities.length}
        </div>
    {/if}
</div>

<style>
    .knowledge-panel {
        width: 280px; min-width: 280px;
        background: #1a1a1a; border-left: 1px solid #333;
        display: flex; flex-direction: column;
        position: relative;
    }
    .knowledge-panel.collapsed { width: 24px; min-width: 24px; }
    .collapse-toggle {
        position: absolute; left: -12px; top: 8px;
        width: 24px; height: 24px; border-radius: 50%;
        background: #2a2a2a; border: 1px solid #444;
        color: #888; cursor: pointer; font-size: 10px;
        display: flex; align-items: center; justify-content: center;
        z-index: 10;
    }
    .collapse-toggle:hover { background: #333; color: #ddd; }
    .panel-content { flex: 1; overflow-y: auto; }
    .panel-header {
        padding: 12px; color: #888; font-size: 11px;
        text-transform: uppercase; letter-spacing: 1px;
        border-bottom: 1px solid #333;
    }
    .summary {
        padding: 12px; color: #555; font-size: 11px;
        border-top: 1px solid #333;
    }
    .collapsed-label {
        writing-mode: vertical-lr; text-align: center;
        padding: 12px 4px; color: #666; font-size: 12px;
    }
</style>
```

- [ ] **Step 4: Update +page.svelte layout to include KnowledgePanel**

In `studio/src/routes/+page.svelte`, modify the layout (around line 31-42) to add the third pane:

```svelte
<script lang="ts">
    import KnowledgePanel from '$lib/components/KnowledgePanel.svelte';
    // ... existing imports
</script>

<main class="app-layout">
    <Sidebar ... />
    <EditorPane ... />
    {#if activeDocId}
        <KnowledgePanel />
    {/if}
</main>
```

- [ ] **Step 5: Commit**

```bash
git add studio/src/lib/components/EntityList.svelte studio/src/lib/components/EntityDetail.svelte studio/src/lib/components/KnowledgePanel.svelte studio/src/routes/+page.svelte
git commit -m "feat(studio): add Knowledge Panel with entity list and detail views"
```

---

## Task 11: Integration — Full Build and Test

**Files:** All modified files

- [ ] **Step 1: Run all Rust tests**

Run: `cargo test -- --nocapture`
Expected: All tests PASS across all crates

- [ ] **Step 2: Build the full Tauri app**

Run: `cd studio && cargo tauri build --debug 2>&1 | tail -20`
Expected: BUILD SUCCESS

- [ ] **Step 3: Check frontend build**

Run: `cd studio && pnpm build`
Expected: BUILD SUCCESS with no TypeScript errors

- [ ] **Step 4: Manual smoke test checklist**

Verify the following flow works end-to-end:
1. Open the app, open a workspace folder containing `.md` files
2. Open a `.md` file that has an existing `.sparkdown-sem` sidecar
3. Verify entity gutter dots and underlines appear
4. Select text, press `Cmd+E`, verify type picker popup appears
5. Pick a type, verify entity appears in gutter, underlines, and Knowledge Panel
6. Hover an entity in the editor, verify whisper card shows
7. Edit text under an existing entity to make it stale
8. Verify stale anchor widget appears with accept/dismiss buttons
9. Click accept, verify entity goes back to synced
10. Click an entity in the Knowledge Panel, verify detail view shows
11. Click "Delete entity", verify it's removed from everywhere
12. Save (`Cmd+S`), verify `.sparkdown-sem` is updated on disk

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(studio): complete Phase 1.5 integration — entity creation, stale resolution, Knowledge Panel"
```

---

## Deferred Items

The following spec features are explicitly deferred to a follow-up task to keep this plan focused:

- **Editor→Panel bidirectional sync** (Cmd+Click on entity underline selects in panel, panel click scrolls editor with temp highlight) — requires adding a CM `keymap` extension for Cmd+Click and a `scrollIntoView` dispatch from panel. Can be added as a polish task after core features work.
- **Built-in pack conversion** (converting existing builtins to produce TypeCategoryDto shape with human-readable category labels) — the current `list_available_types` uses `base_iri` as the category label, which is functional but not ideal. A follow-up task should add category metadata to the builtin providers.
- **Workspace.toml reading** — the pack loading sequence reads `.sparkdown/workspace.toml` to determine active packs. For MVP, all installed packs are active. Workspace-level configuration can be added when users have multiple pack sets to manage.

These can be addressed in a "Phase 1.5 polish" task after the core features are verified working.
