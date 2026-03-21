# Sparkdown Studio Phase 1: Core Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Sparkdown Studio Tauri 2 desktop app with a CodeMirror 6 markdown editor that displays semantic entity overlays (gutter dots, inline underlines, hover tooltips) driven by Sparkdown's existing Rust engine.

**Architecture:** Per-document actor model. Each open document is a tokio task owning its parsed AST, semantic graph, and mapping index. The Svelte 5 frontend communicates via Tauri IPC (invoke for commands, listen for pushed events). Entity data flows from Rust to frontend as pre-resolved DTOs with character offsets ready for CodeMirror.

**Tech Stack:** Rust 2024, Tauri 2, Svelte 5 (runes), SvelteKit, CodeMirror 6, Vite, pnpm

**Spec:** `docs/superpowers/specs/2026-03-21-sparkdown-studio-tauri-design.md`

---

## File Map

### Tauri Backend (`studio/src-tauri/`)

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Crate manifest with path deps on sparkdown-core, sparkdown-overlay, sparkdown-render, sparkdown-ontology |
| `build.rs` | `tauri_build::build()` |
| `tauri.conf.json` | Window config, frontend dev/build commands |
| `capabilities/default.json` | Tauri 2 permission grants (fs, dialog, core) |
| `src/main.rs` | Thin entry point calling `lib::run()` |
| `src/lib.rs` | Tauri app setup, plugin init, command registration |
| `src/types.rs` | `DocId`, `RenderFormat`, `EntityDto`, `Relation`, `SidecarStatus`, `StaleAnchor`, `FileEntry`, `WorkspaceInfo` — all with Serialize/Deserialize |
| `src/registry.rs` | `SessionRegistry` — `tokio::sync::RwLock<HashMap<DocId, mpsc::Sender<SessionCommand>>>` |
| `src/session.rs` | `DocumentSession` actor — tokio task owning source, graph, index; processes `SessionCommand`s; emits events |
| `src/commands.rs` | `#[tauri::command]` functions — thin routing layer that sends commands to sessions via registry |
| `src/events.rs` | Event emission helpers using `AppHandle::emit()` |

### Svelte Frontend (`studio/src/`)

| File | Responsibility |
|------|---------------|
| `app.html` | SvelteKit HTML shell |
| `routes/+page.svelte` | Main app layout — sidebar + editor pane |
| `lib/tauri/commands.ts` | Typed `invoke()` wrappers for all Tauri commands |
| `lib/stores/workspace.svelte.ts` | `$state` runes for workspace path, file list, active doc |
| `lib/stores/document.svelte.ts` | `$state` runes for entities, sidecar status, stale anchors |
| `lib/stores/events.ts` | Tauri event listeners that update document/workspace stores |
| `lib/theme/colors.ts` | Entity type IRI to hex color mapping |
| `lib/theme/tokens.css` | CSS custom properties for dark theme |
| `lib/components/Sidebar.svelte` | Left panel: workspace header + file tree |
| `lib/components/FileTree.svelte` | Recursive `.md` file listing with sidecar indicators |
| `lib/components/EditorPane.svelte` | Main editor area: CodeMirror + suggestion tray |
| `lib/components/CodeMirrorEditor.svelte` | CodeMirror 6 instance with custom extensions |
| `lib/components/SuggestionTray.svelte` | Bottom status bar (entity count, sidecar status, triple count) |
| `lib/components/WhisperCard.svelte` | Entity hover tooltip content |
| `lib/editor/semantic-gutter.ts` | CM gutter extension — colored dots per entity |
| `lib/editor/entity-decorations.ts` | CM decoration extension — inline underlines |
| `lib/editor/whisper-tooltip.ts` | CM hoverTooltip extension — entity whisper card |

---

## Task 1: Tauri + Svelte Scaffold

**Files:**
- Create: `studio/src-tauri/Cargo.toml`
- Create: `studio/src-tauri/build.rs`
- Create: `studio/src-tauri/tauri.conf.json`
- Create: `studio/src-tauri/capabilities/default.json`
- Create: `studio/src-tauri/src/main.rs`
- Create: `studio/src-tauri/src/lib.rs`
- Create: `studio/package.json`
- Create: `studio/src/app.html`
- Create: `studio/src/routes/+page.svelte`
- Create: `studio/svelte.config.js`
- Create: `studio/vite.config.ts`
- Create: `studio/tsconfig.json`
- Modify: `Cargo.toml` (root workspace)

- [ ] **Step 1: Create Svelte 5 + SvelteKit frontend scaffold**

Initialize the studio frontend. Create `studio/package.json`:

```json
{
  "name": "sparkdown-studio",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "@sveltejs/adapter-static": "^3",
    "@sveltejs/kit": "^2",
    "@sveltejs/vite-plugin-svelte": "^5",
    "svelte": "^5",
    "typescript": "^5",
    "vite": "^6"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "@tauri-apps/plugin-fs": "^2"
  }
}
```

Create `studio/svelte.config.js`:

```javascript
import adapter from '@sveltejs/adapter-static';

export default {
  kit: {
    adapter: adapter({
      fallback: 'index.html'
    })
  }
};
```

Create `studio/vite.config.ts`:

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
});
```

Create `studio/tsconfig.json`:

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "strict": true,
    "moduleResolution": "bundler"
  }
}
```

Create `studio/src/app.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Sparkdown Studio</title>
    %sveltekit.head%
  </head>
  <body data-sveltekit-prerender="off">
    %sveltekit.body%
  </body>
</html>
```

Create `studio/src/routes/+page.svelte`:

```svelte
<h1>Sparkdown Studio</h1>
<p>Shell is working.</p>
```

- [ ] **Step 2: Create Tauri backend scaffold**

Create `studio/src-tauri/build.rs`:

```rust
fn main() {
    tauri_build::build();
}
```

Create `studio/src-tauri/Cargo.toml`:

```toml
[package]
name = "sparkdown-studio"
version = "0.1.0"
edition = "2024"

[lib]
name = "sparkdown_studio_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
sparkdown-core = { path = "../../crates/sparkdown-core" }
sparkdown-overlay = { path = "../../crates/sparkdown-overlay" }
sparkdown-render = { path = "../../crates/sparkdown-render" }
sparkdown-ontology = { path = "../../crates/sparkdown-ontology" }
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
sha2 = "0.10"

[build-dependencies]
tauri-build = { version = "2" }
```

Create `studio/src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    sparkdown_studio_lib::run();
}
```

Create `studio/src-tauri/src/lib.rs`:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .run(tauri::generate_context!())
        .expect("error while running Sparkdown Studio");
}
```

Create `studio/src-tauri/tauri.conf.json`:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Sparkdown Studio",
  "version": "0.1.0",
  "identifier": "com.yestech.sparkdown-studio",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../build"
  },
  "app": {
    "windows": [
      {
        "title": "Sparkdown Studio",
        "width": 1200,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600,
        "label": "main"
      }
    ]
  }
}
```

Create `studio/src-tauri/capabilities/default.json`:

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

- [ ] **Step 3: Add studio to workspace**

Modify root `Cargo.toml` — add `"studio/src-tauri"` to workspace members:

```toml
[workspace]
resolver = "2"
members = [
    "crates/sparkdown-core",
    "crates/sparkdown-ontology",
    "crates/sparkdown-render",
    "crates/sparkdown-overlay",
    "crates/sparkdown-cli",
    "studio/src-tauri",
]
```

- [ ] **Step 4: Install frontend dependencies, generate icons, and verify build**

Run from `studio/`:

```bash
cd studio && pnpm install
```

Generate default Tauri app icons (creates `studio/src-tauri/icons/`):

```bash
cd studio && pnpm tauri icon --output src-tauri/icons
```

If `pnpm tauri icon` is not available, create the `studio/src-tauri/icons/` directory manually and add a placeholder `icon.png` (32x32 minimum). Tauri requires icons at build time.

Run from project root:

```bash
cargo build -p sparkdown-studio
```

Expected: Both succeed. The Tauri crate compiles with path deps to all sparkdown crates.

- [ ] **Step 5: Commit scaffold**

```bash
git add studio/ Cargo.toml
git commit -m "feat: scaffold Sparkdown Studio Tauri 2 + Svelte 5 app"
```

---

## Task 2: IPC Types and Registry

**Files:**
- Create: `studio/src-tauri/src/types.rs`
- Create: `studio/src-tauri/src/registry.rs`
- Create: `studio/src-tauri/src/events.rs`
- Modify: `studio/src-tauri/src/lib.rs`
- Test: `studio/src-tauri/src/types.rs` (inline tests)
- Test: `studio/src-tauri/src/registry.rs` (inline tests)

- [ ] **Step 1: Write failing test for types serialization**

Create `studio/src-tauri/src/types.rs` with types and a test:

```rust
use serde::{Deserialize, Serialize};

/// Identifies an open document by absolute path.
pub type DocId = String;

/// Output format for export commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderFormat {
    HtmlRdfa,
    JsonLd,
    Turtle,
}

/// Entity data transfer object — pre-resolved for frontend consumption.
/// Spans are character offsets (UTF-16), ready for CodeMirror.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDto {
    pub id: String,
    pub label: String,
    pub type_iris: Vec<String>,
    pub type_prefix: String,
    pub span_start: usize,
    pub span_end: usize,
    pub status: EntityStatus,
    pub top_relations: Vec<Relation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityStatus {
    Synced,
    Stale,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub predicate_label: String,
    pub target_label: String,
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarStatus {
    pub synced: usize,
    pub stale: usize,
    pub detached: usize,
    pub total_triples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleAnchor {
    pub entity_id: String,
    pub old_snippet: String,
    pub new_text: String,
    pub span_start: usize,
    pub span_end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub has_sidecar: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub path: String,
    pub files: Vec<FileEntry>,
}

/// Maps byte offset to UTF-16 character offset for CodeMirror compatibility.
pub fn byte_to_char_offset(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_dto_serializes_to_json() {
        let entity = EntityDto {
            id: "_:e1".into(),
            label: "Niko Matsakis".into(),
            type_iris: vec!["http://schema.org/Person".into()],
            type_prefix: "schema:Person".into(),
            span_start: 0,
            span_end: 13,
            status: EntityStatus::Synced,
            top_relations: vec![Relation {
                predicate_label: "performerIn".into(),
                target_label: "RustConf".into(),
                target_id: "_:e2".into(),
            }],
        };
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains("Niko Matsakis"));
        assert!(json.contains("synced"));
        assert!(json.contains("performerIn"));
    }

    #[test]
    fn byte_to_char_ascii() {
        assert_eq!(byte_to_char_offset("hello world", 5), 5);
    }

    #[test]
    fn byte_to_char_multibyte() {
        // "cafe\u{0301}" = "café" — the é is 2 bytes (U+0301 combining accent)
        let s = "caf\u{00e9}!"; // é is 2 bytes in UTF-8, 1 UTF-16 code unit
        assert_eq!(byte_to_char_offset(s, 3), 3); // "caf" = 3 bytes, 3 chars
        assert_eq!(byte_to_char_offset(s, 5), 4); // "café" = 5 bytes, 4 chars
    }

    #[test]
    fn sidecar_status_serializes() {
        let status = SidecarStatus {
            synced: 3,
            stale: 1,
            detached: 0,
            total_triples: 12,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("\"synced\":3"));
    }
}
```

- [ ] **Step 2: Run types tests**

```bash
cargo test -p sparkdown-studio --lib types::tests
```

Expected: All 4 tests pass.

- [ ] **Step 3: Write failing test for SessionRegistry**

Create `studio/src-tauri/src/registry.rs`:

```rust
use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};

use crate::types::DocId;

/// Commands that can be sent to a DocumentSession actor.
/// Each variant carries a oneshot reply channel where applicable.
pub enum SessionCommand {
    UpdateSource {
        new_source: String,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    GetEntitiesAt {
        start: usize,
        end: usize,
        reply: tokio::sync::oneshot::Sender<Vec<crate::types::EntityDto>>,
    },
    ExportAs {
        format: crate::types::RenderFormat,
        reply: tokio::sync::oneshot::Sender<Result<String, String>>,
    },
    Save {
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    Close,
}

/// Routes commands to the correct DocumentSession by DocId.
pub struct SessionRegistry {
    sessions: RwLock<HashMap<DocId, mpsc::Sender<SessionCommand>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, doc_id: DocId, sender: mpsc::Sender<SessionCommand>) {
        self.sessions.write().await.insert(doc_id, sender);
    }

    pub async fn unregister(&self, doc_id: &str) {
        self.sessions.write().await.remove(doc_id);
    }

    pub async fn get(&self, doc_id: &str) -> Option<mpsc::Sender<SessionCommand>> {
        self.sessions.read().await.get(doc_id).cloned()
    }

    pub async fn is_open(&self, doc_id: &str) -> bool {
        self.sessions.read().await.contains_key(doc_id)
    }

    pub async fn open_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_get_session() {
        let registry = SessionRegistry::new();
        let (tx, _rx) = mpsc::channel(16);
        registry.register("/test/doc.md".into(), tx).await;

        assert!(registry.is_open("/test/doc.md").await);
        assert!(registry.get("/test/doc.md").await.is_some());
        assert_eq!(registry.open_count().await, 1);
    }

    #[tokio::test]
    async fn unregister_removes_session() {
        let registry = SessionRegistry::new();
        let (tx, _rx) = mpsc::channel(16);
        registry.register("/test/doc.md".into(), tx).await;
        registry.unregister("/test/doc.md").await;

        assert!(!registry.is_open("/test/doc.md").await);
        assert_eq!(registry.open_count().await, 0);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let registry = SessionRegistry::new();
        assert!(registry.get("/nonexistent.md").await.is_none());
    }
}
```

- [ ] **Step 4: Run registry tests**

```bash
cargo test -p sparkdown-studio --lib registry::tests
```

Expected: All 3 tests pass.

- [ ] **Step 5: Create events module**

Create `studio/src-tauri/src/events.rs`:

```rust
use tauri::{AppHandle, Emitter};

use crate::types::{DocId, EntityDto, SidecarStatus, StaleAnchor};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentOpenedPayload {
    pub doc_id: DocId,
    pub entities: Vec<EntityDto>,
    pub sidecar_status: SidecarStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntitiesUpdatedPayload {
    pub doc_id: DocId,
    pub entities: Vec<EntityDto>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SidecarStatusPayload {
    pub doc_id: DocId,
    pub status: SidecarStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StaleAnchorsPayload {
    pub doc_id: DocId,
    pub anchors: Vec<StaleAnchor>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorPayload {
    pub doc_id: DocId,
    pub message: String,
}

pub fn emit_document_opened(app: &AppHandle, payload: DocumentOpenedPayload) {
    let _ = app.emit("document-opened", payload);
}

pub fn emit_entities_updated(app: &AppHandle, payload: EntitiesUpdatedPayload) {
    let _ = app.emit("entities-updated", payload);
}

pub fn emit_sidecar_status(app: &AppHandle, payload: SidecarStatusPayload) {
    let _ = app.emit("sidecar-status", payload);
}

pub fn emit_stale_anchors(app: &AppHandle, payload: StaleAnchorsPayload) {
    let _ = app.emit("stale-anchors", payload);
}

pub fn emit_parse_error(app: &AppHandle, payload: ErrorPayload) {
    let _ = app.emit("parse-error", payload);
}

pub fn emit_sidecar_error(app: &AppHandle, payload: ErrorPayload) {
    let _ = app.emit("sidecar-error", payload);
}
```

- [ ] **Step 6: Wire modules into lib.rs**

Update `studio/src-tauri/src/lib.rs`:

```rust
mod commands;
mod events;
mod registry;
mod session;
mod types;

use std::sync::Arc;

pub fn run() {
    let registry = Arc::new(registry::SessionRegistry::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(registry)
        .run(tauri::generate_context!())
        .expect("error while running Sparkdown Studio");
}
```

Create empty placeholder files so the crate compiles:

`studio/src-tauri/src/commands.rs`:
```rust
// Tauri command handlers — implemented in Task 4
```

`studio/src-tauri/src/session.rs`:
```rust
// DocumentSession actor — implemented in Task 3
```

- [ ] **Step 7: Verify full crate compiles and tests pass**

```bash
cargo test -p sparkdown-studio
```

Expected: All tests pass, crate compiles.

- [ ] **Step 8: Commit**

```bash
git add studio/src-tauri/src/
git commit -m "feat(studio): add IPC types, session registry, and event payloads"
```

---

## Task 3: DocumentSession Actor

**Files:**
- Modify: `studio/src-tauri/src/session.rs`
- Test: `studio/src-tauri/src/session.rs` (inline tests)

- [ ] **Step 1: Write failing test for session DTO builder**

Add to `studio/src-tauri/src/session.rs` — the DTO building logic that converts Sparkdown types to `EntityDto`:

```rust
use std::path::{Path, PathBuf};

use sparkdown_core::parser::SparkdownParser;
use sparkdown_core::prefix::PrefixMap;
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
    registry: ThemeRegistry,
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
            registry,
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
                _ => {} // Phase 2+ commands — ignore for now
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
    // Use the graph's prefix map for resolution when available.
    // Fallback to known prefixes for standalone use.
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
    iri.rsplit_once('/')
        .or_else(|| iri.rsplit_once('#'))
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
            iri_to_curie("http://purl.org/dc/elements/1.1/title"),
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
```

- [ ] **Step 2: Run session tests**

```bash
cargo test -p sparkdown-studio --lib session::tests
```

Expected: All 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add studio/src-tauri/src/session.rs
git commit -m "feat(studio): implement DocumentSession actor with DTO building"
```

---

## Task 4: Tauri Commands

**Files:**
- Modify: `studio/src-tauri/src/commands.rs`
- Modify: `studio/src-tauri/src/lib.rs`

- [ ] **Step 1: Implement command handlers**

Replace `studio/src-tauri/src/commands.rs`:

```rust
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::registry::{SessionCommand, SessionRegistry};
use crate::session::DocumentSession;
use crate::types::{DocId, EntityDto, FileEntry, RenderFormat, WorkspaceInfo};

#[tauri::command]
pub async fn open_workspace(app: AppHandle) -> Result<WorkspaceInfo, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });

    let folder = rx.await.map_err(|_| "Dialog cancelled")?.ok_or("No folder selected")?;
    let path = folder.to_string();

    let files = scan_workspace_files(&path).await?;

    Ok(WorkspaceInfo { path, files })
}

#[tauri::command]
pub async fn list_workspace_files(path: String) -> Result<Vec<FileEntry>, String> {
    scan_workspace_files(&path).await
}

#[tauri::command]
pub async fn open_document(
    app: AppHandle,
    registry: State<'_, Arc<SessionRegistry>>,
    path: String,
) -> Result<DocId, String> {
    let file_path = PathBuf::from(&path);

    // Check if already open
    let canonical = file_path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {e}"))?
        .to_string_lossy()
        .into_owned();

    if registry.is_open(&canonical).await {
        return Ok(canonical);
    }

    let (doc_id, tx) = DocumentSession::open(app, file_path).await?;
    registry.register(doc_id.clone(), tx).await;
    Ok(doc_id)
}

#[tauri::command]
pub async fn close_document(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
) -> Result<(), String> {
    if let Some(tx) = registry.get(&doc_id).await {
        let _ = tx.send(SessionCommand::Close).await;
    }
    registry.unregister(&doc_id).await;
    Ok(())
}

#[tauri::command]
pub async fn update_source(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
    new_source: String,
) -> Result<(), String> {
    let tx = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open")?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::UpdateSource {
        new_source,
        reply: reply_tx,
    })
    .await
    .map_err(|_| "Session closed")?;
    reply_rx.await.map_err(|_| "Session dropped")?;
    Ok(())
}

#[tauri::command]
pub async fn get_entities_at(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
    start: usize,
    end: usize,
) -> Result<Vec<EntityDto>, String> {
    let tx = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open")?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::GetEntitiesAt {
        start,
        end,
        reply: reply_tx,
    })
    .await
    .map_err(|_| "Session closed")?;
    reply_rx.await.map_err(|_| "Session dropped")
}

#[tauri::command]
pub async fn export_document(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
    format: RenderFormat,
) -> Result<String, String> {
    let tx = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open")?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::ExportAs {
        format,
        reply: reply_tx,
    })
    .await
    .map_err(|_| "Session closed")?;
    reply_rx.await.map_err(|_| "Session dropped")?
}

#[tauri::command]
pub async fn save_document(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
) -> Result<(), String> {
    let tx = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open")?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::Save { reply: reply_tx })
        .await
        .map_err(|_| "Session closed")?;
    reply_rx.await.map_err(|_| "Session dropped")?
}

/// Recursively scan a directory for .md files.
async fn scan_workspace_files(dir: &str) -> Result<Vec<FileEntry>, String> {
    let mut files = Vec::new();
    let mut stack = vec![PathBuf::from(dir)];

    while let Some(current) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&current)
            .await
            .map_err(|e| format!("Cannot read directory: {e}"))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden directories
                if !path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    stack.push(path);
                }
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let sidecar = path.with_extension("sparkdown-sem");
                files.push(FileEntry {
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    path: path.to_string_lossy().into_owned(),
                    has_sidecar: sidecar.exists(),
                });
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
```

- [ ] **Step 2: Register commands in lib.rs**

Update `studio/src-tauri/src/lib.rs`:

```rust
mod commands;
mod events;
mod registry;
mod session;
mod types;

use std::sync::Arc;

pub fn run() {
    let registry = Arc::new(registry::SessionRegistry::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(registry)
        .invoke_handler(tauri::generate_handler![
            commands::open_workspace,
            commands::list_workspace_files,
            commands::open_document,
            commands::close_document,
            commands::update_source,
            commands::get_entities_at,
            commands::export_document,
            commands::save_document,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sparkdown Studio");
}
```

- [ ] **Step 3: Verify compilation**

```bash
cargo build -p sparkdown-studio
```

Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add studio/src-tauri/src/commands.rs studio/src-tauri/src/lib.rs
git commit -m "feat(studio): add Tauri command handlers for document and workspace operations"
```

---

## Task 5: Frontend Theme and TypeScript Types

**Files:**
- Create: `studio/src/lib/theme/colors.ts`
- Create: `studio/src/lib/theme/tokens.css`
- Create: `studio/src/lib/tauri/commands.ts`

- [ ] **Step 1: Create entity type color mapping**

Create `studio/src/lib/theme/colors.ts`:

```typescript
export const ENTITY_COLORS: Record<string, string> = {
  'schema:Person': '#F59E0B',
  'schema:Place': '#14B8A6',
  'schema:Event': '#8B5CF6',
  'schema:Organization': '#6366F1',
  'schema:Article': '#22C55E',
  'schema:CreativeWork': '#22C55E',
  'sd:Review': '#F43F5E',
  'sd:Abstract': '#F43F5E',
  'sd:Argument': '#F43F5E',
  'foaf:Person': '#F59E0B',
  'foaf:Organization': '#6366F1',
};

const DEFAULT_COLOR = '#6B7280';

export function entityColor(typePrefix: string): string {
  return ENTITY_COLORS[typePrefix] ?? DEFAULT_COLOR;
}
```

- [ ] **Step 2: Create CSS design tokens**

Create `studio/src/lib/theme/tokens.css`:

```css
:root {
  --bg-app: #0F0F0F;
  --bg-editor: #1A1A1A;
  --bg-sidebar: #141414;
  --bg-tray: #111111;

  --text-primary: #E5E5E5;
  --text-secondary: #A3A3A3;
  --text-muted: #737373;

  --border-subtle: #2A2A2A;

  --font-editor: 'JetBrains Mono', 'Berkeley Mono', monospace;
  --font-ui: 'Inter', -apple-system, sans-serif;

  --font-size-editor: 14px;
  --font-size-ui: 13px;
  --font-size-label: 11px;

  --entity-opacity-bg: 0.15;
  --entity-opacity-underline: 0.20;

  --sidebar-width: 240px;
  --tray-height: 28px;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: var(--bg-app);
  color: var(--text-primary);
  font-family: var(--font-ui);
  font-size: var(--font-size-ui);
  overflow: hidden;
}
```

- [ ] **Step 3: Create typed Tauri command wrappers**

Create `studio/src/lib/tauri/commands.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface EntityDto {
  id: string;
  label: string;
  type_iris: string[];
  type_prefix: string;
  span_start: number;
  span_end: number;
  status: 'synced' | 'stale' | 'detached';
  top_relations: Relation[];
}

export interface Relation {
  predicate_label: string;
  target_label: string;
  target_id: string;
}

export interface SidecarStatus {
  synced: number;
  stale: number;
  detached: number;
  total_triples: number;
}

export interface StaleAnchor {
  entity_id: string;
  old_snippet: string;
  new_text: string;
  span_start: number;
  span_end: number;
}

export interface FileEntry {
  name: string;
  path: string;
  has_sidecar: boolean;
}

export interface WorkspaceInfo {
  path: string;
  files: FileEntry[];
}

export async function openWorkspace(): Promise<WorkspaceInfo> {
  return invoke('open_workspace');
}

export async function listWorkspaceFiles(path: string): Promise<FileEntry[]> {
  return invoke('list_workspace_files', { path });
}

export async function openDocument(path: string): Promise<string> {
  return invoke('open_document', { path });
}

export async function closeDocument(docId: string): Promise<void> {
  return invoke('close_document', { docId });
}

export async function updateSource(docId: string, newSource: string): Promise<void> {
  return invoke('update_source', { docId, newSource });
}

export async function getEntitiesAt(docId: string, start: number, end: number): Promise<EntityDto[]> {
  return invoke('get_entities_at', { docId, start, end });
}

export async function exportDocument(docId: string, format: 'html_rdfa' | 'json_ld' | 'turtle'): Promise<string> {
  return invoke('export_document', { docId, format });
}

export async function saveDocument(docId: string): Promise<void> {
  return invoke('save_document', { docId });
}
```

- [ ] **Step 4: Commit**

```bash
git add studio/src/lib/
git commit -m "feat(studio): add theme tokens, entity colors, and typed Tauri command wrappers"
```

---

## Task 6: Svelte State Stores and Event Listeners

**Files:**
- Create: `studio/src/lib/stores/workspace.svelte.ts`
- Create: `studio/src/lib/stores/document.svelte.ts`
- Create: `studio/src/lib/stores/events.ts`

- [ ] **Step 1: Create workspace state**

Create `studio/src/lib/stores/workspace.svelte.ts`:

```typescript
import type { FileEntry } from '$lib/tauri/commands';

let workspacePath = $state<string | null>(null);
let fileList = $state<FileEntry[]>([]);
let activeDocId = $state<string | null>(null);

export function getWorkspacePath() { return workspacePath; }
export function getFileList() { return fileList; }
export function getActiveDocId() { return activeDocId; }

export function setWorkspacePath(path: string | null) { workspacePath = path; }
export function setFileList(files: FileEntry[]) { fileList = files; }
export function setActiveDocId(docId: string | null) { activeDocId = docId; }
```

- [ ] **Step 2: Create document state**

Create `studio/src/lib/stores/document.svelte.ts`:

```typescript
import type { EntityDto, SidecarStatus, StaleAnchor } from '$lib/tauri/commands';

let entities = $state<EntityDto[]>([]);
let sidecarStatus = $state<SidecarStatus>({ synced: 0, stale: 0, detached: 0, total_triples: 0 });
let staleAnchors = $state<StaleAnchor[]>([]);

export function getEntities() { return entities; }
export function getSidecarStatus() { return sidecarStatus; }
export function getStaleAnchors() { return staleAnchors; }

export function setEntities(e: EntityDto[]) { entities = e; }
export function setSidecarStatus(s: SidecarStatus) { sidecarStatus = s; }
export function setStaleAnchors(a: StaleAnchor[]) { staleAnchors = a; }

export function clearDocumentState() {
  entities = [];
  sidecarStatus = { synced: 0, stale: 0, detached: 0, total_triples: 0 };
  staleAnchors = [];
}
```

- [ ] **Step 3: Create event listeners**

Create `studio/src/lib/stores/events.ts`:

```typescript
import { listen } from '@tauri-apps/api/event';
import { setEntities, setSidecarStatus, setStaleAnchors } from './document.svelte';
import type { EntityDto, SidecarStatus, StaleAnchor } from '$lib/tauri/commands';

interface DocumentOpenedPayload {
  doc_id: string;
  entities: EntityDto[];
  sidecar_status: SidecarStatus;
}

interface EntitiesUpdatedPayload {
  doc_id: string;
  entities: EntityDto[];
}

interface SidecarStatusPayload {
  doc_id: string;
  status: SidecarStatus;
}

interface StaleAnchorsPayload {
  doc_id: string;
  anchors: StaleAnchor[];
}

interface ErrorPayload {
  doc_id: string;
  message: string;
}

let unlisteners: (() => void)[] = [];

export async function setupEventListeners() {
  unlisteners.push(
    await listen<DocumentOpenedPayload>('document-opened', (event) => {
      setEntities(event.payload.entities);
      setSidecarStatus(event.payload.sidecar_status);
    }),

    await listen<EntitiesUpdatedPayload>('entities-updated', (event) => {
      setEntities(event.payload.entities);
    }),

    await listen<SidecarStatusPayload>('sidecar-status', (event) => {
      setSidecarStatus(event.payload.status);
    }),

    await listen<StaleAnchorsPayload>('stale-anchors', (event) => {
      setStaleAnchors(event.payload.anchors);
    }),

    await listen<ErrorPayload>('parse-error', (event) => {
      console.warn('[Sparkdown] Parse error:', event.payload.message);
    }),

    await listen<ErrorPayload>('sidecar-error', (event) => {
      console.warn('[Sparkdown] Sidecar error:', event.payload.message);
    }),
  );
}

export function teardownEventListeners() {
  unlisteners.forEach((fn) => fn());
  unlisteners = [];
}
```

- [ ] **Step 4: Commit**

```bash
git add studio/src/lib/stores/
git commit -m "feat(studio): add Svelte 5 state stores and Tauri event listeners"
```

---

## Task 7: Sidebar and File Tree Components

**Files:**
- Create: `studio/src/lib/components/Sidebar.svelte`
- Create: `studio/src/lib/components/FileTree.svelte`
- Modify: `studio/src/routes/+page.svelte`

- [ ] **Step 1: Create FileTree component**

Create `studio/src/lib/components/FileTree.svelte`:

```svelte
<script lang="ts">
  import { getFileList, getActiveDocId } from '$lib/stores/workspace.svelte';
  import { entityColor } from '$lib/theme/colors';

  interface Props {
    onSelect: (path: string) => void;
  }

  let { onSelect }: Props = $props();
  let fileList = $derived(getFileList());
  let activeDocId = $derived(getActiveDocId());
</script>

<ul class="file-tree">
  {#each fileList as file}
    <li class="file-entry" class:active={activeDocId?.endsWith(file.path)}>
      <button onclick={() => onSelect(file.path)}>
        <span class="file-name">{file.name}</span>
        {#if file.has_sidecar}
          <span class="sidecar-indicator" title="Has semantic sidecar">●</span>
        {/if}
      </button>
    </li>
  {/each}
</ul>

<style>
  .file-tree {
    list-style: none;
    padding: 0;
  }

  .file-entry button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 4px 12px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-family: var(--font-ui);
    font-size: var(--font-size-ui);
    cursor: pointer;
    text-align: left;
  }

  .file-entry button:hover {
    background: var(--border-subtle);
    color: var(--text-primary);
  }

  .file-entry.active button {
    background: var(--border-subtle);
    color: var(--text-primary);
  }

  .sidecar-indicator {
    color: #8B5CF6;
    font-size: 8px;
  }
</style>
```

- [ ] **Step 2: Create Sidebar component**

Create `studio/src/lib/components/Sidebar.svelte`:

```svelte
<script lang="ts">
  import FileTree from './FileTree.svelte';
  import { getWorkspacePath } from '$lib/stores/workspace.svelte';
  import { openWorkspace, listWorkspaceFiles } from '$lib/tauri/commands';
  import { setWorkspacePath, setFileList } from '$lib/stores/workspace.svelte';

  interface Props {
    onFileSelect: (path: string) => void;
  }

  let { onFileSelect }: Props = $props();
  let workspacePath = $derived(getWorkspacePath());

  async function handleOpenWorkspace() {
    try {
      const info = await openWorkspace();
      setWorkspacePath(info.path);
      setFileList(info.files);
    } catch (e) {
      console.error('Failed to open workspace:', e);
    }
  }

  async function handleRefresh() {
    if (workspacePath) {
      try {
        const files = await listWorkspaceFiles(workspacePath);
        setFileList(files);
      } catch (e) {
        console.error('Failed to refresh:', e);
      }
    }
  }
</script>

<aside class="sidebar">
  <div class="workspace-header">
    {#if workspacePath}
      <span class="workspace-name" title={workspacePath}>
        {workspacePath.split('/').pop()}
      </span>
      <button class="refresh-btn" onclick={handleRefresh} title="Refresh">↻</button>
    {:else}
      <button class="open-btn" onclick={handleOpenWorkspace}>Open Folder</button>
    {/if}
  </div>

  {#if workspacePath}
    <FileTree onSelect={onFileSelect} />
  {:else}
    <p class="empty-message">Open a folder to start</p>
  {/if}
</aside>

<style>
  .sidebar {
    width: var(--sidebar-width);
    height: 100vh;
    background: var(--bg-sidebar);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .workspace-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .workspace-name {
    font-weight: 500;
    font-size: var(--font-size-ui);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .open-btn, .refresh-btn {
    background: none;
    border: 1px solid var(--border-subtle);
    color: var(--text-secondary);
    padding: 4px 12px;
    border-radius: 4px;
    cursor: pointer;
    font-family: var(--font-ui);
    font-size: var(--font-size-ui);
  }

  .refresh-btn {
    border: none;
    padding: 4px;
    font-size: 16px;
  }

  .open-btn:hover, .refresh-btn:hover {
    color: var(--text-primary);
    border-color: var(--text-muted);
  }

  .empty-message {
    padding: 16px 12px;
    color: var(--text-muted);
    font-size: var(--font-size-ui);
  }
</style>
```

- [ ] **Step 3: Update main page layout**

Replace `studio/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import '$lib/theme/tokens.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { openDocument } from '$lib/tauri/commands';
  import { setActiveDocId } from '$lib/stores/workspace.svelte';
  import { clearDocumentState } from '$lib/stores/document.svelte';
  import { setupEventListeners, teardownEventListeners } from '$lib/stores/events';
  import { onMount } from 'svelte';

  onMount(() => {
    setupEventListeners();
    return teardownEventListeners;
  });

  async function handleFileSelect(path: string) {
    try {
      clearDocumentState();
      const docId = await openDocument(path);
      setActiveDocId(docId);
    } catch (e) {
      console.error('Failed to open document:', e);
    }
  }
</script>

<div class="app-layout">
  <Sidebar onFileSelect={handleFileSelect} />
  <main class="editor-area">
    <p class="placeholder">Select a file to start editing</p>
  </main>
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
  }

  .editor-area {
    flex: 1;
    background: var(--bg-editor);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .placeholder {
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 4: Verify the app launches**

```bash
cd studio && pnpm tauri dev
```

Expected: App window opens showing sidebar with "Open Folder" button and empty editor area. Clicking "Open Folder" opens a system file picker.

- [ ] **Step 5: Commit**

```bash
git add studio/src/
git commit -m "feat(studio): add sidebar, file tree, and app layout with workspace management"
```

---

## Task 8: CodeMirror Editor with Markdown Support

**Files:**
- Create: `studio/src/lib/components/EditorPane.svelte`
- Create: `studio/src/lib/components/CodeMirrorEditor.svelte`
- Create: `studio/src/lib/components/SuggestionTray.svelte`
- Modify: `studio/src/routes/+page.svelte`
- Modify: `studio/package.json` (add codemirror deps)

- [ ] **Step 1: Install CodeMirror dependencies**

```bash
cd studio && pnpm add codemirror @codemirror/lang-markdown @codemirror/language @codemirror/state @codemirror/view @codemirror/theme-one-dark
```

- [ ] **Step 2: Create SuggestionTray component**

Create `studio/src/lib/components/SuggestionTray.svelte`:

```svelte
<script lang="ts">
  import { getEntities, getSidecarStatus } from '$lib/stores/document.svelte';

  let entities = $derived(getEntities());
  let status = $derived(getSidecarStatus());

  let statusText = $derived.by(() => {
    const stale = status.stale;
    const detached = status.detached;
    if (stale === 0 && detached === 0) return 'synced';
    const parts = [];
    if (stale > 0) parts.push(`${stale} stale`);
    if (detached > 0) parts.push(`${detached} detached`);
    return parts.join(', ');
  });
</script>

<div class="suggestion-tray">
  <span class="tray-item">{entities.length} entities</span>
  <span class="tray-separator">·</span>
  <span class="tray-item">sidecar: {statusText}</span>
  <span class="tray-separator">·</span>
  <span class="tray-item">{status.total_triples} triples</span>
</div>

<style>
  .suggestion-tray {
    height: var(--tray-height);
    background: var(--bg-tray);
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    padding: 0 12px;
    gap: 8px;
    font-size: var(--font-size-label);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .tray-separator {
    opacity: 0.4;
  }
</style>
```

- [ ] **Step 3: Create CodeMirrorEditor component**

Create `studio/src/lib/components/CodeMirrorEditor.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { markdown } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { updateSource } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
  let editorContainer: HTMLDivElement;
  let view: EditorView;
  let debounceTimer: ReturnType<typeof setTimeout>;

  onMount(() => {
    const state = EditorState.create({
      doc: initialContent,
      extensions: [
        keymap.of([...defaultKeymap, ...historyKeymap]),
        history(),
        markdown(),
        oneDark,
        EditorView.theme({
          '&': {
            height: '100%',
            fontSize: 'var(--font-size-editor)',
            fontFamily: 'var(--font-editor)',
          },
          '.cm-content': {
            padding: '16px',
          },
          '.cm-scroller': {
            overflow: 'auto',
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => {
              const docId = getActiveDocId();
              if (docId) {
                const source = update.state.doc.toString();
                updateSource(docId, source).catch(console.error);
              }
            }, 150);
          }
        }),
      ],
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });

    return () => {
      clearTimeout(debounceTimer);
      view.destroy();
    };
  });

  export function setContent(content: string) {
    if (view) {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: content,
        },
      });
    }
  }
</script>

<div class="editor-wrapper" bind:this={editorContainer}></div>

<style>
  .editor-wrapper {
    flex: 1;
    overflow: hidden;
  }

  .editor-wrapper :global(.cm-editor) {
    height: 100%;
  }
</style>
```

- [ ] **Step 4: Create EditorPane component**

Create `studio/src/lib/components/EditorPane.svelte`:

```svelte
<script lang="ts">
  import CodeMirrorEditor from './CodeMirrorEditor.svelte';
  import SuggestionTray from './SuggestionTray.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
  let activeDocId = $derived(getActiveDocId());
</script>

{#if activeDocId}
  <div class="editor-pane">
    <CodeMirrorEditor {initialContent} />
    <SuggestionTray />
  </div>
{:else}
  <div class="empty-state">
    <p>Select a file to start editing</p>
  </div>
{/if}

<style>
  .editor-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--bg-editor);
    overflow: hidden;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-editor);
    color: var(--text-muted);
  }
</style>
```

- [ ] **Step 5: Wire editor into main layout**

Update `studio/src/routes/+page.svelte` to replace the placeholder with the editor pane and load file content on open:

```svelte
<script lang="ts">
  import '$lib/theme/tokens.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import EditorPane from '$lib/components/EditorPane.svelte';
  import { openDocument } from '$lib/tauri/commands';
  import { setActiveDocId } from '$lib/stores/workspace.svelte';
  import { clearDocumentState } from '$lib/stores/document.svelte';
  import { setupEventListeners, teardownEventListeners } from '$lib/stores/events';
  import { onMount } from 'svelte';
  import { readTextFile } from '@tauri-apps/plugin-fs';

  let fileContent = $state('');

  onMount(() => {
    setupEventListeners();
    return teardownEventListeners;
  });

  async function handleFileSelect(path: string) {
    try {
      clearDocumentState();
      fileContent = await readTextFile(path);
      const docId = await openDocument(path);
      setActiveDocId(docId);
    } catch (e) {
      console.error('Failed to open document:', e);
    }
  }
</script>

<div class="app-layout">
  <Sidebar onFileSelect={handleFileSelect} />
  <EditorPane initialContent={fileContent} />
</div>

<style>
  .app-layout {
    display: flex;
    height: 100vh;
    width: 100vw;
  }
</style>
```

- [ ] **Step 6: Verify editor works**

```bash
cd studio && pnpm tauri dev
```

Expected: Open a workspace folder, click a `.md` file, editor loads with markdown content. Suggestion tray shows "0 entities · sidecar: synced · 0 triples". Typing triggers debounced update_source calls.

- [ ] **Step 7: Commit**

```bash
git add studio/
git commit -m "feat(studio): add CodeMirror 6 editor with markdown support and suggestion tray"
```

---

## Task 9: CodeMirror Semantic Extensions

**Files:**
- Create: `studio/src/lib/editor/semantic-gutter.ts`
- Create: `studio/src/lib/editor/entity-decorations.ts`
- Create: `studio/src/lib/editor/whisper-tooltip.ts`
- Create: `studio/src/lib/components/WhisperCard.svelte`
- Modify: `studio/src/lib/components/CodeMirrorEditor.svelte`

- [ ] **Step 1: Create semantic gutter extension**

Create `studio/src/lib/editor/semantic-gutter.ts`:

```typescript
import { gutter, GutterMarker } from '@codemirror/view';
import type { EditorView } from '@codemirror/view';
import { StateField, StateEffect } from '@codemirror/state';
import type { EntityDto } from '$lib/tauri/commands';
import { entityColor } from '$lib/theme/colors';

// Effect to update entities from outside
export const setEntitiesEffect = StateEffect.define<EntityDto[]>();

// State field that holds current entity list
export const entitiesField = StateField.define<EntityDto[]>({
  create: () => [],
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setEntitiesEffect)) return effect.value;
    }
    return value;
  },
});

class EntityDotMarker extends GutterMarker {
  constructor(private colors: string[]) {
    super();
  }

  toDOM() {
    const container = document.createElement('div');
    container.style.display = 'flex';
    container.style.flexDirection = 'column';
    container.style.gap = '1px';
    container.style.padding = '2px 0';

    for (const color of this.colors) {
      const dot = document.createElement('div');
      dot.style.width = '4px';
      dot.style.height = '4px';
      dot.style.borderRadius = '50%';
      dot.style.backgroundColor = color;
      container.appendChild(dot);
    }

    return container;
  }
}

export const semanticGutter = gutter({
  class: 'cm-semantic-gutter',
  lineMarker(view: EditorView, line) {
    const entities = view.state.field(entitiesField);
    const lineFrom = line.from;
    const lineTo = line.to;

    const colors: string[] = [];
    for (const entity of entities) {
      if (entity.span_end > lineFrom && entity.span_start < lineTo) {
        colors.push(entityColor(entity.type_prefix));
      }
    }

    if (colors.length === 0) return null;

    // Deduplicate colors
    const unique = [...new Set(colors)];
    return new EntityDotMarker(unique);
  },
  lineMarkerChange(update) {
    return update.transactions.some((tr) =>
      tr.effects.some((e) => e.is(setEntitiesEffect))
    );
  },
});
```

- [ ] **Step 2: Create entity decorations extension**

Create `studio/src/lib/editor/entity-decorations.ts`:

```typescript
import { ViewPlugin, Decoration } from '@codemirror/view';
import type { DecorationSet, ViewUpdate } from '@codemirror/view';
import { RangeSetBuilder } from '@codemirror/state';
import { entitiesField, setEntitiesEffect } from './semantic-gutter';
import { entityColor } from '$lib/theme/colors';

function buildDecorations(view: import('@codemirror/view').EditorView): DecorationSet {
  const entities = view.state.field(entitiesField);
  const builder = new RangeSetBuilder<Decoration>();

  // Sort by span_start for RangeSetBuilder (requires sorted input)
  const sorted = [...entities].sort((a, b) => a.span_start - b.span_start);

  for (const entity of sorted) {
    const from = entity.span_start;
    const to = Math.min(entity.span_end, view.state.doc.length);

    if (from >= to || from < 0) continue;

    const color = entityColor(entity.type_prefix);
    const style = entity.status === 'synced'
      ? `text-decoration: underline; text-decoration-color: ${color}33; text-underline-offset: 3px;`
      : `text-decoration: underline dotted; text-decoration-color: ${color}26; text-underline-offset: 3px;`;

    builder.add(
      from,
      to,
      Decoration.mark({
        attributes: {
          style,
          'data-entity-id': entity.id,
        },
      })
    );
  }

  return builder.finish();
}

export const entityDecorations = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;

    constructor(view: import('@codemirror/view').EditorView) {
      this.decorations = buildDecorations(view);
    }

    update(update: ViewUpdate) {
      if (
        update.docChanged ||
        update.transactions.some((tr) =>
          tr.effects.some((e) => e.is(setEntitiesEffect))
        )
      ) {
        this.decorations = buildDecorations(update.view);
      }
    }
  },
  {
    decorations: (v) => v.decorations,
  }
);
```

- [ ] **Step 3: Create whisper tooltip extension**

Create `studio/src/lib/editor/whisper-tooltip.ts`:

```typescript
import { hoverTooltip } from '@codemirror/view';
import type { EditorView, Tooltip } from '@codemirror/view';
import { entitiesField } from './semantic-gutter';
import { entityColor } from '$lib/theme/colors';

export const whisperTooltip = hoverTooltip(
  (view: EditorView, pos: number): Tooltip | null => {
    const entities = view.state.field(entitiesField);

    // Find entity at position
    const entity = entities.find(
      (e) => pos >= e.span_start && pos < e.span_end
    );

    if (!entity) return null;

    return {
      pos: entity.span_start,
      end: entity.span_end,
      above: false,
      create() {
        const dom = document.createElement('div');
        dom.className = 'whisper-card';

        const color = entityColor(entity.type_prefix);

        dom.innerHTML = `
          <div style="display: flex; align-items: center; gap: 6px; margin-bottom: 4px;">
            <span style="width: 6px; height: 6px; border-radius: 50%; background: ${color}; flex-shrink: 0;"></span>
            <strong style="color: #E5E5E5; font-size: 13px;">${escapeHtml(entity.label)}</strong>
          </div>
          <div style="color: #A3A3A3; font-size: 11px; margin-bottom: 2px;">${escapeHtml(entity.type_prefix)}</div>
          ${entity.top_relations
            .map(
              (r) =>
                `<div style="color: #737373; font-size: 11px;">${escapeHtml(r.predicate_label)} → ${escapeHtml(r.target_label)}</div>`
            )
            .join('')}
        `;

        dom.style.cssText = `
          background: #1E1E1E;
          border: 1px solid #333;
          border-radius: 6px;
          padding: 8px 10px;
          max-width: 260px;
          font-family: var(--font-ui, Inter, sans-serif);
        `;

        return { dom };
      },
    };
  },
  { hoverTime: 300 }
);

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}
```

- [ ] **Step 4: Wire extensions into CodeMirrorEditor**

Replace `studio/src/lib/components/CodeMirrorEditor.svelte` with the full updated version:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, keymap } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
  import { markdown } from '@codemirror/lang-markdown';
  import { oneDark } from '@codemirror/theme-one-dark';
  import { updateSource, saveDocument } from '$lib/tauri/commands';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { getEntities } from '$lib/stores/document.svelte';
  import { entitiesField, setEntitiesEffect, semanticGutter } from '$lib/editor/semantic-gutter';
  import { entityDecorations } from '$lib/editor/entity-decorations';
  import { whisperTooltip } from '$lib/editor/whisper-tooltip';

  interface Props {
    initialContent?: string;
  }

  let { initialContent = '' }: Props = $props();
  let editorContainer: HTMLDivElement;
  let view: EditorView;
  let debounceTimer: ReturnType<typeof setTimeout>;

  // Push entity updates from Svelte state into CodeMirror
  $effect(() => {
    const current = getEntities();
    if (view) {
      view.dispatch({
        effects: setEntitiesEffect.of(current),
      });
    }
  });

  onMount(() => {
    const state = EditorState.create({
      doc: initialContent,
      extensions: [
        keymap.of([
          ...defaultKeymap,
          ...historyKeymap,
          {
            key: 'Mod-s',
            run: () => {
              const docId = getActiveDocId();
              if (docId) {
                saveDocument(docId).catch(console.error);
              }
              return true;
            },
          },
        ]),
        history(),
        markdown(),
        oneDark,
        entitiesField,
        semanticGutter,
        entityDecorations,
        whisperTooltip,
        EditorView.theme({
          '&': {
            height: '100%',
            fontSize: 'var(--font-size-editor)',
            fontFamily: 'var(--font-editor)',
          },
          '.cm-content': {
            padding: '16px',
          },
          '.cm-scroller': {
            overflow: 'auto',
          },
          '.cm-semantic-gutter': {
            width: '12px',
          },
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => {
              const docId = getActiveDocId();
              if (docId) {
                const source = update.state.doc.toString();
                updateSource(docId, source).catch(console.error);
              }
            }, 150);
          }
        }),
      ],
    });

    view = new EditorView({
      state,
      parent: editorContainer,
    });

    return () => {
      clearTimeout(debounceTimer);
      view.destroy();
    };
  });

  export function setContent(content: string) {
    if (view) {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: content,
        },
      });
    }
  }
</script>

<div class="editor-wrapper" bind:this={editorContainer}></div>

<style>
  .editor-wrapper {
    flex: 1;
    overflow: hidden;
  }

  .editor-wrapper :global(.cm-editor) {
    height: 100%;
  }
</style>
```

- [ ] **Step 5: Verify semantic overlays work**

```bash
cd studio && pnpm tauri dev
```

Expected: Open a workspace with `.md` files that have `.sparkdown-sem` sidecars. The editor should show:
- Colored dots in the gutter next to lines containing entities
- Subtle underlines on entity text spans
- Whisper card tooltip on 300ms hover over entity text

If no sidecar files exist, create a test `.md` and `.sparkdown-sem` pair to verify.

- [ ] **Step 6: Commit**

```bash
git add studio/src/lib/editor/ studio/src/lib/components/
git commit -m "feat(studio): add semantic gutter, entity decorations, and whisper card tooltip"
```

---

## Task 10: Save, Export, and Keyboard Shortcuts

**Files:**
- Modify: `studio/src/lib/components/CodeMirrorEditor.svelte`
- Modify: `studio/src/routes/+page.svelte`

- [ ] **Step 1: Add Ctrl+S save shortcut**

In `CodeMirrorEditor.svelte`, add to the keymap:

```typescript
import { saveDocument } from '$lib/tauri/commands';

// Add to keymap array:
{
  key: 'Mod-s',
  run: () => {
    const docId = getActiveDocId();
    if (docId) {
      saveDocument(docId).catch(console.error);
    }
    return true;
  },
},
```

- [ ] **Step 2: Add export menu to suggestion tray**

Replace `studio/src/lib/components/SuggestionTray.svelte` with the full updated version:

```svelte
<script lang="ts">
  import { getEntities, getSidecarStatus } from '$lib/stores/document.svelte';
  import { getActiveDocId } from '$lib/stores/workspace.svelte';
  import { exportDocument } from '$lib/tauri/commands';

  let entities = $derived(getEntities());
  let status = $derived(getSidecarStatus());
  let activeDocId = $derived(getActiveDocId());
  let showExportMenu = $state(false);

  let statusText = $derived.by(() => {
    const stale = status.stale;
    const detached = status.detached;
    if (stale === 0 && detached === 0) return 'synced';
    const parts: string[] = [];
    if (stale > 0) parts.push(`${stale} stale`);
    if (detached > 0) parts.push(`${detached} detached`);
    return parts.join(', ');
  });

  async function handleExport(format: 'html_rdfa' | 'json_ld' | 'turtle') {
    if (!activeDocId) return;
    try {
      const result = await exportDocument(activeDocId, format);
      console.log(`Exported ${format}:`, result.substring(0, 200));
    } catch (e) {
      console.error('Export failed:', e);
    }
    showExportMenu = false;
  }
</script>

<div class="suggestion-tray">
  <span class="tray-item">{entities.length} entities</span>
  <span class="tray-separator">&middot;</span>
  <span class="tray-item">sidecar: {statusText}</span>
  <span class="tray-separator">&middot;</span>
  <span class="tray-item">{status.total_triples} triples</span>

  <div class="tray-spacer"></div>

  {#if activeDocId}
    <div class="export-wrapper">
      <button class="tray-button" onclick={() => showExportMenu = !showExportMenu}>
        Export
      </button>
      {#if showExportMenu}
        <div class="export-menu">
          <button onclick={() => handleExport('html_rdfa')}>HTML+RDFa</button>
          <button onclick={() => handleExport('json_ld')}>JSON-LD</button>
          <button onclick={() => handleExport('turtle')}>Turtle</button>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .suggestion-tray {
    height: var(--tray-height);
    background: var(--bg-tray);
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    padding: 0 12px;
    gap: 8px;
    font-size: var(--font-size-label);
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .tray-separator {
    opacity: 0.4;
  }

  .tray-spacer {
    flex: 1;
  }

  .export-wrapper {
    position: relative;
  }

  .tray-button {
    background: none;
    border: 1px solid var(--border-subtle);
    color: var(--text-muted);
    padding: 1px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-size: var(--font-size-label);
    font-family: var(--font-ui);
  }

  .tray-button:hover {
    color: var(--text-secondary);
    border-color: var(--text-muted);
  }

  .export-menu {
    position: absolute;
    bottom: 100%;
    right: 0;
    background: #1E1E1E;
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    padding: 4px 0;
    margin-bottom: 4px;
    min-width: 120px;
  }

  .export-menu button {
    display: block;
    width: 100%;
    padding: 4px 12px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: var(--font-size-label);
    font-family: var(--font-ui);
    cursor: pointer;
    text-align: left;
  }

  .export-menu button:hover {
    background: var(--border-subtle);
    color: var(--text-primary);
  }
</style>
```

- [ ] **Step 3: Verify save and export**

```bash
cd studio && pnpm tauri dev
```

Expected: Ctrl+S saves both source and sidecar to disk. Export button produces output in console.

- [ ] **Step 4: Commit**

```bash
git add studio/src/
git commit -m "feat(studio): add Ctrl+S save shortcut and export functionality"
```

---

## Task 11: End-to-End Integration Test

**Files:**
- Test manually with a real `.md` + `.sparkdown-sem` pair

- [ ] **Step 1: Create test fixtures**

Create a test workspace directory with a sample `.md` and `.sparkdown-sem` file pair. Use the CLI or manually create:

```bash
mkdir -p /tmp/sparkdown-test-workspace
```

Create `/tmp/sparkdown-test-workspace/test.md`:
```markdown
---
title: RustConf 2026
prefixes:
  schema: http://schema.org/
---

Niko Matsakis will deliver the keynote at RustConf in Portland.
```

Create the sidecar using the CLI (if available) or manually.

- [ ] **Step 2: Run full integration test**

```bash
cd studio && pnpm tauri dev
```

Test the following flow:
1. Open workspace: select `/tmp/sparkdown-test-workspace/`
2. File tree shows `test.md` with sidecar indicator
3. Click `test.md` — editor loads content
4. Suggestion tray shows entity count and triple count from sidecar
5. Gutter shows colored dots on lines with entities
6. Entity text has subtle underlines
7. Hover over entity text for 300ms — whisper card appears
8. Type in editor — after 150ms debounce, entities update
9. Ctrl+S saves to disk
10. Export produces valid HTML/JSON-LD/Turtle output

- [ ] **Step 3: Fix any issues found during integration**

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat(studio): Phase 1 complete — Sparkdown Studio core shell"
```
