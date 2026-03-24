# rig + BlockSuite PoC: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a throwaway proof-of-concept demonstrating AI agents (rig-core) collaboratively editing a BlockSuite document alongside a human, synced in real time via Yjs CRDT.

**Tech Stack:** SvelteKit 2, Svelte 5 (runes), Tailwind CSS 4, BlockSuite (latest), Vite 7, pnpm, Vitest + Playwright — Rust 2024, Axum 0.8, rig-core 0.33, Yrs 0.21, y-sync 0.5, tokio-tungstenite 0.26

**Spec:** `docs/superpowers/specs/2026-03-24-rig-blocksuite-poc.md`

**Existing scaffold:** `pocblock/` — SvelteKit skeleton already initialized with Svelte 5, Tailwind 4, Vitest, Playwright

---

## File Map

### Frontend (`pocblock/`) — existing SvelteKit project

| File | Responsibility |
|------|---------------|
| `package.json` | SvelteKit + BlockSuite + y-websocket deps |
| `svelte.config.js` | SvelteKit config with `adapter-static`, `ssr: false` for editor route |
| `vite.config.ts` | Vite config |
| `tsconfig.json` | TypeScript config |
| `src/app.html` | HTML shell |
| `src/routes/+layout.ts` | Disable SSR globally (`export const ssr = false`) |
| `src/routes/+page.svelte` | Main editor page — mounts BlockSuite, connects sync |
| `src/routes/+page.ts` | Page load (empty, but needed for SvelteKit) |
| `src/lib/editor.ts` | BlockSuite setup: Schema, DocCollection, Doc, Editor creation |
| `src/lib/sync.ts` | y-websocket provider connection to sync server |
| `src/lib/blocks/agent-note-schema.ts` | `sparkdown:agent-note` block schema definition |
| `src/lib/blocks/agent-note-component.ts` | Lit web component rendering the agent note block |
| `src/lib/blocks/agent-note-service.ts` | Block service (accept/dismiss handlers) |
| `src/lib/blocks/agent-note-spec.ts` | BlockSpec wiring schema + service + view |
| `src/lib/blocks/index.ts` | Re-export all custom block specs |
| `src/lib/styles.css` | Global styles and agent note theming |

### Sync Server (`pocblock/sync-server/`)

| File | Responsibility |
|------|---------------|
| `package.json` | `y-websocket` dependency |
| `start.sh` | Launch script with env vars for callback URL, debounce, port |

### Agent Server (`pocblock/agent-server/`)

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Crate manifest: rig-core, yrs, y-sync, axum, tokio-tungstenite |
| `src/main.rs` | Axum server setup, route registration, provider init, spawn sync client |
| `src/config.rs` | `Config` struct loaded from env vars (provider, model, debounce timing) |
| `src/doc_bridge.rs` | `DocumentView`, `BlockView` — decode Yrs doc into agent-readable struct |
| `src/doc_writer.rs` | Insert/update/delete agent note blocks in a Yrs doc |
| `src/yjs_client.rs` | WebSocket client connecting Yrs doc to y-websocket server as a peer |
| `src/routes.rs` | Axum handlers: `POST /on-doc-update`, `POST /run-agents` |
| `src/agents/mod.rs` | Agent trait, shared types (`EntitySuggestion`, `Summary`, `Question`) |
| `src/agents/entity_detector.rs` | Entity detection agent using rig-core |
| `src/agents/summarizer.rs` | Document summarizer agent |
| `src/agents/question_generator.rs` | Discussion question agent |
| `src/agents/mock_provider.rs` | Test double for rig's `CompletionModel` |

### Root (`pocblock/`)

| File | Responsibility |
|------|---------------|
| `README.md` | Setup instructions, how to run, architecture overview |
| `justfile` | Task runner: `just dev` starts all three services |
| `.env.example` | Template for `ANTHROPIC_API_KEY`, model names, ports |

---

## Task 1: Project Scaffold

The SvelteKit frontend already exists at `pocblock/` with Svelte 5 (runes), Tailwind CSS 4, Vitest + Playwright. This task adds BlockSuite dependencies, the sync server, and the Rust agent server.

**Files:**
- Modify: `pocblock/package.json` (add BlockSuite + y-websocket deps)
- Create: `pocblock/src/routes/+layout.ts` (disable SSR)
- Create: `pocblock/README.md`
- Create: `pocblock/justfile`
- Create: `pocblock/.env.example`
- Create: `pocblock/sync-server/package.json`
- Create: `pocblock/sync-server/start.sh`
- Create: `pocblock/agent-server/Cargo.toml`
- Create: `pocblock/agent-server/src/main.rs` (hello world Axum)

- [ ] **Step 1: Add BlockSuite and Yjs dependencies to existing frontend**

```bash
cd pocblock
pnpm add @blocksuite/presets @blocksuite/blocks @blocksuite/store @blocksuite/block-std @blocksuite/inline @blocksuite/lit y-websocket yjs
```

- [ ] **Step 2: Disable SSR for the editor route**

Create `pocblock/src/routes/+layout.ts`:

```typescript
// BlockSuite requires the DOM — disable SSR globally for PoC
export const ssr = false;
```

- [ ] **Step 3: Initialize sync server**

Create `pocblock/sync-server/package.json`:

```json
{
  "name": "sparkdown-poc-sync",
  "private": true,
  "scripts": {
    "start": "bash start.sh"
  },
  "dependencies": {
    "y-websocket": "^2"
  }
}
```

Create `pocblock/sync-server/start.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail

export CALLBACK_URL="${CALLBACK_URL:-http://localhost:3001/on-doc-update}"
export CALLBACK_DEBOUNCE_WAIT="${CALLBACK_DEBOUNCE_WAIT:-500}"
export CALLBACK_DEBOUNCE_MAXWAIT="${CALLBACK_DEBOUNCE_MAXWAIT:-2000}"

echo "Starting y-websocket on :4444 (callback → $CALLBACK_URL)"
npx y-websocket --port 4444
```

- [ ] **Step 4: Initialize Rust agent server**

Create `pocblock/agent-server/Cargo.toml`:

```toml
[package]
name = "sparkdown-agent-poc"
version = "0.1.0"
edition = "2024"

[dependencies]
rig-core = "0.33"
yrs = "0.21"
y-sync = "0.5"
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = "0.26"
axum = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
thiserror = "2"
```

Create `pocblock/agent-server/src/main.rs`:

```rust
use axum::{Router, routing::get};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("sparkdown_agent_poc=debug,info")
        .init();

    let app = Router::new()
        .route("/health", get(|| async { "ok" }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    tracing::info!("Agent server listening on :3001");
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 5: Create justfile and env template**

Create `poc/justfile`:

```just
# Start all three PoC services
dev:
    just sync &
    just agents &
    just frontend

frontend:
    pnpm dev

sync:
    cd sync-server && npm install && bash start.sh

agents:
    cd agent-server && cargo run

# Run agent server tests
test:
    cd agent-server && cargo test

# Install all dependencies
install:
    pnpm install
    cd sync-server && npm install
    cd agent-server && cargo build
```

Create `poc/.env.example`:

```bash
# LLM Provider (required — pick one)
ANTHROPIC_API_KEY=sk-ant-...
# OPENAI_API_KEY=sk-...

# Model selection
AGENT_MODEL=claude-sonnet-4-5-20250514
# AGENT_MODEL=gpt-4o

# Ports
FRONTEND_PORT=5173
SYNC_PORT=4444
AGENT_PORT=3001

# Agent behavior
DEBOUNCE_MS=800
CONFIDENCE_THRESHOLD=0.6
```

- [ ] **Step 6: Verify scaffold**

```bash
cd pocblock && pnpm dev                             # Should start Vite on :5173
cd pocblock/sync-server && npm install              # Should install y-websocket
cd pocblock/agent-server && cargo build             # Should compile
```

**Milestone:** Three services start independently. Frontend shows existing SvelteKit page. Agent server responds to `/health`. Sync server starts on :4444.

---

## Task 2: BlockSuite Editor in SvelteKit

**Files:**
- Create: `pocblock/src/lib/editor.ts`
- Modify: `pocblock/src/routes/+page.svelte`
- Create: `pocblock/src/lib/styles.css`

- [ ] **Step 1: Create editor setup module**

Create `pocblock/src/lib/editor.ts`:

```typescript
import { DocCollection, Schema, type Doc } from '@blocksuite/store';
import { AffineSchemas } from '@blocksuite/blocks';
import { AffineEditorContainer } from '@blocksuite/presets';

// Import BlockSuite styles
import '@blocksuite/presets/themes/affine.css';

export interface EditorInstance {
  collection: DocCollection;
  doc: Doc;
  editor: AffineEditorContainer;
}

export function createEditor(container: HTMLElement): EditorInstance {
  // Register all built-in AFFiNE block schemas
  const schema = new Schema().register(AffineSchemas);
  const collection = new DocCollection({ schema });

  // Create and initialize the document
  const doc = collection.createDoc();
  doc.load(() => {
    const rootId = doc.addBlock('affine:page');
    doc.addBlock('affine:surface', {}, rootId);
    const noteId = doc.addBlock('affine:note', {}, rootId);
    doc.addBlock('affine:paragraph', {
      type: 'text',
      text: new doc.Text('Start writing here...'),
    }, noteId);
  });

  // Create the editor web component
  const editor = new AffineEditorContainer();
  editor.doc = doc;
  container.appendChild(editor);

  return { collection, doc, editor };
}
```

Note: The `doc.Text()` call and exact block initialization API may need adjustment based on the installed BlockSuite version. The implementation step should test this interactively and correct as needed.

- [ ] **Step 2: Mount editor in Svelte page**

Update `pocblock/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { EditorInstance } from '$lib/editor';

  let container: HTMLElement;
  let editorInstance: EditorInstance | null = null;

  onMount(async () => {
    // Dynamic import — BlockSuite needs the DOM
    const { createEditor } = await import('$lib/editor');
    editorInstance = createEditor(container);
  });

  onDestroy(() => {
    // Cleanup if needed
    editorInstance = null;
  });
</script>

<div class="page">
  <header>
    <h1>Sparkdown Agent PoC</h1>
    <span class="status">Agents: idle</span>
  </header>
  <main bind:this={container} class="editor-container" />
</div>

<style>
  .page {
    max-width: 900px;
    margin: 0 auto;
    padding: 1rem;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0;
    border-bottom: 1px solid #e0e0e0;
    margin-bottom: 1rem;
  }

  header h1 {
    font-size: 1.2rem;
    font-weight: 600;
    margin: 0;
  }

  .status {
    font-size: 0.8rem;
    color: #888;
  }

  .editor-container {
    min-height: 80vh;
  }
</style>
```

- [ ] **Step 3: Verify editor renders**

```bash
cd pocblock && pnpm dev
# Open http://localhost:5173
# Should see a BlockSuite editor with "Start writing here..."
# Can type, create headings, lists, etc.
```

**Troubleshooting notes for implementor:**
- If BlockSuite fails to render, check browser console for missing custom element registrations. You may need to import `@blocksuite/presets/effects` for side-effect registration.
- If styles are broken, ensure the AFFiNE theme CSS is imported.
- Vite may need `optimizeDeps.include` for BlockSuite packages if they use CommonJS internally.

**Milestone:** BlockSuite editor renders in the browser. Can type text, create headings, and see the block structure.

---

## Task 3: Yjs Sync Layer

**Files:**
- Create: `pocblock/src/lib/sync.ts`
- Modify: `pocblock/src/routes/+page.svelte` (connect sync)
- Create: `pocblock/agent-server/src/yjs_client.rs`
- Modify: `pocblock/agent-server/src/main.rs` (spawn sync task)

- [ ] **Step 1: Create frontend sync provider**

Create `pocblock/src/lib/sync.ts`:

```typescript
import { WebsocketProvider } from 'y-websocket';
import type { Doc } from '@blocksuite/store';

const SYNC_URL = 'ws://localhost:4444';
const ROOM_NAME = 'sparkdown-poc';

export function connectSync(doc: Doc): WebsocketProvider {
  // BlockSuite's Doc wraps a Yjs subdocument accessible via spaceDoc
  const ydoc = doc.spaceDoc;

  const provider = new WebsocketProvider(SYNC_URL, ROOM_NAME, ydoc);

  provider.on('status', (event: { status: string }) => {
    console.log(`[sync] ${event.status}`);
  });

  provider.on('sync', (isSynced: boolean) => {
    console.log(`[sync] synced: ${isSynced}`);
  });

  return provider;
}
```

Note: The exact property to access the underlying Yjs doc from BlockSuite may be `doc.spaceDoc`, `doc.ydoc`, or accessed through the collection. The implementor should check the installed BlockSuite API — inspect the `Doc` object in the browser console to find the Yjs Y.Doc instance.

- [ ] **Step 2: Connect sync in the page**

Update `pocblock/src/routes/+page.svelte` — add sync connection in `onMount`:

```typescript
onMount(async () => {
  const { createEditor } = await import('$lib/editor');
  const { connectSync } = await import('$lib/sync');

  editorInstance = createEditor(container);
  const provider = connectSync(editorInstance.doc);

  // Update status indicator
  provider.on('status', (e: { status: string }) => {
    status = e.status === 'connected' ? 'Sync: connected' : 'Sync: disconnected';
  });
});
```

Add a `let status = $state('Sync: connecting...')` rune and bind it in the header.

- [ ] **Step 3: Verify browser-to-browser sync**

```bash
# Terminal 1: start sync server
cd pocblock/sync-server && bash start.sh

# Terminal 2: start frontend
cd pocblock && pnpm dev

# Open two browser tabs at http://localhost:5173
# Type in one tab → text appears in the other
```

- [ ] **Step 4: Create Rust Yjs WebSocket client**

Create `pocblock/agent-server/src/yjs_client.rs`:

```rust
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use yrs::Doc;

const ROOM_NAME: &str = "sparkdown-poc";

/// Connect a Yrs Doc to the y-websocket server as a Yjs peer.
///
/// This function runs indefinitely, keeping the doc synchronized.
/// It uses the y-sync protocol (sync step 1/2 + incremental updates).
pub async fn connect_and_sync(
    ws_url: &str,
    doc: Arc<Mutex<Doc>>,
) -> Result<()> {
    // The y-websocket protocol encodes the room name in the URL path:
    // ws://localhost:4444/sparkdown-poc
    let url = format!("{}/{}", ws_url, ROOM_NAME);

    tracing::info!("Connecting to y-websocket at {url}");

    let (ws_stream, _response) = tokio_tungstenite::connect_async(&url).await?;
    tracing::info!("Connected to y-websocket");

    // Use y-sync to handle the Yjs sync protocol.
    // y-sync provides Awareness + Sync protocol handling over any async stream.
    //
    // Implementation notes for the developer:
    // - y-sync's API may vary by version. Check y-sync 0.5 docs.
    // - If y-sync doesn't provide a direct Connection type, manually implement:
    //   1. Send SyncStep1 (our state vector)
    //   2. Receive SyncStep2 (missing updates from server)
    //   3. Exchange incremental updates bidirectionally
    // - The yrs::Doc should be locked only briefly for reads/writes,
    //   not held across await points.

    // Pseudocode — actual implementation depends on y-sync 0.5 API:
    // let awareness = yrs::sync::Awareness::new(doc.lock().await.clone());
    // let conn = y_sync::net::Connection::new(awareness, ws_stream);
    // conn.run().await?;

    // FALLBACK: If y-sync doesn't fit cleanly, implement manually:
    // split ws_stream into (sink, stream), spawn read/write tasks,
    // use yrs::updates::encoder/decoder for Yjs binary protocol.

    todo!("Implement y-sync protocol — see implementation notes above");

    Ok(())
}
```

**Important:** The y-sync Rust crate API has changed across versions. The implementor must:
1. Check `y-sync = "0.5"` docs on docs.rs
2. If the API doesn't provide a ready-made `Connection`, implement the 3-message Yjs sync protocol manually (this is well-documented in the Yjs protocol spec and is ~80 lines of code)
3. Test with a simple round-trip before wiring agents

- [ ] **Step 5: Wire sync client into main.rs**

Update `pocblock/agent-server/src/main.rs` to spawn the sync task:

```rust
mod yjs_client;

// In main(), after creating the shared doc:
let doc = Arc::new(Mutex::new(yrs::Doc::new()));

// Spawn sync task
let sync_doc = doc.clone();
tokio::spawn(async move {
    if let Err(e) = yjs_client::connect_and_sync("ws://localhost:4444", sync_doc).await {
        tracing::error!("Yjs sync failed: {e}");
    }
});
```

- [ ] **Step 6: Verify Rust server receives document updates**

Add a temporary observation handler on the Yrs doc that logs block changes:

```rust
// After connecting, observe updates:
{
    let doc_guard = doc.lock().await;
    let sub = doc_guard.observe_update_v1(|txn, event| {
        tracing::debug!("Received Yjs update: {} bytes", event.update.len());
    });
    // Keep subscription alive
    std::mem::forget(sub);
}
```

Type in the browser → see "Received Yjs update" in Rust server logs.

**Milestone:** Type in browser → Rust server logs the update. Two browser tabs stay in sync. The Yjs triangle (browser A ↔ sync server ↔ Rust agent server) works.

---

## Task 4: Document Bridge (Yrs → Agent-Readable Structs)

**Files:**
- Create: `pocblock/agent-server/src/doc_bridge.rs`

- [ ] **Step 1: Define DocumentView and BlockView structs**

Create `pocblock/agent-server/src/doc_bridge.rs`:

```rust
use serde::{Deserialize, Serialize};

/// A simplified, agent-friendly view of the BlockSuite document.
/// Agents receive this instead of raw Yjs data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentView {
    pub blocks: Vec<BlockView>,
}

/// One block in the document tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockView {
    pub id: String,
    pub flavour: String,
    pub text: Option<String>,
    pub block_type: Option<String>,
    pub props: serde_json::Value,
    pub children: Vec<String>,
    pub parent: Option<String>,
}

impl DocumentView {
    /// Extract only human-authored text blocks for agent analysis.
    /// Filters out agent notes to prevent feedback loops.
    pub fn text_for_analysis(&self) -> String {
        self.blocks
            .iter()
            .filter(|b| b.flavour == "affine:paragraph" || b.flavour == "affine:list")
            .filter(|b| b.text.is_some())
            .map(|b| format!("[{}] {}", b.id, b.text.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Find the last content block ID (for appending summaries)
    pub fn last_content_block_id(&self) -> Option<&str> {
        self.blocks
            .iter()
            .filter(|b| b.flavour.starts_with("affine:"))
            .last()
            .map(|b| b.id.as_str())
    }
}
```

- [ ] **Step 2: Implement Yrs doc → DocumentView decoder**

This is the most research-intensive step. BlockSuite stores its block tree in a specific Yjs structure. The implementor must:

1. Open the browser console on the working editor
2. Inspect the Y.Doc structure: `window.doc.spaceDoc.toJSON()` (or similar)
3. Identify the Y.Map keys that hold block data (likely a `blocks` map where each key is a block ID and each value is a Y.Map with `sys:flavour`, `sys:children`, `prop:text`, etc.)
4. Implement the decoder in Rust using `yrs` read transactions

```rust
use yrs::{Doc, ReadTxn, Transact, Map, Array, Text as YText};

/// Read the current state of a Yrs Doc into a DocumentView.
///
/// BlockSuite internal structure (to be confirmed by inspection):
/// - Root Y.Map "blocks" → { block_id: Y.Map { sys:flavour, sys:children, prop:text, ... } }
/// - Or root Y.Map "spaces" → { space_id: Y.Map "blocks" → ... }
pub fn read_document(doc: &Doc) -> DocumentView {
    let txn = doc.transact();

    // Attempt to read the blocks map.
    // The exact path depends on BlockSuite's internal Yjs structure.
    // This MUST be verified by inspecting a live document.

    let mut blocks = Vec::new();

    // Try the most common BlockSuite structure:
    // doc.getMap('blocks') or doc.getMap('spaces').get(spaceId).getMap('blocks')
    if let Some(blocks_map) = txn.get_map("blocks") {
        for (key, value) in blocks_map.iter(&txn) {
            if let Some(block_map) = value.to_ymap() {
                let flavour = block_map
                    .get(&txn, "sys:flavour")
                    .and_then(|v| v.to_string(&txn))
                    .unwrap_or_default();

                let text = block_map
                    .get(&txn, "prop:text")
                    .and_then(|v| v.to_ytext())
                    .map(|t| t.get_string(&txn));

                let block_type = block_map
                    .get(&txn, "prop:type")
                    .and_then(|v| v.to_string(&txn));

                let children = block_map
                    .get(&txn, "sys:children")
                    .and_then(|v| v.to_yarray())
                    .map(|arr| {
                        arr.iter(&txn)
                            .filter_map(|v| v.to_string(&txn))
                            .collect()
                    })
                    .unwrap_or_default();

                blocks.push(BlockView {
                    id: key.to_string(),
                    flavour,
                    text,
                    block_type,
                    props: serde_json::Value::Null, // Extend as needed
                    children,
                    parent: None, // Computed in post-processing
                });
            }
        }
    }

    DocumentView { blocks }
}
```

**Critical note:** The Yrs types (`to_ymap`, `to_ytext`, etc.) and the exact BlockSuite Yjs key names (`sys:flavour` vs `flavour`, etc.) must be verified against:
1. The actual Yjs doc structure in the browser (inspect with `doc.spaceDoc.toJSON()`)
2. The Yrs 0.21 API (check docs.rs for `yrs::Map`, `yrs::types::Value`)

The implementor should write this step iteratively — log the raw Yjs structure first, then write the decoder to match.

- [ ] **Step 3: Write tests for document bridge**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use yrs::{Doc, Transact, Map, Text};

    #[test]
    fn empty_doc_returns_empty_view() {
        let doc = Doc::new();
        let view = read_document(&doc);
        assert!(view.blocks.is_empty());
    }

    #[test]
    fn text_for_analysis_excludes_agent_notes() {
        let view = DocumentView {
            blocks: vec![
                BlockView {
                    id: "b1".into(),
                    flavour: "affine:paragraph".into(),
                    text: Some("Human text".into()),
                    block_type: Some("text".into()),
                    props: serde_json::Value::Null,
                    children: vec![],
                    parent: None,
                },
                BlockView {
                    id: "b2".into(),
                    flavour: "sparkdown:agent-note".into(),
                    text: Some("Agent text".into()),
                    block_type: None,
                    props: serde_json::Value::Null,
                    children: vec![],
                    parent: None,
                },
            ],
        };

        let analysis_text = view.text_for_analysis();
        assert!(analysis_text.contains("Human text"));
        assert!(!analysis_text.contains("Agent text"));
    }
}
```

**Milestone:** Can call `read_document(&doc)` on a Yrs doc that received updates from the browser and get a populated `DocumentView` with block IDs, flavours, and text content.

---

## Task 5: rig-core Agents

**Files:**
- Create: `pocblock/agent-server/src/agents/mod.rs`
- Create: `pocblock/agent-server/src/agents/entity_detector.rs`
- Create: `pocblock/agent-server/src/agents/summarizer.rs`
- Create: `pocblock/agent-server/src/agents/question_generator.rs`
- Create: `pocblock/agent-server/src/agents/mock_provider.rs`

- [ ] **Step 1: Define shared agent types**

Create `pocblock/agent-server/src/agents/mod.rs`:

```rust
pub mod entity_detector;
pub mod summarizer;
pub mod question_generator;
#[cfg(test)]
pub mod mock_provider;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An entity identified in the document
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntitySuggestion {
    /// The block ID where the entity appears
    pub block_id: String,
    /// The exact text span matched
    pub text_span: String,
    /// Schema.org type (e.g., "schema:Person")
    pub entity_type: String,
    /// 0.0 to 1.0
    pub confidence: f64,
    /// Why this was identified
    pub reasoning: String,
}

/// A document summary
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    /// One-paragraph summary
    pub text: String,
    /// Key topics covered
    pub topics: Vec<String>,
}

/// A suggested discussion question
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Question {
    /// The question text
    pub text: String,
    /// Which block prompted this question
    pub source_block_id: String,
    /// "clarification", "exploration", "challenge"
    pub question_type: String,
}
```

- [ ] **Step 2: Implement EntityDetector**

Create `pocblock/agent-server/src/agents/entity_detector.rs`:

```rust
use anyhow::Result;
use crate::doc_bridge::DocumentView;
use super::EntitySuggestion;

pub struct EntityDetector<M: rig::completion::CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: rig::completion::CompletionModel> EntityDetector<M> {
    pub fn new(model: M) -> Self {
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(
                "You are a semantic entity detector for a knowledge authoring tool.\n\
                 Given document blocks (format: [block_id] text), identify named entities.\n\
                 For each entity return: block_id, text_span, entity_type (schema.org), \
                 confidence (0-1), and reasoning.\n\
                 Only return entities with confidence >= 0.6.\n\
                 Return a JSON array of objects. If no entities found, return []."
            )
            .temperature(0.3)
            .build();

        Self { agent }
    }

    pub async fn analyze(&self, doc: &DocumentView) -> Result<Vec<EntitySuggestion>> {
        let text = doc.text_for_analysis();
        if text.trim().is_empty() {
            return Ok(vec![]);
        }

        let response: Vec<EntitySuggestion> = self.agent
            .prompt(&format!("Analyze these document blocks:\n\n{text}"))
            .await
            .map(|r: String| serde_json::from_str(&r).unwrap_or_default())?;

        Ok(response)
    }
}
```

**Implementation note:** The exact rig-core 0.33 API for `AgentBuilder`, `.preamble()`, and `.prompt()` should be verified against docs.rs. If `prompt_typed::<Vec<EntitySuggestion>>()` is available (requires `schemars` integration), prefer that over manual JSON parsing.

- [ ] **Step 3: Implement Summarizer**

Create `pocblock/agent-server/src/agents/summarizer.rs`:

```rust
use anyhow::Result;
use crate::doc_bridge::DocumentView;
use super::Summary;

pub struct Summarizer<M: rig::completion::CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: rig::completion::CompletionModel> Summarizer<M> {
    pub fn new(model: M) -> Self {
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(
                "You summarize documents concisely.\n\
                 Given document blocks, return a JSON object with:\n\
                 - text: one-paragraph summary\n\
                 - topics: array of key topic strings\n\
                 If the document is too short to summarize (< 2 sentences), \
                 return {\"text\": \"\", \"topics\": []}."
            )
            .temperature(0.5)
            .build();

        Self { agent }
    }

    pub async fn summarize(&self, doc: &DocumentView) -> Result<Option<Summary>> {
        let text = doc.text_for_analysis();
        if text.trim().is_empty() {
            return Ok(None);
        }

        let response: String = self.agent
            .prompt(&format!("Summarize:\n\n{text}"))
            .await?;

        let summary: Summary = serde_json::from_str(&response)?;
        if summary.text.is_empty() {
            return Ok(None);
        }

        Ok(Some(summary))
    }
}
```

- [ ] **Step 4: Implement QuestionGenerator**

Create `pocblock/agent-server/src/agents/question_generator.rs`:

```rust
use anyhow::Result;
use crate::doc_bridge::DocumentView;
use super::Question;

pub struct QuestionGenerator<M: rig::completion::CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: rig::completion::CompletionModel> QuestionGenerator<M> {
    pub fn new(model: M) -> Self {
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(
                "You suggest discussion questions based on document content.\n\
                 Given document blocks, return a JSON array of 1-3 questions.\n\
                 Each question has: text, source_block_id (which block prompted it), \
                 and question_type (\"clarification\", \"exploration\", or \"challenge\").\n\
                 Focus on what's interesting, unclear, or worth exploring further.\n\
                 If the document is too short, return []."
            )
            .temperature(0.7)
            .build();

        Self { agent }
    }

    pub async fn generate(&self, doc: &DocumentView) -> Result<Vec<Question>> {
        let text = doc.text_for_analysis();
        if text.trim().is_empty() {
            return Ok(vec![]);
        }

        let response: String = self.agent
            .prompt(&format!("Generate questions:\n\n{text}"))
            .await?;

        let questions: Vec<Question> = serde_json::from_str(&response).unwrap_or_default();
        Ok(questions)
    }
}
```

- [ ] **Step 5: Create mock provider for tests**

Create `pocblock/agent-server/src/agents/mock_provider.rs`:

```rust
use serde_json::json;

/// A mock LLM provider for testing agents without API calls.
/// Returns canned responses based on input content.
///
/// Implementation depends on rig-core 0.33's CompletionModel trait.
/// The implementor should:
/// 1. Check what methods CompletionModel requires
/// 2. Implement them to return deterministic JSON responses
/// 3. Use this in tests to avoid API calls and costs
///
/// Example canned responses:
pub fn entity_response() -> serde_json::Value {
    json!([{
        "block_id": "block-1",
        "text_span": "Niko Matsakis",
        "entity_type": "schema:Person",
        "confidence": 0.95,
        "reasoning": "Named individual, known Rust contributor"
    }])
}

pub fn summary_response() -> serde_json::Value {
    json!({
        "text": "The document discusses Rust programming and its community.",
        "topics": ["Rust", "programming", "community"]
    })
}

pub fn question_response() -> serde_json::Value {
    json!([{
        "text": "What specific contributions has Niko Matsakis made to Rust?",
        "source_block_id": "block-1",
        "question_type": "exploration"
    }])
}
```

- [ ] **Step 6: Write agent unit tests**

Add tests in each agent file using the mock provider. Verify:
- Empty input → empty output
- Well-formed input → correctly parsed suggestions
- Malformed LLM response → graceful error (not panic)

**Milestone:** All three agents compile and pass tests with mock providers. `cargo test` succeeds.

---

## Task 6: Agent Note Writer (Rust → Yjs)

**Files:**
- Create: `pocblock/agent-server/src/doc_writer.rs`

- [ ] **Step 1: Implement agent note block insertion**

Create `pocblock/agent-server/src/doc_writer.rs`:

```rust
use yrs::{Doc, Map, Transact, Array};
use anyhow::Result;

/// Insert an agent note block into the Yrs document.
///
/// This creates a new block in BlockSuite's internal Yjs structure
/// with flavour "sparkdown:agent-note" and the given properties.
///
/// The block is inserted as a child of the note container,
/// positioned after `after_block_id` if provided.
pub fn insert_agent_note(
    doc: &Doc,
    after_block_id: Option<&str>,
    agent_id: &str,
    agent_name: &str,
    note_type: &str,  // "entity", "summary", "question"
    content: &str,
    confidence: f64,
) -> Result<String> {
    let block_id = generate_block_id();

    let mut txn = doc.transact_mut();

    // Create the block in the blocks map
    // Structure must match what BlockSuite expects — keys like:
    //   sys:flavour = "sparkdown:agent-note"
    //   sys:id = block_id
    //   sys:children = Y.Array []
    //   prop:agentId = agent_id
    //   prop:agentName = agent_name
    //   prop:noteType = note_type
    //   prop:content = content
    //   prop:confidence = confidence
    //   prop:accepted = false
    //
    // AND register in the parent's sys:children array
    //
    // Implementation notes:
    // 1. Get or create the "blocks" Y.Map from the root
    // 2. Create a nested Y.Map for this block
    // 3. Set all sys: and prop: keys
    // 4. Find the parent note block's sys:children Y.Array
    // 5. Insert the block_id at the correct position (after after_block_id)

    // The exact Yrs API calls depend on:
    // - yrs 0.21 Map/Array API (check docs.rs)
    // - BlockSuite's internal key naming convention (check browser inspection)

    todo!("Implement block insertion — see notes above");

    Ok(block_id)
}

/// Remove all agent note blocks from the document.
/// Called before re-running agents to avoid stale notes accumulating.
pub fn clear_agent_notes(doc: &Doc) -> Result<usize> {
    let mut txn = doc.transact_mut();
    let mut removed = 0;

    // Find all blocks with flavour "sparkdown:agent-note" and delete them
    // Also remove their IDs from parent children arrays

    todo!("Implement agent note cleanup");

    Ok(removed)
}

fn generate_block_id() -> String {
    // BlockSuite uses nanoid-style IDs
    // Use a simple UUID for the PoC
    uuid::Uuid::new_v4().to_string().replace('-', "")[..10].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_read_agent_note() {
        let doc = Doc::new();

        // Set up a minimal BlockSuite-like structure
        {
            let mut txn = doc.transact_mut();
            let blocks = txn.get_or_insert_map("blocks");
            // ... create a root block and note container ...
        }

        let id = insert_agent_note(
            &doc, None,
            "test-agent", "Test Agent", "entity",
            "schema:Person — Test", 0.9,
        ).unwrap();

        // Verify the block exists in the doc
        let txn = doc.transact();
        let blocks = txn.get_map("blocks").unwrap();
        assert!(blocks.get(&txn, &id).is_some());
    }

    #[test]
    fn clear_removes_only_agent_notes() {
        // Insert human blocks + agent notes
        // Call clear_agent_notes
        // Assert human blocks remain, agent notes gone
    }
}
```

**Implementation note:** This is the highest-risk step. The Yrs write API and BlockSuite's expected Yjs structure must align exactly. Approach:
1. First, use the browser inspector to capture the Yjs structure of a document with a known block
2. Replicate that structure in Rust with Yrs
3. Verify by syncing the Rust-written update back to the browser

**Milestone:** Insert an agent note from Rust → it appears in the browser editor via Yjs sync.

---

## Task 7: Agent Execution Pipeline

**Files:**
- Create: `pocblock/agent-server/src/routes.rs`
- Create: `pocblock/agent-server/src/config.rs`
- Modify: `pocblock/agent-server/src/main.rs` (wire everything together)

- [ ] **Step 1: Create config module**

Create `pocblock/agent-server/src/config.rs`:

```rust
#[derive(Debug, Clone)]
pub struct Config {
    pub provider: String,          // "anthropic" or "openai"
    pub model: String,             // e.g., "claude-sonnet-4-5-20250514"
    pub agent_port: u16,
    pub sync_url: String,
    pub debounce_ms: u64,
    pub confidence_threshold: f64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            provider: std::env::var("AGENT_PROVIDER").unwrap_or("anthropic".into()),
            model: std::env::var("AGENT_MODEL")
                .unwrap_or("claude-sonnet-4-5-20250514".into()),
            agent_port: std::env::var("AGENT_PORT")
                .ok().and_then(|p| p.parse().ok()).unwrap_or(3001),
            sync_url: std::env::var("SYNC_URL")
                .unwrap_or("ws://localhost:4444".into()),
            debounce_ms: std::env::var("DEBOUNCE_MS")
                .ok().and_then(|d| d.parse().ok()).unwrap_or(800),
            confidence_threshold: std::env::var("CONFIDENCE_THRESHOLD")
                .ok().and_then(|c| c.parse().ok()).unwrap_or(0.6),
        }
    }
}
```

- [ ] **Step 2: Create route handlers**

Create `pocblock/agent-server/src/routes.rs`:

```rust
use axum::{extract::State, Json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agents::{entity_detector::EntityDetector, summarizer::Summarizer,
                    question_generator::QuestionGenerator};
use crate::doc_bridge::{self, DocumentView};
use crate::doc_writer;

pub struct AppState<M: rig::completion::CompletionModel> {
    pub entity_detector: EntityDetector<M>,
    pub summarizer: Summarizer<M>,
    pub question_gen: QuestionGenerator<M>,
    pub doc: Arc<Mutex<yrs::Doc>>,
    pub config: crate::config::Config,
}

/// Called by y-websocket HTTP callback when the document changes.
pub async fn handle_doc_update<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    State(state): State<Arc<AppState<M>>>,
) -> Json<serde_json::Value> {
    tracing::info!("Document update received, running agents...");

    // 1. Read current document state
    let doc_view = {
        let doc = state.doc.lock().await;
        doc_bridge::read_document(&doc)
    };

    // Skip if document has no meaningful content
    if doc_view.text_for_analysis().trim().is_empty() {
        tracing::debug!("Document is empty, skipping agent run");
        return Json(serde_json::json!({ "status": "skipped", "reason": "empty" }));
    }

    // 2. Run all three agents in parallel
    let (entities, summary, questions) = tokio::join!(
        state.entity_detector.analyze(&doc_view),
        state.summarizer.summarize(&doc_view),
        state.question_gen.generate(&doc_view),
    );

    // 3. Clear old agent notes
    {
        let doc = state.doc.lock().await;
        match doc_writer::clear_agent_notes(&doc) {
            Ok(n) => tracing::debug!("Cleared {n} old agent notes"),
            Err(e) => tracing::warn!("Failed to clear agent notes: {e}"),
        }
    }

    // 4. Write new agent notes to the document
    let mut notes_written = 0;
    {
        let doc = state.doc.lock().await;

        if let Ok(entities) = entities {
            for entity in entities {
                if entity.confidence >= state.config.confidence_threshold {
                    if let Ok(_) = doc_writer::insert_agent_note(
                        &doc,
                        Some(&entity.block_id),
                        "entity-detector",
                        "Entity Detector",
                        "entity",
                        &format!("{} — {} ({})", entity.entity_type, entity.text_span, entity.reasoning),
                        entity.confidence,
                    ) {
                        notes_written += 1;
                    }
                }
            }
        }

        if let Ok(Some(summary)) = summary {
            if let Ok(_) = doc_writer::insert_agent_note(
                &doc,
                doc_view.last_content_block_id(),
                "summarizer",
                "Summarizer",
                "summary",
                &summary.text,
                1.0,
            ) {
                notes_written += 1;
            }
        }

        if let Ok(questions) = questions {
            for question in questions {
                if let Ok(_) = doc_writer::insert_agent_note(
                    &doc,
                    Some(&question.source_block_id),
                    "question-gen",
                    "Question Generator",
                    "question",
                    &question.text,
                    0.8,
                ) {
                    notes_written += 1;
                }
            }
        }
    }
    // Yrs doc changes auto-sync to browser via y-websocket peer connection

    tracing::info!("Wrote {notes_written} agent notes");
    Json(serde_json::json!({ "status": "ok", "notes_written": notes_written }))
}

/// Manual trigger endpoint for testing
pub async fn run_agents_manually<M: rig::completion::CompletionModel + Send + Sync + 'static>(
    State(state): State<Arc<AppState<M>>>,
) -> Json<serde_json::Value> {
    handle_doc_update(State(state)).await
}
```

- [ ] **Step 3: Wire everything in main.rs**

Update `pocblock/agent-server/src/main.rs` to initialize providers, agents, routes, and the sync client:

```rust
mod agents;
mod config;
mod doc_bridge;
mod doc_writer;
mod routes;
mod yjs_client;

use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{Router, routing::{get, post}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("sparkdown_agent_poc=debug,info")
        .init();

    let config = config::Config::from_env();
    tracing::info!("Config: provider={}, model={}", config.provider, config.model);

    // Initialize LLM provider
    let client = rig::providers::anthropic::Client::from_env();
    let model = client.completion_model(&config.model);

    // Create shared Yrs doc
    let doc = Arc::new(Mutex::new(yrs::Doc::new()));

    // Spawn Yjs sync client
    let sync_doc = doc.clone();
    let sync_url = config.sync_url.clone();
    tokio::spawn(async move {
        loop {
            match yjs_client::connect_and_sync(&sync_url, sync_doc.clone()).await {
                Ok(()) => break,
                Err(e) => {
                    tracing::error!("Yjs sync error: {e}, reconnecting in 3s...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    });

    // Create agents
    let state = Arc::new(routes::AppState {
        entity_detector: agents::entity_detector::EntityDetector::new(model.clone()),
        summarizer: agents::summarizer::Summarizer::new(model.clone()),
        question_gen: agents::question_generator::QuestionGenerator::new(model),
        doc,
        config: config.clone(),
    });

    // Routes
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/on-doc-update", post(routes::handle_doc_update))
        .route("/run-agents", post(routes::run_agents_manually))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.agent_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Agent server listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
```

**Implementation note:** The `model.clone()` pattern may not work depending on whether rig's model types implement `Clone`. If not, create the model three times from the client, or wrap in `Arc`.

- [ ] **Step 4: Test the pipeline end-to-end (no browser)**

```bash
# Terminal 1
cd pocblock/sync-server && bash start.sh

# Terminal 2
ANTHROPIC_API_KEY=sk-ant-... cargo run

# Terminal 3 — manually trigger agents
curl -X POST http://localhost:3001/run-agents
```

Check agent server logs for "Wrote N agent notes".

**Milestone:** Agent server processes a document update and writes agent notes back to the Yrs doc. The sync server relays them.

---

## Task 8: Custom Agent Note Block (Frontend)

**Files:**
- Create: `pocblock/src/lib/blocks/agent-note-schema.ts`
- Create: `pocblock/src/lib/blocks/agent-note-component.ts`
- Create: `pocblock/src/lib/blocks/agent-note-service.ts`
- Create: `pocblock/src/lib/blocks/agent-note-spec.ts`
- Create: `pocblock/src/lib/blocks/index.ts`
- Modify: `pocblock/src/lib/editor.ts` (register custom block)

- [ ] **Step 1: Define agent note schema**

Create `pocblock/src/lib/blocks/agent-note-schema.ts`:

```typescript
import { defineBlockSchema } from '@blocksuite/store';

export const AgentNoteBlockSchema = defineBlockSchema({
  flavour: 'sparkdown:agent-note',
  props: (internal) => ({
    agentId: '' as string,
    agentName: '' as string,
    noteType: 'entity' as 'entity' | 'summary' | 'question',
    content: '' as string,
    confidence: 0 as number,
    accepted: false as boolean,
  }),
  metadata: {
    version: 1,
    role: 'content',
    parent: ['affine:note'],
  },
});

export type AgentNoteBlockModel = typeof AgentNoteBlockSchema.model;
```

- [ ] **Step 2: Create Lit web component for rendering**

Create `pocblock/src/lib/blocks/agent-note-component.ts`:

```typescript
import { BlockComponent } from '@blocksuite/lit';
import { html, css } from 'lit';
import { customElement } from 'lit/decorators.js';

@customElement('sparkdown-agent-note')
export class AgentNoteComponent extends BlockComponent {
  static styles = css`
    :host {
      display: block;
      margin: 8px 0;
    }
    .agent-note {
      border-left: 3px solid var(--border-color, #4a9eff);
      background: var(--bg-color, #f0f7ff);
      border-radius: 4px;
      padding: 12px 16px;
      font-size: 0.9em;
      position: relative;
    }
    .agent-note[data-type="entity"] {
      --border-color: #4a9eff;
      --bg-color: #f0f7ff;
    }
    .agent-note[data-type="summary"] {
      --border-color: #2da44e;
      --bg-color: #f0fff4;
    }
    .agent-note[data-type="question"] {
      --border-color: #e16f24;
      --bg-color: #fff8f0;
    }
    .agent-note.accepted {
      opacity: 0.6;
      border-left-style: dashed;
    }
    .header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 6px;
      font-size: 0.8em;
      color: #666;
    }
    .agent-name {
      font-weight: 600;
    }
    .confidence {
      background: #e8e8e8;
      border-radius: 8px;
      padding: 1px 6px;
      font-size: 0.85em;
    }
    .content {
      line-height: 1.5;
    }
    .actions {
      margin-top: 8px;
      display: flex;
      gap: 8px;
    }
    .actions button {
      font-size: 0.8em;
      padding: 2px 10px;
      border-radius: 4px;
      border: 1px solid #ddd;
      background: white;
      cursor: pointer;
    }
    .actions button:hover {
      background: #f0f0f0;
    }
    .actions button.accept {
      border-color: #2da44e;
      color: #2da44e;
    }
    .actions button.dismiss {
      border-color: #cf222e;
      color: #cf222e;
    }
  `;

  override render() {
    const model = this.model;
    const noteType = model.props?.noteType || 'entity';
    const agentName = model.props?.agentName || 'Agent';
    const content = model.props?.content || '';
    const confidence = model.props?.confidence || 0;
    const accepted = model.props?.accepted || false;

    const icon = noteType === 'entity' ? '🔍'
      : noteType === 'summary' ? '📝'
      : '❓';

    const confidencePct = Math.round(confidence * 100);

    return html`
      <div class="agent-note ${accepted ? 'accepted' : ''}"
           data-type="${noteType}">
        <div class="header">
          <span class="agent-name">${icon} ${agentName}</span>
          <span class="confidence">${confidencePct}%</span>
        </div>
        <div class="content">${content}</div>
        ${!accepted ? html`
          <div class="actions">
            <button class="accept" @click=${this._accept}>Accept</button>
            <button class="dismiss" @click=${this._dismiss}>Dismiss</button>
          </div>
        ` : html`<div class="actions"><em>Accepted</em></div>`}
      </div>
    `;
  }

  private _accept() {
    this.doc.updateBlock(this.model, { accepted: true });
  }

  private _dismiss() {
    this.doc.deleteBlock(this.model);
  }
}
```

**Implementation note:** The exact `BlockComponent` API (how to access `model`, `model.props`, `this.doc`) depends on the BlockSuite version. The implementor should check `@blocksuite/lit` exports and the AFFiNE source for reference implementations of custom blocks.

- [ ] **Step 3: Create block service and spec**

Create `pocblock/src/lib/blocks/agent-note-service.ts`:

```typescript
import { BlockService } from '@blocksuite/block-std';

export class AgentNoteBlockService extends BlockService {
  override mounted() {
    // No special behavior needed for PoC
  }
}
```

Create `pocblock/src/lib/blocks/agent-note-spec.ts`:

```typescript
import { literal } from 'lit/static-html.js';
import type { BlockSpec } from '@blocksuite/block-std';
import { AgentNoteBlockSchema } from './agent-note-schema';
import { AgentNoteBlockService } from './agent-note-service';

export const AgentNoteBlockSpec: BlockSpec = {
  schema: AgentNoteBlockSchema,
  service: AgentNoteBlockService,
  view: {
    component: literal`sparkdown-agent-note`,
  },
};
```

Create `pocblock/src/lib/blocks/index.ts`:

```typescript
export { AgentNoteBlockSchema } from './agent-note-schema';
export { AgentNoteBlockSpec } from './agent-note-spec';
// Side-effect: register the web component
import './agent-note-component';
```

- [ ] **Step 4: Register custom block in editor setup**

Update `pocblock/src/lib/editor.ts`:

```typescript
import { AgentNoteBlockSchema, AgentNoteBlockSpec } from './blocks';

// In createEditor():
const schema = new Schema().register([...AffineSchemas, AgentNoteBlockSchema]);

// When creating the editor, register the spec:
// editor.specs = [...defaultSpecs, AgentNoteBlockSpec];
// (exact API depends on BlockSuite version)
```

- [ ] **Step 5: Verify custom block renders**

Manually insert a test block via the browser console:

```javascript
// In browser console:
const noteId = doc.addBlock('sparkdown:agent-note', {
  agentId: 'test',
  agentName: 'Test Agent',
  noteType: 'entity',
  content: 'schema:Person — Test Entity',
  confidence: 0.95,
  accepted: false,
}, noteBlockId);
```

Should see a styled agent note card appear in the editor.

**Milestone:** Custom agent note blocks render correctly in the editor with accept/dismiss buttons. Accept marks the note, dismiss deletes the block.

---

## Task 9: End-to-End Integration

**Files:**
- Modify: `pocblock/src/routes/+page.svelte` (status indicators)

- [ ] **Step 1: Start all services and test the full loop**

```bash
# Terminal 1: sync server
cd pocblock/sync-server && bash start.sh

# Terminal 2: agent server
cd pocblock/agent-server && ANTHROPIC_API_KEY=sk-ant-... cargo run

# Terminal 3: frontend
cd pocblock && pnpm dev
```

Open `http://localhost:5173`. Type: "Niko Matsakis presented his work on Rust's type system at RustConf 2025 in Portland."

Expected: After ~3-5 seconds, agent note blocks appear:
- Entity Detector: `schema:Person — Niko Matsakis`, `schema:Event — RustConf 2025`, `schema:Place — Portland`
- Summarizer: One-paragraph summary
- Question Generator: "What aspects of the type system did Matsakis present?"

- [ ] **Step 2: Add agent status indicator to frontend**

Update `+page.svelte` to poll or listen for agent activity:

```typescript
// Simple polling approach for PoC
let agentStatus = $state('idle');

setInterval(async () => {
  try {
    const res = await fetch('http://localhost:3001/health');
    agentStatus = res.ok ? 'connected' : 'disconnected';
  } catch {
    agentStatus = 'disconnected';
  }
}, 5000);
```

Display in the header: `Agents: {agentStatus}`.

- [ ] **Step 3: Test multi-tab collaboration**

1. Open two browser tabs
2. Type in tab 1 → text syncs to tab 2
3. Agent notes appear in both tabs (synced via Yjs)
4. Dismiss a note in tab 2 → it disappears in tab 1

- [ ] **Step 4: Test agent note loop prevention**

Verify that when agents write notes, the callback fires but agents skip re-analysis of agent note content. Check logs:

```
Document update received, running agents...
Document is empty, skipping agent run    # ← if only agent notes changed
```

If the loop prevention doesn't work (infinite agent runs), add a flag or timestamp-based debounce:
- Track the last set of block IDs agents analyzed
- Skip if the text content hasn't changed since last run

- [ ] **Step 5: Run manual test matrix**

Execute the 10-scenario manual test matrix from the spec (Section 7.4):
1. Basic agent response
2. Multi-paragraph
3. Edit existing text
4. Delete text
5. Accept entity
6. Dismiss note
7. Two users + agents
8. Agent note loop prevention
9. Empty document
10. Provider offline

Document results and any issues found.

**Milestone:** Full end-to-end demo works. Type text → agents analyze → agent notes appear in the editor → can accept/dismiss. Multiple tabs stay in sync.

---

## Task 10: Documentation and Cleanup

**Files:**
- Create: `poc/README.md`

- [ ] **Step 1: Write README with setup instructions**

```markdown
# Sparkdown Agent PoC

Proof-of-concept: AI agents collaboratively editing a BlockSuite document.

## Architecture

SvelteKit (BlockSuite editor) ↔ y-websocket (CRDT sync) ↔ Rust (rig-core agents + Yrs)

## Prerequisites

- Node.js 20+
- Rust 1.85+
- pnpm
- An Anthropic API key

## Quick Start

    cp .env.example .env
    # Edit .env with your ANTHROPIC_API_KEY

    # Terminal 1: sync server
    cd sync-server && npm install && bash start.sh

    # Terminal 2: agent server
    cd agent-server && cargo run

    # Terminal 3: frontend
    cd frontend && pnpm install && pnpm dev

Open http://localhost:5173 and start typing.

## What to Try

1. Write a paragraph about a person or event
2. Wait ~3-5 seconds for agent notes to appear
3. Click "Accept" or "Dismiss" on agent notes
4. Open a second browser tab — see real-time sync
```

- [ ] **Step 2: Record findings and lessons learned**

Add a `FINDINGS.md` capturing:
- What worked well
- What was harder than expected
- BlockSuite Yjs internal structure (document for future reference)
- rig-core API pain points or surprises
- Yrs ↔ y-websocket interop issues
- Performance observations (latency, responsiveness)
- Recommendations for the production architecture

**Milestone:** PoC is complete, documented, and findings recorded.
