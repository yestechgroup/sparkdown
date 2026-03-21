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
|  |         (tokio::sync::RwLock)               |      |
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

- **SessionRegistry** is the only shared state: a `tokio::sync::RwLock<HashMap<DocId, mpsc::Sender<SessionCommand>>>`. Uses `tokio::sync::RwLock` (not `std::sync`) because Tauri commands run on the tokio runtime and the lock may be held across `.await` points.
- Each **DocumentSession** is a standalone tokio task that owns all its Sparkdown data. No shared mutexes between documents. The session retains the current `source: String` internally so it can provide `old_source` to `sync_graph` on each update.
- The frontend communicates via `invoke()` (request/response) for commands and `listen()` (subscription) for backend-pushed events.
- The backend pre-resolves CURIEs, extracts display labels, and converts byte offsets to UTF-16 character offsets before serializing. The Svelte side never needs prefix resolution or offset conversion logic.

---

## Part 2: DocumentSession Actor

### Core types

```rust
/// Identifies an open document. Canonical absolute path, normalized.
type DocId = String; // Absolute path to the .md file, e.g. "/home/user/notes/meeting.md"

/// Output format for export commands.
enum RenderFormat {
    HtmlRdfa,
    JsonLd,
    Turtle,
}
```

### Session lifecycle

```
open_document(path)
  |
  +-- Read .md file from disk
  +-- Read .sparkdown-sem sidecar (if exists)
  +-- Parse source via SparkdownParser::new().parse(&source)
  |     -> Result<SparkdownDocument, SparkdownError>
  |     On parse error: emit "parse-error", continue with empty AST
  +-- Parse sidecar via sidecar::parse(&input)
  |     -> Result<SemanticGraph, OverlayError>
  |     On parse error: emit "sidecar-error", continue with empty graph
  |     On missing sidecar: start with empty graph (normal case)
  +-- Build MappingIndex::build(&graph) -> MappingIndex
  +-- Spawn tokio task with mpsc::Receiver<SessionCommand>
  |     Session stores: source, document, graph, index, parser, file_path
  +-- Register sender in SessionRegistry
  +-- Emit "document-opened" event with initial state
```

### SessionCommand enum

```rust
enum SessionCommand {
    // Phase 1
    UpdateSource { new_source: String, reply: oneshot::Sender<()> },
    GetEntitiesAt { start: usize, end: usize, reply: oneshot::Sender<Vec<EntityDto>> },
    ExportAs { format: RenderFormat, reply: oneshot::Sender<Result<String, String>> },
    Save { reply: oneshot::Sender<Result<(), String>> },
    Close,

    // Phase 2 (extension points)
    CreateEntity { span_start: usize, span_end: usize, type_iri: String, reply: oneshot::Sender<EntityDto> },
    AcceptSuggestion { entity_id: String },
    DismissSuggestion { entity_id: String },
    UpdateStaleAnchor { entity_id: String, reply: oneshot::Sender<()> },
}
```

### Workspace commands (outside session system)

```rust
// These are standalone #[tauri::command] functions, not routed through sessions.

/// Opens a Tauri dialog to pick a folder, scans for .md files,
/// returns FileEntry[] with has_sidecar flags.
#[tauri::command]
async fn open_workspace() -> Result<WorkspaceInfo, String>;

/// Re-scans the current workspace directory for .md files.
#[tauri::command]
async fn list_workspace_files(path: String) -> Result<Vec<FileEntry>, String>;

struct WorkspaceInfo {
    path: String,
    files: Vec<FileEntry>,
}
```

File scanning is a flat recursive directory walk filtering for `*.md` files. For each `.md` file, check if a corresponding `.sparkdown-sem` sidecar exists. No file watching in Phase 1 — the user can manually refresh via a UI button.

### Events emitted to frontend

| Event | Payload | When |
|-------|---------|------|
| `document-opened` | Full entity list + sidecar status | Document first loaded |
| `entities-updated` | Full entity list with spans, types, statuses | After source update + sync |
| `sidecar-status` | `{ synced, stale, detached, total_triples }` | After any graph change |
| `stale-anchors` | List of stale entities with old/new snippet text | After sync detects staleness |
| `document-saved` | Confirmation | After save completes |
| `parse-error` | Error details | If source has parse issues |
| `sidecar-error` | Error details | If sidecar file is corrupt or malformed |

### Editing hot path

1. User types in CodeMirror.
2. Frontend debounces keystrokes (150ms).
3. Sends `UpdateSource { new_source }` via `invoke()`.
4. Session runs `SparkdownParser::new().parse(&new_source)` -> `Result<SparkdownDocument, SparkdownError>`. On error, emits `parse-error` and skips steps 5-7.
5. Session runs `sync::sync_graph(&mut graph, &old_source, &new_source)` to adjust anchors. This internally calls `Anchor::verify_snippet()` and marks entities as stale/detached — no separate staleness check needed.
6. Session updates `self.source = new_source` and rebuilds `MappingIndex::build(&graph)`.
7. Session builds `Vec<EntityDto>` by:
   - Iterating `graph.entities` for each entity's id, anchor span, types, and status
   - For each entity, calling `graph.triples_for_subject(id)` to find top 1-2 relations
   - Resolving predicate IRIs to display labels via `ThemeRegistry::lookup_property()` or namespace stripping
   - Resolving target entity labels from their anchor snippets
   - Converting byte offsets to character offsets (see Position Mapping below)
   - Pre-resolving type CURIEs via `PrefixMap::resolve()` (handling `Result<NamedNode, SparkdownError>` — unknown prefixes fall back to raw IRI)
8. Session emits `entities-updated` and `sidecar-status` events.
9. Frontend stores update reactively; CM extensions redecorate.

The debounce ensures we don't reparse on every keystroke. 150ms feels instant but keeps CPU reasonable.

### Position mapping: byte offsets to character offsets

Sparkdown's `Anchor` and `SemanticNode` spans use byte offsets. CodeMirror uses character positions (UTF-16 code units). For ASCII-only content these are identical, but multi-byte characters (accented text, emoji, CJK) cause divergence.

**Strategy:** The backend converts byte offsets to character offsets before emitting `EntityDto`. The `DocumentSession` maintains a byte-to-char mapping table, rebuilt from the source string on each `UpdateSource`. This is a single linear scan of the source, O(n) in document length, amortized into the reparse step.

```rust
/// Maps byte offset to UTF-16 character offset.
fn byte_to_char_offset(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].encode_utf16().count()
}
```

The `EntityDto` fields `span_start` and `span_end` are character offsets ready for CodeMirror consumption.

### Sparkdown API usage per operation

| Operation | Sparkdown API | Returns | Crate |
|-----------|---------------|---------|-------|
| Parse source | `SparkdownParser::new().parse(&source)` | `Result<SparkdownDocument, SparkdownError>` | sparkdown-core |
| Load sidecar | `sidecar::parse(&input)` | `Result<SemanticGraph, OverlayError>` | sparkdown-overlay |
| Save sidecar | `sidecar::serialize(&graph)` | `String` | sparkdown-overlay |
| Build index | `MappingIndex::build(&graph)` | `MappingIndex` | sparkdown-overlay |
| Entity lookup | `MappingIndex::entities_at(range)` | `Vec<BlankNode>` (resolve against graph for DTOs) | sparkdown-overlay |
| Sync after edit | `sync::sync_graph(&mut graph, &old_source, &new_source)` | `()` (mutates graph in place) | sparkdown-overlay |
| Type validation | `ThemeRegistry::lookup_type(prefix, local)` | `Option<&TypeDef>` | sparkdown-ontology |
| Property lookup | `ThemeRegistry::lookup_property(prefix, local)` | `Option<&PropertyDef>` | sparkdown-ontology |
| Export HTML | `HtmlRdfaRenderer::new().render(&doc, &mut out)` | `Result<(), RenderError>` | sparkdown-render |
| Export JSON-LD | `JsonLdRenderer::new().render(&doc, &mut out)` | `Result<(), RenderError>` | sparkdown-render |
| Export Turtle | `TurtleRenderer::new().render(&doc, &mut out)` | `Result<(), RenderError>` | sparkdown-render |
| Prefix resolution | `PrefixMap::resolve(curie)` | `Result<NamedNode, SparkdownError>` | sparkdown-core |

### Error handling

| Scenario | Behavior |
|----------|----------|
| Source parse failure | Emit `parse-error`, keep previous AST/graph state, mark document as having errors in suggestion tray |
| Sidecar parse failure | Emit `sidecar-error`, open with empty graph, show warning in suggestion tray |
| Sidecar file missing | Normal case — start with empty graph, no error |
| Unknown prefix in CURIE resolution | Fall back to raw IRI string for display |
| Save fails (permissions, disk full) | Return `Err` from Save command, frontend shows error notification |
| File externally deleted while open | Save command returns error; no file watching in Phase 1 |
| File externally modified | Not detected in Phase 1 (no file watching); save overwrites. Phase 2 may add modification checking. |

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

### Svelte 5 state (runes)

```typescript
// workspace.svelte.ts — workspace-level state
let workspacePath = $state<string | null>(null);
let fileList = $state<FileEntry[]>([]);
let activeDocId = $state<string | null>(null);

// document.svelte.ts — per-document state, updated from backend events
let entities = $state<EntityDto[]>([]);
let sidecarStatus = $state<SidecarStatus>({ synced: 0, stale: 0, detached: 0, total_triples: 0 });
let staleAnchors = $state<StaleAnchor[]>([]);
```

State is managed via Svelte 5 runes (`$state`, `$derived`, `$effect`), not the Svelte 4 `Writable<T>` store API.

### CodeMirror extensions

**Semantic Gutter** — A CM `gutter` extension. Reads from the `entities` rune. For each line, checks if any entity span (character offsets, ready for CM) overlaps that line's range. Renders colored dots (4px circles) using the entity type color map.

**Entity Decorations** — A CM `Decoration.mark` extension. Creates inline underline decorations from entity character spans. Solid 1px underline at 20% opacity for confirmed entities; dotted for suggestions (suggestion source is Phase 4, but decoration type is ready). Uses CM `ViewPlugin` to efficiently update decorations when entities change without re-rendering the whole document.

**Whisper Card** — A CM `hoverTooltip` extension. On 300ms hover, looks up entities at cursor position from the local `entities` state (no IPC round-trip). Renders a lightweight tooltip showing entity name, type, and top 1-2 relations. The "Open" navigation link is hidden in Phase 1 (Knowledge Panel is Phase 2).

### Suggestion Tray (Phase 1 scope)

In Phase 1, the tray shows three items only:

```
  3 entities  ·  sidecar: synced  ·  12 triples
```

The "suggestions" count is omitted until Phase 4 (AI suggestion engine). The tray component accepts an optional suggestion count prop so Phase 4 can enable it without restructuring.

### Data flow: editing hot path

```
User types -> CodeMirror transaction
  -> debounce (150ms)
  -> invoke("update_source", { docId, newSource })
  -> Tauri routes to DocumentSession via SessionRegistry
  -> Session: reparse -> sync -> rebuild index -> build DTOs
  -> emit("entities-updated", { docId, entities })
  -> Svelte state updates via event listener
  -> CM extensions reactively redecorate
```

The frontend never does entity lookups via IPC during hover. The `entities` state already contains the full entity list with character offsets. The CM hover extension filters locally.

---

## Part 4: IPC Type Contract

### types.rs <-> commands.ts boundary

```typescript
interface EntityDto {
  id: string                // blank node id, e.g. "_:e1"
  label: string             // display name (from anchor snippet)
  type_iris: string[]       // full IRIs, e.g. ["http://schema.org/Person"]
  type_prefix: string       // pre-resolved CURIE, e.g. "schema:Person" (first type)
  span_start: number        // character offset (UTF-16), ready for CodeMirror
  span_end: number          // character offset (UTF-16), ready for CodeMirror
  status: "synced" | "stale" | "detached"
  top_relations: Relation[] // max 2, for whisper card
}

interface Relation {
  predicate_label: string   // e.g. "performerIn" (local name or ThemeRegistry label)
  target_label: string      // e.g. "RustConf" (from target entity's snippet)
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
  span_start: number        // character offset
  span_end: number          // character offset
}

interface FileEntry {
  name: string
  path: string              // absolute path
  has_sidecar: boolean
}

interface WorkspaceInfo {
  path: string
  files: FileEntry[]
}
```

**DTO construction:** The backend builds `EntityDto` by resolving `BlankNode` IDs from `MappingIndex::entities_at()` against the `SemanticGraph`, calling `graph.triples_for_subject(id)` for relations, resolving predicate/type IRIs via `ThemeRegistry` or namespace stripping, and converting byte offsets to character offsets. The frontend receives display-ready data.

---

## Part 5: File Structure

```
studio/
+-- src-tauri/
|   +-- Cargo.toml              <- workspace member, depends on sparkdown-* crates
|   +-- tauri.conf.json
|   +-- build.rs                <- tauri_build::build()
|   +-- capabilities/
|   |   +-- default.json        <- Tauri 2 capability permissions
|   +-- src/
|   |   +-- main.rs             <- thin wrapper calling lib::run()
|   |   +-- lib.rs              <- Tauri setup, plugin init, command registration
|   |   +-- session.rs          <- DocumentSession actor + SessionCommand enum
|   |   +-- registry.rs         <- SessionRegistry (tokio::sync::RwLock<HashMap>)
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
|   |   |   +-- workspace.svelte.ts   <- workspace + file list state (runes)
|   |   |   +-- document.svelte.ts    <- active doc entities/status (runes)
|   |   |   +-- events.ts            <- Tauri event listeners -> state updates
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
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[build-dependencies]
tauri-build = { version = "2" }
```

### Tauri capabilities (capabilities/default.json)

```json
{
  "identifier": "default",
  "description": "Default capabilities for Sparkdown Studio",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "fs:default",
    "fs:allow-read",
    "fs:allow-write"
  ]
}
```

Required permissions: `fs:read` and `fs:write` for document/sidecar I/O, `dialog:open` for workspace folder picker. Additional permissions (e.g., `shell:open` for external links) can be added as needed.

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
- Workspace commands: open folder dialog, list files, refresh
- DocumentSession actor system with event-driven IPC
- Semantic gutter extension (colored entity dots)
- Entity decoration extension (inline underlines by status)
- Whisper card tooltip (entity name, type, top relations on hover; "Open" link hidden until Phase 2)
- Suggestion tray status bar (entity count, sidecar status, triple count; suggestion count hidden until Phase 4)
- Save command (write source + serialize sidecar to disk)
- Export commands (HTML+RDFa, JSON-LD, Turtle via invoke)
- Error handling: graceful degradation on parse/sidecar errors
- Dark theme with entity type color system

### Phase 2: Knowledge UI

- Quick entity creation (`Cmd+E` selection popup with type picker)
- Stale anchor nudge decorations (inline y/n to update snippet)
- Suggestion ribbon (accept/dismiss from tray)
- Knowledge Panel (right panel, context-sensitive entity details)
- Whisper card "Open" link enabled (navigates to Knowledge Panel)
- Mode transitions (deep writing -> light writing -> review -> full reading)
- `Cmd+K` inline property editor on pinned whisper card
- File modification detection on save (warn if externally changed)

### Phase 3: Constellation

- Cross-document index actor (scans all sidecars on workspace open)
- Constellation Bar: force-directed graph visualization
- Constellation Bar: timeline view for temporal entities
- Constellation Bar: connections web (cross-doc shared entities)
- Semantic search (`Cmd+P` with `type:`, `related:`, `stale:` prefixes)
- "Across all docs" section in Knowledge Panel
- File system watcher for workspace changes

### Phase 4: Intelligence

- AI suggestion engine (NLP-based entity detection)
- Discovery feed (home screen insights and enrichment suggestions)
- Cross-document entity unification (auto-linking known entities)
- Entity density heatmap in reading mode
- Suggestion count enabled in suggestion tray

### Phase 5: Team

- Shared entity registry (network sync)
- Entity merging UI
- Knowledge graph dashboard

### Extension points in Phase 1 architecture

- `SessionCommand` enum grows with new variants for each phase; no restructuring needed.
- `SessionRegistry` pattern extends to a `WorkspaceIndex` actor in Phase 3.
- Event payloads use serde; frontend ignores unknown fields via forward compatibility.
- CM extension system is modular; Phase 2 features are additional extensions.
- Suggestion tray accepts optional props for features enabled in later phases.
