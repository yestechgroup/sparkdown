# Sparkdown Studio: Tauri Implementation Design

> Implementation architecture for Sparkdown Studio as a Tauri 2 desktop application.
> Companion to: `2026-03-21-sparkdown-studio-ui-design.md` (UI spec) and `2026-03-21-sparkdown-studio-planning-notes.md` (planning notes).

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Frontend framework | Svelte 5 | Compiler-based, small runtime, runes reactivity model fits fine-grained CM updates |
| Editor | CodeMirror 6 | Best extension API for custom decorations (gutters, inline marks, tooltips) |
| App location | `studio/` subfolder in existing workspace | `studio/src-tauri/` as workspace member, path deps on sparkdown crates |
| IPC model | Event-driven hybrid | Session state on backend + pushed events to frontend; request/response for commands |
| Backend architecture | Per-document actors | Each open doc is a tokio task with owned state; no shared mutexes |
| Workspace model (Phase 1) | File tree, no cross-doc index | Establishes workspace concept early; global index deferred to Phase 3 |

---

## Part 1: System Architecture

```
+-----------------------------------------------------+
|                   Svelte 5 Frontend                  |
|  +----------+  +----------+  +-------------------+  |
|  | File Tree |  |CodeMirror|  | Knowledge Panel   |  |
|  | Sidebar   |  | Editor   |  | (Phase 2)         |  |
|  +----------+  +----------+  +-------------------+  |
|         |            |               |               |
|         +------------+---------------+               |
|                      | invoke() + listen()           |
+----------------------+-------------------------------+
|              Tauri IPC Bridge                        |
+----------------------+-------------------------------+
|              Tauri Backend (sparkdown-studio)         |
|                      |                               |
|  +-------------------+------------------------+      |
|  |          SessionRegistry                    |      |
|  |   HashMap<DocId, Sender<SessionCommand>>    |      |
|  |              (RwLock)                       |      |
|  +-----+---------------+--------------+-------+      |
|        |               |              |              |
|  +-----+------+  +-----+------+  +----+-------+     |
|  | DocSession  |  | DocSession  |  | DocSession  |     |
|  | (tokio task) |  | (tokio task) |  | (tokio task) |     |
|  |             |  |             |  |             |     |
|  | - source    |  | - source    |  | - source    |     |
|  | - Document  |  | - Document  |  | - Document  |     |
|  | - Graph     |  | - Graph     |  | - Graph     |     |
|  | - MapIndex  |  | - MapIndex  |  | - MapIndex  |     |
|  | - Parser    |  | - Parser    |  | - Parser    |     |
|  +-------------+  +-------------+  +-------------+     |
|                                                      |
|  Uses: sparkdown-core, sparkdown-overlay,            |
|        sparkdown-render, sparkdown-ontology           |
+------------------------------------------------------+
```

### Key architectural properties

- **SessionRegistry** is the only shared state: a `RwLock<HashMap<DocId, mpsc::Sender<SessionCommand>>>`.
- Each **DocumentSession** is a standalone tokio task that owns all its Sparkdown data. No shared mutexes between documents.
- The frontend communicates via `invoke()` (request/response) for commands and `listen()` (subscription) for backend-pushed events.
- The backend pre-resolves CURIEs, extracts display labels, and shapes data for the frontend before serializing. The Svelte side never needs prefix resolution logic.

---

## Part 2: DocumentSession Actor

### Session lifecycle

```
open_document(path)
  |
  +-- Read .md file from disk
  +-- Read .sparkdown-sem sidecar (if exists)
  +-- Parse source -> SparkdownDocument  (sparkdown-core)
  +-- Parse sidecar -> SemanticGraph     (sparkdown-overlay)
  +-- Build MappingIndex from graph      (sparkdown-overlay)
  +-- Spawn tokio task with mpsc::Receiver<SessionCommand>
  +-- Register sender in SessionRegistry
  +-- Emit "document-opened" event with initial state
```

### SessionCommand enum

```rust
enum SessionCommand {
    // Phase 1
    UpdateSource { new_source: String, reply: oneshot::Sender<()> },
    GetEntitiesAt { start: usize, end: usize, reply: oneshot::Sender<Vec<EntityDto>> },
    ExportAs { format: RenderFormat, reply: oneshot::Sender<String> },
    Save { reply: oneshot::Sender<Result<(), String>> },
    Close,

    // Phase 2 (extension points)
    CreateEntity { span_start: usize, span_end: usize, type_iri: String, reply: oneshot::Sender<EntityDto> },
    AcceptSuggestion { entity_id: String },
    DismissSuggestion { entity_id: String },
    UpdateStaleAnchor { entity_id: String, reply: oneshot::Sender<()> },
}
```

### Events emitted to frontend

| Event | Payload | When |
|-------|---------|------|
| `document-opened` | Full entity list + sidecar status | Document first loaded |
| `entities-updated` | Full entity list with spans, types, statuses | After source update + sync |
| `sidecar-status` | `{ synced, stale, detached, total_triples }` | After any graph change |
| `stale-anchors` | List of stale entities with old/new snippet text | After sync detects staleness |
| `document-saved` | Confirmation | After save completes |
| `parse-error` | Error details | If source has parse issues |

### Editing hot path

1. User types in CodeMirror.
2. Frontend debounces keystrokes (150ms).
3. Sends `UpdateSource { new_source }` via `invoke()`.
4. Session runs `SparkdownParser::parse()` on new source.
5. Session runs `sync_graph(graph, old_source, new_source)` to adjust anchors.
6. Session rebuilds `MappingIndex` from updated graph.
7. Session emits `entities-updated` and `sidecar-status` events.
8. Frontend stores update reactively; CM extensions redecorate.

The debounce ensures we don't reparse on every keystroke. 150ms feels instant but keeps CPU reasonable.

### Sparkdown API usage per operation

| Operation | Sparkdown API | Crate |
|-----------|---------------|-------|
| Parse source | `SparkdownParser::new().parse(source)` | sparkdown-core |
| Load sidecar | `sidecar::parse(input)` | sparkdown-overlay |
| Save sidecar | `sidecar::serialize(graph)` | sparkdown-overlay |
| Build index | `MappingIndex::build(graph)` | sparkdown-overlay |
| Entity lookup | `MappingIndex::entities_at(range)` | sparkdown-overlay |
| Sync after edit | `sync::sync_graph(graph, old, new)` | sparkdown-overlay |
| Staleness check | `Anchor::verify_snippet(source)` | sparkdown-overlay |
| Type validation | `ThemeRegistry::lookup_type(prefix, local)` | sparkdown-ontology |
| Export HTML | `HtmlRdfaRenderer::new().render(doc, &mut out)` | sparkdown-render |
| Export JSON-LD | `JsonLdRenderer::new().render(doc, &mut out)` | sparkdown-render |
| Export Turtle | `TurtleRenderer::new().render(doc, &mut out)` | sparkdown-render |
| Prefix resolution | `PrefixMap::resolve(curie)` | sparkdown-core |

---

## Part 3: Svelte Frontend

### Component tree (Phase 1)

```
App.svelte
+-- Sidebar.svelte              <- file tree + workspace controls
|   +-- WorkspaceHeader.svelte  <- workspace name, open folder button
|   +-- FileTree.svelte         <- recursive .md file listing
|
+-- EditorPane.svelte           <- main editing area
|   +-- CodeMirrorEditor.svelte <- CodeMirror 6 instance
|   |   +-- semanticGutter      <- CM extension: colored dots
|   |   +-- entityDecorations   <- CM extension: underlines
|   |   +-- whisperCard         <- CM tooltip: entity hover card
|   +-- SuggestionTray.svelte   <- bottom status bar
|
+-- (KnowledgePanel.svelte)     <- Phase 2, placeholder slot only
```

### Svelte stores

```typescript
// Workspace-level state
workspacePath: Writable<string | null>
fileList: Writable<FileEntry[]>
activeDocId: Writable<string | null>

// Per-document state (updated from backend events)
entities: Writable<EntityDto[]>
sidecarStatus: Writable<SidecarStatus>
staleAnchors: Writable<StaleAnchor[]>
```

### CodeMirror extensions

**Semantic Gutter** — A CM `gutter` extension. Subscribes to the `entities` store. For each line, checks if any entity span overlaps that line's byte range. Renders colored dots (4px circles) using the entity type color map.

**Entity Decorations** — A CM `Decoration.mark` extension. Creates inline underline decorations from entity spans. Solid 1px underline at 20% opacity for confirmed entities; dotted for suggestions. Uses CM `ViewPlugin` to efficiently update decorations when the entities store changes without re-rendering the whole document.

**Whisper Card** — A CM `hoverTooltip` extension. On 300ms hover, looks up entities at cursor position from the local `entities` store (no IPC round-trip). Renders a lightweight tooltip showing entity name, type, and top 1-2 relations.

### Data flow: editing hot path

```
User types -> CodeMirror transaction
  -> debounce (150ms)
  -> invoke("update_source", { docId, newSource })
  -> Tauri routes to DocumentSession via SessionRegistry
  -> Session: reparse -> sync -> rebuild index
  -> emit("entities-updated", { docId, entities })
  -> Svelte store updates
  -> CM extensions reactively redecorate
```

The frontend never does entity lookups via IPC during hover. The `entities` store already contains the full entity list with byte spans. The CM hover extension filters locally.

---

## Part 4: IPC Type Contract

### types.rs <-> commands.ts boundary

```typescript
interface EntityDto {
  id: string                // blank node id, e.g. "_:e1"
  label: string             // display name (from snippet)
  type_iris: string[]       // full IRIs, e.g. ["http://schema.org/Person"]
  type_prefix: string       // pre-resolved CURIE, e.g. "schema:Person"
  span_start: number        // byte offset in source
  span_end: number          // byte offset in source
  status: "synced" | "stale" | "detached"
  top_relations: Relation[] // max 2, for whisper card
}

interface Relation {
  predicate_label: string   // e.g. "performerIn"
  target_label: string      // e.g. "RustConf"
  target_id: string
}

interface SidecarStatus {
  synced: number
  stale: number
  detached: number
  total_triples: number
}

interface StaleAnchor {
  entity_id: string
  old_snippet: string
  new_text: string
  span_start: number
  span_end: number
}

interface FileEntry {
  name: string
  path: string
  has_sidecar: boolean
}
```

The backend pre-resolves CURIEs and extracts display labels before serialization. The frontend receives display-ready data.

---

## Part 5: File Structure

```
studio/
+-- src-tauri/
|   +-- Cargo.toml              <- workspace member, depends on sparkdown-* crates
|   +-- tauri.conf.json
|   +-- src/
|   |   +-- main.rs             <- thin wrapper calling lib::run()
|   |   +-- lib.rs              <- Tauri setup, plugin init, command registration
|   |   +-- session.rs          <- DocumentSession actor + SessionCommand enum
|   |   +-- registry.rs         <- SessionRegistry (RwLock<HashMap>)
|   |   +-- commands.rs         <- #[tauri::command] functions (thin routing layer)
|   |   +-- events.rs           <- Event payload types (Serialize)
|   |   +-- types.rs            <- Shared DTOs serialized across IPC boundary
|   +-- icons/
|
+-- src/
|   +-- app.html
|   +-- routes/
|   |   +-- +page.svelte        <- main app shell
|   +-- lib/
|   |   +-- components/
|   |   |   +-- Sidebar.svelte
|   |   |   +-- FileTree.svelte
|   |   |   +-- EditorPane.svelte
|   |   |   +-- CodeMirrorEditor.svelte
|   |   |   +-- SuggestionTray.svelte
|   |   |   +-- WhisperCard.svelte
|   |   +-- editor/
|   |   |   +-- semantic-gutter.ts    <- CM gutter extension
|   |   |   +-- entity-decorations.ts <- CM decoration extension
|   |   |   +-- whisper-tooltip.ts    <- CM hover tooltip extension
|   |   +-- stores/
|   |   |   +-- workspace.ts    <- workspace + file list state
|   |   |   +-- document.ts     <- active doc entities/status
|   |   |   +-- events.ts       <- Tauri event listeners -> store updates
|   |   +-- tauri/
|   |   |   +-- commands.ts     <- typed invoke() wrappers
|   |   +-- theme/
|   |       +-- colors.ts       <- entity type -> color mapping
|   |       +-- tokens.css      <- design tokens from the UI spec
|   +-- svelte.config.js
|   +-- vite.config.ts
|
+-- package.json
+-- pnpm-lock.yaml
```

### Cargo.toml workspace integration

The root `Cargo.toml` adds `studio/src-tauri` as a workspace member:

```toml
[workspace]
members = [
    "crates/sparkdown-core",
    "crates/sparkdown-ontology",
    "crates/sparkdown-render",
    "crates/sparkdown-overlay",
    "crates/sparkdown-cli",
    "studio/src-tauri",
]
```

The studio crate's `Cargo.toml` uses path dependencies:

```toml
[dependencies]
sparkdown-core = { path = "../../crates/sparkdown-core" }
sparkdown-overlay = { path = "../../crates/sparkdown-overlay" }
sparkdown-render = { path = "../../crates/sparkdown-render" }
sparkdown-ontology = { path = "../../crates/sparkdown-ontology" }
tauri = { version = "2", features = ["devtools"] }
tauri-build = { version = "2" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

---

## Part 6: Visual Language (from UI spec)

### Entity type color map

| Type | Color | Hex |
|------|-------|-----|
| Person | Warm Amber | `#F59E0B` |
| Place | Teal | `#14B8A6` |
| Event | Violet | `#8B5CF6` |
| Organization | Slate Blue | `#6366F1` |
| Article/Document | Sage Green | `#22C55E` |
| Review/Rhetorical | Rose | `#F43F5E` |
| Custom | Gray | `#6B7280` |

### Typography

- Editor: JetBrains Mono or Berkeley Mono, 14px
- UI chrome: Inter, 13px
- Entity labels: Inter, 11px, medium weight

### Dark mode (default)

- Background: `#0F0F0F`
- Editor surface: `#1A1A1A`
- Entity highlights: type colors at 15% opacity for backgrounds, full saturation for dots/underlines

---

## Part 7: Phase Roadmap

### Phase 1: Core Shell (implementation target)

- Tauri 2 scaffold with Svelte 5 + Vite + SvelteKit
- CodeMirror 6 editor with markdown syntax highlighting
- File tree sidebar (workspace directory, `.md` files, sidecar indicators)
- DocumentSession actor system with event-driven IPC
- Semantic gutter extension (colored entity dots)
- Entity decoration extension (inline underlines, solid/dotted by status)
- Whisper card tooltip (entity name, type, top relations on hover)
- Suggestion tray status bar (entity count, sidecar status, triple count)
- Save command (write source + serialize sidecar to disk)
- Export commands (HTML+RDFa, JSON-LD, Turtle via invoke)
- Dark theme with entity type color system

### Phase 2: Knowledge UI

- Quick entity creation (`Cmd+E` selection popup with type picker)
- Stale anchor nudge decorations (inline y/n to update snippet)
- Suggestion ribbon (accept/dismiss from tray)
- Knowledge Panel (right panel, context-sensitive entity details)
- Mode transitions (deep writing -> light writing -> review -> full reading)
- `Cmd+K` inline property editor on pinned whisper card

### Phase 3: Constellation

- Cross-document index actor (scans all sidecars on workspace open)
- Constellation Bar: force-directed graph visualization
- Constellation Bar: timeline view for temporal entities
- Constellation Bar: connections web (cross-doc shared entities)
- Semantic search (`Cmd+P` with `type:`, `related:`, `stale:` prefixes)
- "Across all docs" section in Knowledge Panel

### Phase 4: Intelligence

- AI suggestion engine (NLP-based entity detection)
- Discovery feed (home screen insights and enrichment suggestions)
- Cross-document entity unification (auto-linking known entities)
- Entity density heatmap in reading mode

### Phase 5: Team

- Shared entity registry (network sync)
- Entity merging UI
- Knowledge graph dashboard

### Extension points in Phase 1 architecture

- `SessionCommand` enum grows with new variants for each phase; no restructuring needed.
- `SessionRegistry` pattern extends to a `WorkspaceIndex` actor in Phase 3.
- Event payloads use serde; frontend ignores unknown fields via forward compatibility.
- CM extension system is modular; Phase 2 features are additional extensions.
