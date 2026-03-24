# Proof of Concept: Collaborative AI Agents on a BlockSuite Document

> A SvelteKit + Rust PoC that connects rig-core LLM agents to a BlockSuite editor via Yjs CRDT sync, so agents and humans collaboratively edit the same document in real time.
> GitHub issue: [#6](https://github.com/yestechgroup/sparkdown/issues/6) (Subsystem 1), [#11](https://github.com/yestechgroup/sparkdown/issues/11) (Subsystem 6)

---

## 1. Goal

Prove that:

1. A **rig-core agent** (Rust) can read a BlockSuite document, reason about it, and write new blocks — visible in real time to the human editor.
2. **Multiple agents** can work on the same document concurrently without conflicts, thanks to Yjs CRDT convergence.
3. The **block-based editing model** (not flat text) is a viable substrate for semantic annotation agents.

This is explicitly a throwaway PoC — not production code. It validates the architecture before we invest in Tauri integration.

---

## 2. Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  Browser                                                     │
│  ┌──────────────────────────────────┐                        │
│  │ SvelteKit App (localhost:5173)   │                        │
│  │                                  │                        │
│  │  ┌────────────────────────────┐  │                        │
│  │  │ BlockSuite PageEditor      │  │                        │
│  │  │ (web component)            │  │                        │
│  │  │                            │  │                        │
│  │  │ ● Paragraph blocks         │  │                        │
│  │  │ ● Agent suggestion blocks  │  │                        │
│  │  │   (custom block type)      │  │                        │
│  │  └────────────┬───────────────┘  │                        │
│  │               │ Yjs updates      │                        │
│  │  ┌────────────▼───────────────┐  │                        │
│  │  │ y-websocket client         │  │                        │
│  │  └────────────┬───────────────┘  │                        │
│  └───────────────┼──────────────────┘                        │
│                  │ WebSocket                                  │
│  ┌───────────────▼──────────────────┐                        │
│  │ y-websocket server (Node.js)     │ ◄── localhost:4444     │
│  │ Bridges all clients + persistence│                        │
│  └───────────────┬──────────────────┘                        │
│                  │ HTTP callback on doc change                │
│  ┌───────────────▼──────────────────┐                        │
│  │ Rust Agent Server (Axum)         │ ◄── localhost:3001     │
│  │                                  │                        │
│  │  ┌────────────────────────────┐  │                        │
│  │  │ Yrs (Rust Yjs port)       │  │                        │
│  │  │ Decodes/encodes CRDT      │  │                        │
│  │  └────────────┬───────────────┘  │                        │
│  │               │                  │                        │
│  │  ┌────────────▼───────────────┐  │                        │
│  │  │ rig-core agents            │  │                        │
│  │  │ ● EntityDetector           │  │                        │
│  │  │ ● Summarizer               │  │                        │
│  │  │ ● QuestionGenerator        │  │                        │
│  │  └────────────┬───────────────┘  │                        │
│  │               │                  │                        │
│  │  ┌────────────▼───────────────┐  │                        │
│  │  │ y-websocket client (Rust)  │  │                        │
│  │  │ Pushes CRDT updates back   │  │                        │
│  │  └────────────────────────────┘  │                        │
│  └──────────────────────────────────┘                        │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Why this architecture

- **y-websocket** is the proven bridge. Both the browser and the Rust backend connect as Yjs peers. The server merges updates and broadcasts them. No custom sync protocol needed.
- **Yrs** (Rust Yjs port) gives agents native CRDT access. No Node.js middleman for document manipulation.
- **Axum** is a thin HTTP/WebSocket server — lightweight, familiar, and already in the Tauri ecosystem.
- **BlockSuite renders as web components** — SvelteKit just mounts them in `onMount`. No framework adapter needed.

---

## 3. Components

### 3.1 SvelteKit Frontend

**Purpose:** Render the BlockSuite editor and connect it to the sync server.

**Key files:**
```
poc/
  frontend/
    src/
      routes/
        +page.svelte          # Main editor page (ssr: false)
      lib/
        editor.ts             # BlockSuite setup (DocCollection, Doc, Editor)
        sync.ts               # y-websocket provider connection
        blocks/
          agent-note.ts       # Custom "agent note" block spec
    package.json
    svelte.config.js
    vite.config.ts
```

**Editor setup** (`editor.ts`):
```typescript
import { DocCollection, Schema } from '@blocksuite/store';
import { AffineSchemas } from '@blocksuite/blocks';
import { AffineEditorContainer } from '@blocksuite/presets';
import { AgentNoteSchema } from './blocks/agent-note';

export function createEditor(container: HTMLElement) {
  const schema = new Schema().register([...AffineSchemas, AgentNoteSchema]);
  const collection = new DocCollection({ schema });
  const doc = collection.createDoc();

  doc.load(() => {
    const rootId = doc.addBlock('affine:page');
    doc.addBlock('affine:surface', {}, rootId);
    const noteId = doc.addBlock('affine:note', {}, rootId);
    doc.addBlock('affine:paragraph', { type: 'text' }, noteId);
  });

  const editor = new AffineEditorContainer();
  editor.doc = doc;
  container.appendChild(editor);

  return { collection, doc, editor };
}
```

**Custom agent note block** (`agent-note.ts`):

A read-only block that agents insert to show their suggestions. Visually distinct (colored border, agent icon, accept/dismiss buttons).

```typescript
import { defineBlockSchema } from '@blocksuite/store';

export const AgentNoteSchema = defineBlockSchema({
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
  },
});
```

**Svelte page** (`+page.svelte`):
```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { createEditor } from '$lib/editor';
  import { connectSync } from '$lib/sync';

  let container: HTMLElement;

  onMount(() => {
    const { doc, collection } = createEditor(container);
    connectSync(doc, 'ws://localhost:4444');
  });
</script>

<svelte:head>
  <title>Sparkdown Agent PoC</title>
</svelte:head>

<div bind:this={container} class="editor-container" />

<style>
  .editor-container {
    max-width: 800px;
    margin: 2rem auto;
    min-height: 100vh;
  }
</style>
```

### 3.2 y-websocket Sync Server

**Purpose:** Bridge browser ↔ Rust agent CRDT updates.

A stock y-websocket server with HTTP callbacks enabled:

```bash
CALLBACK_URL=http://localhost:3001/on-doc-update \
CALLBACK_DEBOUNCE_WAIT=500 \
CALLBACK_DEBOUNCE_MAXWAIT=2000 \
npx y-websocket --port 4444
```

When the document changes, after 500ms of inactivity it POSTs a callback to the Rust backend. This is the trigger for agents to process the document.

### 3.3 Rust Agent Server (Axum + rig-core + Yrs)

**Purpose:** Host LLM agents that read the document and write suggestions back.

**Key files:**
```
poc/
  agent-server/
    Cargo.toml
    src/
      main.rs                 # Axum server setup
      doc_bridge.rs           # Yrs ↔ BlockSuite document codec
      agents/
        mod.rs
        entity_detector.rs    # Finds entities in text blocks
        summarizer.rs         # Generates document summaries
        question_generator.rs # Suggests discussion questions
      yjs_client.rs           # WebSocket client to y-websocket server
```

**Cargo.toml:**
```toml
[package]
name = "sparkdown-agent-poc"
edition = "2024"

[dependencies]
rig-core = "0.33"
yrs = "0.21"                    # Rust Yjs port
y-sync = "0.5"                  # Yjs sync protocol for Rust
tokio = { version = "1", features = ["full"] }
axum = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio-tungstenite = "0.26"      # WebSocket client
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

**Document bridge** (`doc_bridge.rs`):

Reads BlockSuite's Yjs structure into a Rust representation agents can work with:

```rust
use yrs::{Doc, ReadTxn, Transact, types::ToJson};
use serde::{Serialize, Deserialize};

/// A simplified view of the BlockSuite document for agents
#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentView {
    pub blocks: Vec<BlockView>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockView {
    pub id: String,
    pub flavour: String,
    pub text: Option<String>,
    pub block_type: Option<String>,  // "text", "h1", "h2", etc.
    pub children: Vec<String>,
}

/// Decode a Yjs binary update into a DocumentView
pub fn decode_document(update: &[u8]) -> anyhow::Result<DocumentView> {
    let doc = Doc::new();
    {
        let mut txn = doc.transact_mut();
        yrs::Update::decode_v1(update)
            .map(|u| txn.apply_update(u))?;
    }

    let txn = doc.transact();
    // BlockSuite stores blocks in a Y.Map called "blocks"
    // Each block has flavour, props (including text), children
    let blocks = read_blocks_from_yjs(&txn)?;

    Ok(DocumentView { blocks })
}

/// Encode an "add agent note" operation as a Yjs update
pub fn encode_agent_note(
    doc: &Doc,
    parent_id: &str,
    agent_id: &str,
    agent_name: &str,
    note_type: &str,
    content: &str,
    confidence: f64,
) -> Vec<u8> {
    let mut txn = doc.transact_mut();
    // Create a new block in the BlockSuite Y.Map structure
    // with flavour "sparkdown:agent-note"
    // ... Yrs operations to insert into the block tree ...
    txn.encode_update_v1()
}
```

**Entity detector agent** (`entity_detector.rs`):

```rust
use rig::providers::anthropic;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EntitySuggestion {
    pub block_id: String,
    pub text_span: String,
    pub entity_type: String,
    pub confidence: f64,
    pub reasoning: String,
}

pub struct EntityDetector {
    agent: rig::agent::Agent<anthropic::completion_model::CompletionModel>,
}

impl EntityDetector {
    pub fn new(client: &anthropic::Client) -> Self {
        let agent = client
            .agent("claude-sonnet-4-5-20250514")
            .preamble(
                "You are a semantic entity detector. Given document blocks, \
                 identify named entities (people, places, organizations, events, \
                 concepts) and return them as structured JSON. \
                 For each entity, specify which block_id it appears in, \
                 the exact text span, the schema.org type, and your confidence."
            )
            .temperature(0.3)
            .build();

        Self { agent }
    }

    pub async fn analyze(&self, doc: &DocumentView) -> anyhow::Result<Vec<EntitySuggestion>> {
        let text_blocks: String = doc.blocks.iter()
            .filter(|b| b.flavour == "affine:paragraph")
            .map(|b| format!("[{}] {}", b.id, b.text.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n");

        if text_blocks.trim().is_empty() {
            return Ok(vec![]);
        }

        let suggestions: Vec<EntitySuggestion> = self.agent
            .prompt_typed(&format!(
                "Analyze these document blocks for entities:\n\n{text_blocks}"
            ))
            .await?;

        Ok(suggestions)
    }
}
```

**Axum server** (`main.rs`):

```rust
use axum::{Router, routing::post, Json, extract::State};
use std::sync::Arc;
use tokio::sync::Mutex;

struct AppState {
    entity_detector: EntityDetector,
    summarizer: Summarizer,
    question_gen: QuestionGenerator,
    ws_url: String,           // y-websocket server URL
    doc: Arc<Mutex<yrs::Doc>>,  // Local Yrs doc mirror
}

#[tokio::main]
async fn main() {
    tracing_subscriber::init();

    let anthropic = rig::providers::anthropic::Client::from_env();

    let state = Arc::new(AppState {
        entity_detector: EntityDetector::new(&anthropic),
        summarizer: Summarizer::new(&anthropic),
        question_gen: QuestionGenerator::new(&anthropic),
        ws_url: "ws://localhost:4444".into(),
        doc: Arc::new(Mutex::new(yrs::Doc::new())),
    });

    // Connect to y-websocket as a peer
    tokio::spawn(yjs_client::connect_and_sync(
        state.ws_url.clone(),
        state.doc.clone(),
    ));

    let app = Router::new()
        .route("/on-doc-update", post(handle_doc_update))
        .route("/run-agents", post(run_agents_manually))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    tracing::info!("Agent server listening on :3001");
    axum::serve(listener, app).await.unwrap();
}

/// Called by y-websocket HTTP callback when the doc changes
async fn handle_doc_update(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Json<serde_json::Value> {
    tracing::info!("Doc update received, running agents...");

    // Read current doc state
    let doc_view = {
        let doc = state.doc.lock().await;
        doc_bridge::read_document(&doc)
    };

    // Run agents in parallel
    let (entities, summary, questions) = tokio::join!(
        state.entity_detector.analyze(&doc_view),
        state.summarizer.summarize(&doc_view),
        state.question_gen.generate(&doc_view),
    );

    // Write agent suggestions back as blocks
    {
        let doc = state.doc.lock().await;
        if let Ok(entities) = entities {
            for entity in entities {
                doc_bridge::insert_agent_note(
                    &doc,
                    &entity.block_id,  // insert after this block
                    "entity-detector",
                    "Entity Detector",
                    "entity",
                    &format!("{}: {} ({})", entity.entity_type, entity.text_span, entity.reasoning),
                    entity.confidence,
                );
            }
        }
        // Similar for summary and questions...
    }
    // The Yrs doc change automatically syncs to all peers via y-websocket

    Json(serde_json::json!({ "status": "ok" }))
}
```

**WebSocket sync client** (`yjs_client.rs`):

Connects the Rust `yrs::Doc` to the y-websocket server as a Yjs peer, using the `y-sync` protocol:

```rust
use tokio_tungstenite::connect_async;
use yrs::Doc;
use y_sync::net::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn connect_and_sync(
    ws_url: String,
    doc: Arc<Mutex<Doc>>,
) -> anyhow::Result<()> {
    let (ws_stream, _) = connect_async(&ws_url).await?;
    tracing::info!("Connected to y-websocket at {ws_url}");

    // y-sync handles the Yjs sync protocol (step1, step2, update exchange)
    // The doc stays synchronized with all other peers
    let conn = Connection::new(doc, ws_stream);
    conn.run().await?;

    Ok(())
}
```

---

## 4. Three Agents for the PoC

| Agent | Trigger | Input | Output (block) | Purpose |
|-------|---------|-------|----------------|---------|
| **EntityDetector** | On doc change (debounced) | All paragraph blocks | `sparkdown:agent-note` with `noteType: "entity"` | Identifies people, places, orgs, events |
| **Summarizer** | On doc change (debounced) | All paragraph blocks | `sparkdown:agent-note` with `noteType: "summary"` | Running summary at the end of the doc |
| **QuestionGenerator** | On doc change (debounced) | All paragraph blocks | `sparkdown:agent-note` with `noteType: "question"` | Suggests follow-up questions / discussion points |

Each agent writes its output as an **agent note block** — a custom BlockSuite block type with a distinctive visual style. The human can:
- **Accept** → the note gets promoted or its data gets used
- **Dismiss** → the block is deleted
- **Ignore** → it stays, faded

---

## 5. Custom Block: `sparkdown:agent-note`

Visual design:

```
┌─ 🤖 Entity Detector ─────────────────── 95% ─┐
│                                                │
│  schema:Person — "Niko Matsakis"               │
│  Known Rust contributor, mentioned in context   │
│  of RustConf presentation.                     │
│                                                │
│  [Accept]  [Dismiss]                           │
└────────────────────────────────────────────────┘
```

- Left border color by agent type: blue (entity), green (summary), orange (question)
- Confidence badge (top-right)
- Agent name (top-left, with robot icon)
- Accept button: writes entity to a semantic store (or in PoC, just marks `accepted: true`)
- Dismiss button: deletes the block

---

## 6. Implementation Plan

### Phase 1: Scaffold (1-2 days)

1. Create `poc/` directory at repo root
2. `poc/frontend/` — `npm create svelte@latest` with TypeScript
3. `poc/agent-server/` — `cargo init`
4. Install BlockSuite: `npm add @blocksuite/presets @blocksuite/blocks @blocksuite/store`
5. Get a basic BlockSuite editor rendering in SvelteKit (`+page.svelte`)
6. **Milestone:** Can type text in the browser

### Phase 2: Yjs Sync (1-2 days)

1. Run `y-websocket` server on port 4444
2. Connect BlockSuite editor to it via `WebsocketProvider`
3. Open two browser tabs → verify real-time sync between them
4. Add `yrs` + `y-sync` to Rust server, connect as a peer
5. Verify: Rust server can read blocks added in the browser
6. **Milestone:** Type in browser, see block content in Rust server logs

### Phase 3: Agents Read (1 day)

1. Implement `doc_bridge.rs` — decode Yrs doc into `DocumentView`
2. Build `EntityDetector` with rig-core (Anthropic provider)
3. Wire up: doc change → agent analyzes → logs suggestions to console
4. **Milestone:** Type "Niko Matsakis spoke at RustConf" → see entity suggestion in server logs

### Phase 4: Agents Write (1-2 days)

1. Define `sparkdown:agent-note` custom block schema (frontend)
2. Implement `doc_bridge::insert_agent_note()` — write agent note blocks via Yrs
3. Wire up: agent suggestion → Yrs mutation → syncs to browser
4. Agent note appears in the editor after the paragraph it annotates
5. **Milestone:** Type text → agent note blocks appear in the editor automatically

### Phase 5: Multi-Agent + Polish (1-2 days)

1. Add Summarizer and QuestionGenerator agents
2. Run all three in parallel via `tokio::join!`
3. Add accept/dismiss buttons to agent note block (frontend)
4. Add debouncing (don't re-run agents if only agent notes changed)
5. **Milestone:** Full demo — type a paragraph, three types of agent notes appear

---

## 7. Testing Strategy

### 7.1 Unit Tests (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Mock rig provider that returns canned entity suggestions
    #[tokio::test]
    async fn entity_detector_parses_response() {
        let mock = MockProvider::returning(json!([{
            "block_id": "block-1",
            "text_span": "Niko Matsakis",
            "entity_type": "schema:Person",
            "confidence": 0.95,
            "reasoning": "Proper noun"
        }]));

        let detector = EntityDetector::with_provider(mock);
        let doc = DocumentView {
            blocks: vec![BlockView {
                id: "block-1".into(),
                flavour: "affine:paragraph".into(),
                text: Some("Niko Matsakis spoke at RustConf.".into()),
                block_type: Some("text".into()),
                children: vec![],
            }],
        };

        let results = detector.analyze(&doc).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "schema:Person");
    }

    /// Verify Yrs document bridge can round-trip blocks
    #[test]
    fn yrs_roundtrip_block_insert() {
        let doc = yrs::Doc::new();
        insert_agent_note(
            &doc, "parent-1", "test-agent", "Test", "entity",
            "schema:Person — Test", 0.9,
        );

        let view = read_document(&doc);
        let agent_blocks: Vec<_> = view.blocks.iter()
            .filter(|b| b.flavour == "sparkdown:agent-note")
            .collect();
        assert_eq!(agent_blocks.len(), 1);
    }

    /// Empty document produces no suggestions
    #[tokio::test]
    async fn empty_doc_no_suggestions() {
        let mock = MockProvider::returning(json!([]));
        let detector = EntityDetector::with_provider(mock);
        let doc = DocumentView { blocks: vec![] };

        let results = detector.analyze(&doc).await.unwrap();
        assert!(results.is_empty());
    }

    /// Agent notes should not trigger re-analysis (avoid infinite loop)
    #[tokio::test]
    async fn agent_notes_filtered_from_analysis() {
        let doc = DocumentView {
            blocks: vec![
                BlockView {
                    id: "b1".into(),
                    flavour: "affine:paragraph".into(),
                    text: Some("Real content".into()),
                    block_type: Some("text".into()),
                    children: vec![],
                },
                BlockView {
                    id: "b2".into(),
                    flavour: "sparkdown:agent-note".into(),
                    text: Some("Agent wrote this".into()),
                    block_type: None,
                    children: vec![],
                },
            ],
        };

        // EntityDetector should only see "Real content", not agent notes
        let text = doc.text_for_analysis();
        assert!(text.contains("Real content"));
        assert!(!text.contains("Agent wrote this"));
    }
}
```

### 7.2 Integration Tests (Yjs Sync)

```rust
/// Verify browser → Rust sync path
#[tokio::test]
async fn browser_edit_reaches_rust_doc() {
    // 1. Start y-websocket server (or mock)
    // 2. Connect two Yrs docs as peers
    // 3. Write a block on doc A
    // 4. Assert doc B receives the block
}

/// Verify Rust → browser sync path
#[tokio::test]
async fn rust_agent_note_reaches_browser_doc() {
    // 1. Connect two Yrs docs
    // 2. Insert agent note on doc A (Rust side)
    // 3. Assert doc B has the agent note block
}

/// Concurrent agent writes don't conflict
#[tokio::test]
async fn concurrent_agent_writes_converge() {
    // 1. Three agents write notes to the same doc simultaneously
    // 2. Assert all three notes are present (CRDT guarantees this)
    // 3. Assert no data loss or corruption
}
```

### 7.3 Frontend Tests (Playwright)

```typescript
// tests/editor.spec.ts
import { test, expect } from '@playwright/test';

test('editor loads and accepts input', async ({ page }) => {
  await page.goto('http://localhost:5173');
  // BlockSuite editor should be visible
  await expect(page.locator('affine-editor-container')).toBeVisible();
  // Type some text
  await page.click('.affine-paragraph-block-container');
  await page.keyboard.type('Niko Matsakis spoke at RustConf');
  // Wait for agent notes to appear (with timeout for LLM latency)
  await expect(page.locator('sparkdown-agent-note')).toBeVisible({ timeout: 15000 });
});

test('dismiss removes agent note', async ({ page }) => {
  // ... setup with existing agent note ...
  await page.click('[data-action="dismiss"]');
  await expect(page.locator('sparkdown-agent-note')).not.toBeVisible();
});

test('two tabs see same content', async ({ browser }) => {
  const page1 = await browser.newPage();
  const page2 = await browser.newPage();
  await page1.goto('http://localhost:5173');
  await page2.goto('http://localhost:5173');

  await page1.click('.affine-paragraph-block-container');
  await page1.keyboard.type('Hello from tab 1');

  // Tab 2 should see the text
  await expect(page2.locator('text=Hello from tab 1')).toBeVisible({ timeout: 5000 });
});
```

### 7.4 Manual Test Matrix

| # | Scenario | Steps | Expected |
|---|----------|-------|----------|
| 1 | Basic agent response | Type a paragraph with named entities | Entity, summary, and question agent notes appear below the paragraph |
| 2 | Multi-paragraph | Type 3 paragraphs about different topics | Each gets its own agent notes |
| 3 | Edit existing text | Modify a paragraph that already has agent notes | Old notes are replaced by new analysis |
| 4 | Delete text | Delete a paragraph | Associated agent notes are cleaned up |
| 5 | Accept entity | Click "Accept" on an entity note | Note is visually marked as accepted |
| 6 | Dismiss note | Click "Dismiss" on any agent note | Note disappears |
| 7 | Two users + agents | Open two browser tabs, type in both | Both tabs see each other's text AND agent notes |
| 8 | Agent note loop prevention | Agents insert notes | Notes do not trigger further agent analysis (no infinite loop) |
| 9 | Empty document | Open fresh document, don't type | No agent notes appear |
| 10 | Provider offline | Stop the LLM API | Error is logged, no crash, no broken blocks |

### 7.5 What We're NOT Testing in the PoC

- Performance at scale (100+ blocks, many agents)
- Provider fallback chains
- Persistent storage across restarts
- Security (auth, API key exposure)
- Mobile/responsive layout

These are deferred to the production implementation.

---

## 8. Success Criteria

The PoC is successful if we can demonstrate:

| # | Criterion | How to verify |
|---|-----------|--------------|
| 1 | Human types → agents respond | Type a sentence, agent notes appear within 5 seconds |
| 2 | Agent notes are real blocks | Agent notes persist in the Yjs doc, survive page reload |
| 3 | No sync conflicts | Two browser tabs + Rust agents all editing → document converges correctly |
| 4 | Agents work in parallel | Three agent types produce output from a single trigger |
| 5 | Block model works for semantics | Entity suggestions reference specific block IDs, not byte offsets |

---

## 9. Key Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `@blocksuite/presets` | latest | Editor UI (PageEditor web component) |
| `@blocksuite/blocks` | latest | Built-in block types (paragraph, heading, etc.) |
| `@blocksuite/store` | latest | Doc, Schema, block CRUD |
| `y-websocket` | ^2 | Yjs WebSocket sync server + client |
| `rig-core` | 0.33 | LLM agent framework (Rust) |
| `yrs` | 0.21 | Yjs Rust port (CRDT operations) |
| `y-sync` | 0.5 | Yjs sync protocol for Rust |
| `axum` | 0.8 | HTTP server (Rust) |
| `tokio-tungstenite` | 0.26 | WebSocket client (Rust) |

---

## 10. Open Questions

1. **BlockSuite Yjs internal format:** The exact Y.Map structure BlockSuite uses for blocks needs to be reverse-engineered or found in their docs. The `doc_bridge.rs` implementation depends on this.
2. **Agent note placement:** Insert after the block being annotated, or collect all notes in a sidebar panel? The block approach is more native to BlockSuite; the panel approach is less intrusive.
3. **Debounce scope:** Should we debounce per-block or per-document? Per-block is more granular but more complex.
4. **Yrs ↔ y-websocket compatibility:** Need to verify the Yjs sync protocol versions match between the JS and Rust implementations.
5. **Agent note cleanup:** When the source text changes, should old agent notes be automatically deleted, or marked stale?

---

## 11. What This PoC Validates for Sparkdown

If successful, this PoC proves:

- **Block-based editing + semantic agents is viable.** Agents can work with structured blocks instead of flat text spans, which may be a better model than byte-offset anchors for the semantic overlay.
- **CRDT sync enables agent collaboration.** Multiple agents (and humans) can write to the same document without a custom conflict resolution layer.
- **rig-core is production-suitable.** The structured output, streaming, and multi-provider support work in a real integration scenario.
- **The path from PoC to Tauri is clear.** Replace Axum with Tauri commands, replace y-websocket with a local Yrs doc, and the architecture carries over.
