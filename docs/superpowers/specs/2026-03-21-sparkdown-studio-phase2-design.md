# Sparkdown Studio Phase 1.5: Knowledge Authoring

> Adds entity creation, stale anchor resolution, Knowledge Panel, and an ontology pack system to make Sparkdown Studio a usable daily-driver and demo-worthy knowledge application.
> Companion to: `2026-03-21-sparkdown-studio-tauri-design.md` (Phase 1 Tauri spec), `2026-03-21-sparkdown-studio-ui-design.md` (UI vision).

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Feature order | Entity creation → Stale resolution → Knowledge Panel | Natural dependency chain: write path → mutation path → read view |
| Ontology extensibility | Pack registry model | Makes Studio a platform, not a tool with hardcoded vocabularies |
| Pack format | TOML definitions (not OWL import) | Curated, fast-loading, human-editable; OWL import can be a future ingestion tool |
| Type picker UX | Categorized by pack + searchable | Handles large ontologies like FIBO without overwhelming the user |
| Stale dismiss | Frontend-only (hide widget) | Entity stays Stale in graph; no backend command needed yet |
| Knowledge Panel data | Separate GetEntityDetail command | Avoids bloating the hot-path EntityDto with full relation lists |
| Editor ↔ Panel sync | Shared selectedEntityId rune | Bidirectional: click in panel scrolls editor, click in editor selects in panel |

---

## Part 1: Ontology Pack System

### Concept

Ontologies are distributed as **packs** — self-contained bundles of type and property definitions. The existing builtins (schema.org, Dublin Core, FOAF, sd:) become built-in packs. Users can install additional packs (e.g., FIBO) globally or per-workspace.

### Pack structure

```
<pack-name>/
  pack.toml            # metadata, namespace prefixes, curated categories
  types.toml           # type definitions with labels, parent types, suggested properties
  properties.toml      # property definitions with expected types, labels
  README.md            # human-readable docs (shown in pack browser)
```

### pack.toml

```toml
[pack]
name = "fibo-foundations"
version = "1.0.0"
description = "FIBO Foundations: Legal entities, contracts, currencies, and financial instruments"
source = "https://spec.edmcouncil.org/fibo/"

[prefixes]
fibo-fnd = "https://spec.edmcouncil.org/fibo/ontology/FND/"
fibo-be = "https://spec.edmcouncil.org/fibo/ontology/BE/"
lcc-cr = "https://www.omg.org/spec/LCC/Countries/CountryRepresentation/"

[categories]
legal = { label = "Legal Entities", types = [
    "fibo-be:LegalEntity",
    "fibo-be:Corporation",
    "fibo-be:Partnership",
    "fibo-be:GovernmentBody",
] }
financial = { label = "Financial", types = [
    "fibo-fnd:Currency",
    "fibo-fnd:MonetaryAmount",
    "fibo-fnd:Contract",
    "fibo-fnd:FinancialInstrument",
] }
```

### types.toml

```toml
[[types]]
iri = "https://spec.edmcouncil.org/fibo/ontology/BE/LegalEntities/LegalPersons/LegalEntity"
curie = "fibo-be:LegalEntity"
label = "Legal Entity"
description = "An entity that can enter into contracts and has legal rights and obligations"
parent = "fibo-fnd:AutonomousAgent"
suggested_properties = ["fibo-fnd:hasLegalName", "fibo-be:isOrganizedIn", "fibo-fnd:hasDateOfIncorporation"]
```

### properties.toml

```toml
[[properties]]
iri = "https://spec.edmcouncil.org/fibo/ontology/FND/AgentsAndPeople/Agents/hasLegalName"
curie = "fibo-fnd:hasLegalName"
label = "Legal Name"
expected_type = "Text"
description = "The legal name of an entity as registered"
```

### Where packs live

| Location | Scope | Path |
|----------|-------|------|
| Built-in | Always available | Compiled into sparkdown-ontology crate |
| Global | All workspaces for this user | `~/.sparkdown/ontology-packs/<pack-name>/` |
| Workspace-local | Single workspace | `.sparkdown/ontology-packs/<pack-name>/` |

### Activation

Active packs are declared per-workspace in `.sparkdown/workspace.toml`:

```toml
[ontology]
active_packs = ["schema-org", "dublin-core", "foaf", "fibo-foundations"]
```

Built-in packs (schema-org, dublin-core, foaf, sparkdown) are always active unless explicitly excluded. Additional packs must be listed to be active.

Per-document override is possible via YAML frontmatter `@context` prefixes — if a document uses a prefix from an installed pack, that pack's types are available in the type picker for that document.

### ThemeRegistry integration

The existing `ThemeRegistry` uses the `OntologyProvider` trait. Each pack produces an `OntologyProvider` implementation by parsing its TOML files at workspace-open time.

```rust
// Existing trait (sparkdown-ontology/src/registry.rs) — reproduced for reference:
trait OntologyProvider: Send + Sync {
    fn prefix(&self) -> &str;
    fn base_iri(&self) -> &str;
    fn lookup_type(&self, local_name: &str) -> Option<&TypeDef>;
    fn lookup_property(&self, local_name: &str) -> Option<&PropertyDef>;
    fn all_types(&self) -> Vec<&TypeDef>;
    fn all_properties(&self) -> Vec<&PropertyDef>;
}
```

New additions to support the pack system:

```rust
/// A loaded ontology pack with metadata and categories.
struct OntologyPack {
    metadata: PackMetadata,       // from pack.toml [pack] section
    prefixes: HashMap<String, String>,
    categories: Vec<TypeCategory>,
    provider: Box<dyn OntologyProvider>,
}

struct PackMetadata {
    name: String,
    version: String,
    description: String,
    source: Option<String>,
}

struct TypeCategory {
    key: String,              // "legal", "financial"
    label: String,            // "Legal Entities"
    types: Vec<String>,       // CURIEs: ["fibo-be:LegalEntity", ...]
}

/// Registry extension: loads packs from disk, provides them to the type picker.
impl ThemeRegistry {
    fn load_packs(&mut self, paths: &[PathBuf]) -> Result<(), OntologyError>;
    fn active_packs(&self) -> &[OntologyPack];
    fn all_type_categories(&self) -> Vec<TypeCategoryDto>;
    fn search_types(&self, query: &str, limit: Option<usize>) -> Vec<TypeOptionDto>;
}
```

### ThemeRegistry as Tauri state

The `ThemeRegistry` is loaded once on workspace open (via `load_packs`) and then read-only for the duration. It is wrapped in `Arc<tokio::sync::RwLock<ThemeRegistry>>` as Tauri managed state:

```rust
type ThemeRegistryState = Arc<tokio::sync::RwLock<ThemeRegistry>>;
```

Standalone commands (`list_available_types`, `search_types`) acquire a read lock. The `load_packs` call at workspace-open acquires a write lock.

### Session access to the registry

Currently each `DocumentSession` creates its own `ThemeRegistry::with_builtins()` (unused, prefixed with `_`). In Phase 1.5, sessions receive an `Arc<tokio::sync::RwLock<ThemeRegistry>>` at construction time instead. The session's `entity_to_dto()` method uses the shared registry for type/property label resolution, replacing the current `iri_to_curie()` and `iri_local_name()` helper functions.

### Pack loading sequence

1. On workspace open, read `.sparkdown/workspace.toml` (if exists)
2. Scan built-in packs + `~/.sparkdown/ontology-packs/` + `.sparkdown/ontology-packs/`
3. Load and register active packs into `ThemeRegistry`
4. Cache `all_type_categories()` result for the type picker

---

## Part 2: Entity Creation

### User flow

1. User selects text in CodeMirror (e.g., "Dr. Sarah Chen")
2. Presses `Cmd+E` (or `Ctrl+E` on Linux)
3. Entity creation popup appears anchored below the selection:
   - Selected text displayed as the entity label
   - Search box for filtering types across all active packs
   - Type list grouped by pack and category (e.g., "Schema.org > People & Organizations > Person")
   - First type pre-selected based on simple string matching against category labels
4. User picks a type (click, or arrow keys + Enter)
5. Entity is written to the graph immediately
6. Events fire: `entities-updated`, `sidecar-status`
7. New entity appears with gutter dot, underline, and is visible in Knowledge Panel
8. Popup dismisses; cursor returns to editor

### Backend: SessionCommand

The session actor operates in byte-offset space (consistent with Phase 1). Character-to-byte conversion happens in the Tauri command layer (`commands.rs`), not in the session.

```rust
// In session.rs — operates on byte offsets like all other SessionCommands
SessionCommand::CreateEntity {
    span_start: usize,       // byte offset (converted from char offset in commands.rs)
    span_end: usize,
    type_iri: String,        // full IRI, resolved from CURIE by frontend
    reply: oneshot::Sender<Result<EntityDto, String>>,
}
```

```rust
// In commands.rs — Tauri command does the char→byte conversion
#[tauri::command]
async fn create_entity(
    doc_id: String,
    char_start: usize,    // UTF-16 character offset from CodeMirror
    char_end: usize,
    type_iri: String,
    registry: State<'_, SessionRegistryState>,
) -> Result<EntityDto, String> {
    // Validation: reject zero-width selection
    if char_start >= char_end {
        return Err("Selection must not be empty".into());
    }
    // char→byte conversion happens here, using the session's current source
    // (fetched via a GetSource helper or passed along with the command)
    ...
}
```

Session handler logic:

1. Validate `span_start < span_end` and both within `source.len()`
2. Validate `type_iri` is a syntactically valid IRI
3. Extract snippet: `source[span_start..span_end].to_string()` (truncated to ~40 chars for anchor storage)
4. Generate blank node ID using a monotonic counter on the session: `_:e{self.next_entity_id}` (counter increments on every create, never reuses IDs even after deletions)
5. Create `SemanticEntity` with:
   - `id`: new blank node via `graph::blank_node(&format!("e{}", self.next_entity_id))`
   - `anchor`: `Anchor::new(span_start..span_end, snippet)`
   - `types`: `vec![NamedNode::new_unchecked(type_iri)]`
   - `status`: `AnchorStatus::Synced`
6. Add entity to `graph.entities`
7. Rebuild `MappingIndex::build(&graph)`
8. Build and return `EntityDto`
9. Emit `entities-updated` and `sidecar-status` events

Error cases:
- Zero-width or inverted selection → `Err("Selection must not be empty")`
- Offsets out of bounds → `Err("Selection range exceeds document length")`
- Invalid IRI → `Err("Invalid type IRI: ...")`
- Selection spans a non-UTF8 boundary → caught by byte offset conversion

### Character ↔ byte offset conversion

Phase 1 has `byte_to_char_offset()`. Entity creation needs the reverse:

```rust
/// Maps UTF-16 character offset to byte offset in the source string.
fn char_to_byte_offset(source: &str, char_offset: usize) -> usize {
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

### Backend: type picker data

```rust
/// Standalone command (not per-session), called once on workspace open.
#[tauri::command]
async fn list_available_types(
    registry: State<'_, ThemeRegistryState>,
) -> Result<Vec<TypeCategoryDto>, String>;

/// Search types across all active packs.
#[tauri::command]
async fn search_types(
    query: String,
    registry: State<'_, ThemeRegistryState>,
) -> Result<Vec<TypeOptionDto>, String>;
```

DTOs:

```rust
#[derive(Serialize, Deserialize)]
struct TypeCategoryDto {
    pack_name: String,          // "schema-org", "fibo-foundations"
    category_label: String,     // "People & Organizations", "Legal Entities"
    types: Vec<TypeOptionDto>,
}

#[derive(Serialize, Deserialize)]
struct TypeOptionDto {
    iri: String,                // "http://schema.org/Person"
    curie: String,              // "schema:Person"
    label: String,              // "Person"
    description: Option<String>,
}
```

### Frontend: TypeScript types

```typescript
interface TypeCategoryDto {
    pack_name: string
    category_label: string
    types: TypeOptionDto[]
}

interface TypeOptionDto {
    iri: string
    curie: string
    label: string
    description?: string
}
```

### Frontend: new commands

```typescript
async function createEntity(docId: string, charStart: number, charEnd: number, typeIri: string): Promise<EntityDto>
async function listAvailableTypes(): Promise<TypeCategoryDto[]>
async function searchTypes(query: string): Promise<TypeOptionDto[]>
```

### Frontend: EntityCreationPopup.svelte

New component mounted in `EditorPane.svelte`. Triggered by `Cmd+E` keybinding registered in CodeMirror.

State:
```typescript
let showPopup = $state(false);
let selectionRange = $state<{ from: number; to: number } | null>(null);
let searchQuery = $state('');
let categories = $state<TypeCategoryDto[]>([]);
let filteredTypes = $derived(/* filter categories by searchQuery */);
let selectedIndex = $state(0);
```

Behavior:
- `Cmd+E` with active selection → set `selectionRange`, fetch `listAvailableTypes()` (cached after first call), show popup
- Search box filters types client-side (instant, no IPC round-trip per keystroke)
- Arrow keys navigate, Enter confirms
- Escape or clicking outside dismisses
- On confirm: call `createEntity()`, close popup

### Keybinding registration

In `CodeMirrorEditor.svelte`, add to the CM keymap:

```typescript
keymap.of([{
    key: 'Mod-e',
    run: (view) => {
        const sel = view.state.selection.main;
        if (sel.from === sel.to) return false; // no selection
        triggerEntityCreation(sel.from, sel.to);
        return true;
    }
}])
```

---

## Part 3: Stale Anchor Resolution

### How staleness is detected

The existing `UpdateSource` hot path runs `sync::sync_graph()`, which adjusts anchors and marks entities as `Stale` when their snippet text changes. The `stale-anchors` event already fires with `StaleAnchor` payloads. Phase 1 shows the count in the suggestion tray.

This part adds **inline UI to accept or dismiss** stale anchors.

### Inline stale decoration

New CodeMirror extension: `staleAnchorWidget` in `lib/editor/stale-anchor-widget.ts`.

For each entry in the `staleAnchors` rune, renders a `Decoration.widget` positioned after the entity's span end:

```
Niko Matsakis will deliver the opening talk.
  ↑ "keynote" → "opening talk" — update? [y] [n]
```

Widget properties:
- Positioned as a block widget below the line containing the entity
- Shows old snippet → new text as a compact diff
- Two action buttons: accept (`y`) and dismiss (`n`)
- Keyboard accessible: `y`/`n` keys when widget is focused, `Tab` cycles between stale widgets
- Widget removed from DOM after action (reactively, via state change)

### Backend: SessionCommand

Note: Phase 1 defined an extension point `UpdateStaleAnchor { entity_id, reply: oneshot::Sender<()> }`. We wrap the reply in `Result` to allow returning errors (e.g., entity not found).

```rust
SessionCommand::UpdateStaleAnchor {
    entity_id: String,
    reply: oneshot::Sender<Result<(), String>>,
}
```

Session handler logic:

1. Find entity by `entity_id` in `graph.entities`. If not found, return `Err("Entity not found: ...")`
2. Read current source text at the entity's anchor span: `source[anchor.span.start..anchor.span.end]`
3. Update the anchor's stored snippet to match the current text (truncated to ~40 chars)
4. Set entity status to `AnchorStatus::Synced`
5. Rebuild `MappingIndex`
6. Emit `entities-updated` and `sidecar-status` events
7. Emit updated `stale-anchors` event (with this entity removed from the list)

### Dismiss behavior

Dismissing a stale anchor is frontend-only: the widget is hidden by adding the entity ID to a local `dismissedStaleIds` set. The entity remains `Stale` in the graph. On next source update (which re-runs sync), the stale anchor may reappear if still stale, or resolve naturally if the text matches again.

No backend command needed for dismiss. A persistent `Acknowledged` status can be added later if desired.

### Frontend: new command

```typescript
async function updateStaleAnchor(docId: string, entityId: string): Promise<void>
```

### Frontend: state

In `document.svelte.ts`:
```typescript
let dismissedStaleIds = $state<Set<string>>(new Set());
let visibleStaleAnchors = $derived(
    staleAnchors.filter(a => !dismissedStaleIds.has(a.entity_id))
);
```

Clear `dismissedStaleIds` when switching documents.

---

## Part 4: Knowledge Panel

### Layout

The main app layout expands from two panes to three:

```
+-- Sidebar (240px fixed) --+-- EditorPane (flex) --+-- KnowledgePanel (280px, collapsible) --+
```

`KnowledgePanel.svelte` is conditionally rendered when a document is open. A collapse toggle button on the panel's left edge minimizes it to a thin strip (24px) showing just the entity count.

### View 1: Document Overview

Shown when no entity is selected (`selectedEntityId === null`).

Contents:
- **Document title** — from frontmatter `title` field, or filename if no frontmatter
- **Entity list** — all entities in the document, grouped by type:
  - Each entry: colored dot (type color), entity label, type CURIE, status badge
  - Click an entity → switch to Entity Detail view + scroll editor to span
- **Sidecar summary** — synced/stale/detached counts, total triples
- **Empty state** — when no entities exist: "No entities yet. Select text and press Cmd+E to create one."

### View 2: Entity Detail

Shown when an entity is selected (`selectedEntityId !== null`).

Contents:
- **Back button** — returns to Document Overview
- **Entity header** — label, type CURIE (with colored dot), status badge
- **Anchor snippet** — the text this entity is anchored to, with "line N" indicator. Click to scroll editor.
- **Outgoing relations** — all triples where this entity is subject:
  - `predicate_label → target_label` (clickable if target is an entity in this document)
- **Incoming relations** — all triples where this entity is object:
  - `predicate_label ← source_label` (clickable if source is an entity in this document)
- **Actions section**:
  - "Delete entity" — removes entity from graph, triggers events

### Backend: new SessionCommands

```rust
SessionCommand::GetDocumentOverview {
    reply: oneshot::Sender<DocumentOverviewDto>,
}

SessionCommand::GetEntityDetail {
    entity_id: String,
    reply: oneshot::Sender<Result<EntityDetailDto, String>>,
}

SessionCommand::DeleteEntity {
    entity_id: String,
    reply: oneshot::Sender<Result<(), String>>,
}
```

### DTOs

```rust
#[derive(Serialize, Deserialize)]
struct DocumentOverviewDto {
    title: Option<String>,
    entities: Vec<EntityDto>,       // full list, reuses existing DTO
    sidecar_status: SidecarStatus,
}

#[derive(Serialize, Deserialize)]
struct EntityDetailDto {
    entity: EntityDto,              // base entity data
    all_relations: Vec<Relation>,   // outgoing, no cap (unlike EntityDto.top_relations max 2)
    incoming_relations: Vec<Relation>,
    anchor_snippet: String,
    anchor_line: usize,             // 1-based line number
}
```

### DeleteEntity handler

1. Find entity by `entity_id` in `graph.entities`. If not found, return `Err("Entity not found: ...")`
2. Remove entity from `graph.entities` via `retain()`
3. Remove all referencing triples via `graph.triples.retain(|t| ...)` — checking both subject and object fields. Note: the existing `graph.triples_referencing()` is read-only and cannot be used for removal directly.
4. Rebuild `MappingIndex`
5. Emit `entities-updated` and `sidecar-status`
6. Return `Ok(())`

### Frontend: new commands

```typescript
async function getDocumentOverview(docId: string): Promise<DocumentOverviewDto>
async function getEntityDetail(docId: string, entityId: string): Promise<EntityDetailDto>
async function deleteEntity(docId: string, entityId: string): Promise<void>
```

### Frontend: state

In `document.svelte.ts`:
```typescript
let selectedEntityId = $state<string | null>(null);
let entityDetail = $state<EntityDetailDto | null>(null);
let documentOverview = $state<DocumentOverviewDto | null>(null);
```

### Editor ↔ Panel bidirectional sync

**Panel → Editor**: Clicking an entity in the panel dispatches a CodeMirror `scrollIntoView` effect targeting the entity's character span, and applies a temporary highlight decoration (stronger opacity, 500ms fade).

**Editor → Panel**: A new CM extension detects `Cmd+Click` (or `Ctrl+Click` on Linux) on entity-decorated text. It resolves the entity ID from the `entities` state at that position and sets `selectedEntityId`.

Implementation: both directions use the shared `selectedEntityId` rune. The editor extension watches it via `$effect` to apply scroll/highlight. The panel watches it to switch between overview and detail views.

### Panel refresh

The panel re-fetches `getDocumentOverview` whenever `entities-updated` fires. If an entity is selected, it also re-fetches `getEntityDetail` to reflect any changes (e.g., staleness resolution changed status).

---

## Part 5: New IPC Commands Summary

### Session-routed commands (new)

| Command | Input | Output | Used by |
|---------|-------|--------|---------|
| `create_entity` | doc_id, char_start, char_end, type_iri | `EntityDto` | Entity creation popup |
| `update_stale_anchor` | doc_id, entity_id | `()` | Stale anchor widget |
| `get_document_overview` | doc_id | `DocumentOverviewDto` | Knowledge Panel overview |
| `get_entity_detail` | doc_id, entity_id | `EntityDetailDto` | Knowledge Panel detail |
| `delete_entity` | doc_id, entity_id | `()` | Knowledge Panel actions |

### Standalone commands (new)

| Command | Input | Output | Used by |
|---------|-------|--------|---------|
| `list_available_types` | — | `Vec<TypeCategoryDto>` | Entity creation type picker |
| `search_types` | query | `Vec<TypeOptionDto>` | Entity creation search |

---

## Part 6: New Frontend Components

| Component | Location | Purpose |
|-----------|----------|---------|
| `EntityCreationPopup.svelte` | `lib/components/` | Type picker popup triggered by `Cmd+E` |
| `KnowledgePanel.svelte` | `lib/components/` | Right panel with overview/detail views |
| `EntityList.svelte` | `lib/components/` | Reusable entity list grouped by type |
| `EntityDetail.svelte` | `lib/components/` | Entity detail view within Knowledge Panel |
| `stale-anchor-widget.ts` | `lib/editor/` | CM extension for inline stale resolution |

---

## Part 7: New Backend Files

| File | Location | Purpose |
|------|----------|---------|
| `pack.rs` | `sparkdown-ontology/src/` | Pack loading: parse TOML, produce OntologyProvider |
| `pack_types.rs` | `studio/src-tauri/src/` | DTOs for type picker (TypeCategoryDto, TypeOptionDto) |

Modified files:
- `session.rs` — new SessionCommand variants (CreateEntity, UpdateStaleAnchor, GetDocumentOverview, GetEntityDetail, DeleteEntity)
- `commands.rs` — new #[tauri::command] functions
- `types.rs` — new DTOs (DocumentOverviewDto, EntityDetailDto, char_to_byte_offset)
- `events.rs` — no new events; existing events carry the updated data
- `lib.rs` — register new commands, manage ThemeRegistry state
- `registry.rs` — ThemeRegistry held as Tauri managed state for standalone commands

---

## Part 8: Implementation Order

### Step 1: Ontology Pack System (sparkdown-ontology)

Add `pack.rs` module to sparkdown-ontology. Define `OntologyPack`, pack TOML parsing, `TypeCategory`, and `ThemeRegistry::load_packs()`. Convert existing builtins to produce the same `TypeCategoryDto` shape. Write unit tests for pack loading and type search.

### Step 2: Entity Creation (backend + frontend)

Backend: add `CreateEntity` command, `char_to_byte_offset()`, `list_available_types` and `search_types` Tauri commands. Frontend: `EntityCreationPopup.svelte`, `Cmd+E` keybinding, `commands.ts` wrappers.

### Step 3: Stale Anchor Resolution (backend + frontend)

Backend: add `UpdateStaleAnchor` command. Frontend: `stale-anchor-widget.ts` CM extension, dismiss logic in document state.

### Step 4: Knowledge Panel (backend + frontend)

Backend: add `GetDocumentOverview`, `GetEntityDetail`, `DeleteEntity` commands. Frontend: `KnowledgePanel.svelte`, `EntityList.svelte`, `EntityDetail.svelte`, editor ↔ panel sync, layout update in `App.svelte`.

### Step 5: Integration and polish

Wire all features together. Verify the full flow: open workspace → open doc → create entity → edit text → resolve stale anchor → view in Knowledge Panel → save → export. Run all existing tests + new unit tests.

---

## Part 9: Implementation Notes

### Stale anchor widget focus management

CodeMirror widgets (`WidgetType.toDOM()`) do not natively participate in the document's focus model. The stale anchor widget must either:
- Return a DOM element with focusable buttons from `toDOM()`, managing focus explicitly
- Or use a CM `keymap` extension that checks cursor proximity to stale decorations and handles `y`/`n` keys contextually

The first approach (focusable DOM elements) is simpler and more accessible. The widget's `toDOM()` returns a `<div>` with `<button>` elements that handle click and keyboard events independently of CM's keymap.

### Deferred Phase 2 features

The Phase 1 Tauri spec's "Phase 2: Knowledge UI" roadmap includes features not covered by this spec. These are explicitly deferred to a future phase:

- **Mode transitions** (Deep Writing → Light Writing → Review → Full Reading) — deferred; requires timing-based UI state machine
- **`Cmd+K` inline property editor** on pinned whisper card — deferred; depends on property editing UX design
- **File modification detection on save** — deferred; requires file watcher or mtime checking
- **Suggestion ribbon** (accept/dismiss from tray) — deferred to Phase 4 (AI suggestion engine)

### search_types result limits

The `search_types` command accepts an optional `limit` parameter (default 50) to prevent oversized payloads when searching across large ontology packs like FIBO. The frontend can request more results via pagination if needed.
