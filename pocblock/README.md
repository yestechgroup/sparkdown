# Sparkdown Agent PoC

Proof-of-concept: AI agents collaboratively editing a BlockSuite document in real time.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Browser (localhost:5173)                               │
│  ┌───────────────────────────────────┐                  │
│  │ SvelteKit + BlockSuite Editor     │                  │
│  │  • Paragraph blocks               │                  │
│  │  • Agent suggestion blocks        │                  │
│  └───────────────┬───────────────────┘                  │
│                  │ WebSocket (Yjs CRDT)                  │
│  ┌───────────────▼───────────────────┐                  │
│  │ y-websocket server (port 4444)    │                  │
│  │ Bridges all peers + HTTP callback │                  │
│  └───────────────┬───────────────────┘                  │
│                  │ WebSocket (Yjs sync protocol)        │
│  ┌───────────────▼───────────────────┐                  │
│  │ Rust Agent Server (port 3001)     │                  │
│  │  • rig-core agents (Anthropic)    │                  │
│  │  • Yrs CRDT (Rust Yjs port)      │                  │
│  │  • Axum HTTP server               │                  │
│  └───────────────────────────────────┘                  │
└─────────────────────────────────────────────────────────┘
```

- **Browser**: BlockSuite editor connected to y-websocket as a Yjs peer
- **Sync server**: Stock y-websocket server bridging all peers, with a debounced HTTP callback on doc changes
- **Agent server**: Rust backend with rig-core LLM agents that read/write blocks via Yrs CRDT

## Prerequisites

- **Node.js** 20+
- **Rust** 1.85+ (2024 edition)
- **pnpm** (any recent version)
- **Anthropic API key** (for the rig-core agents)

## Quick Start

All commands are run from the `pocblock/` directory. You need **three terminals**.

### 1. Set your Anthropic API key

The agent server uses the Anthropic API (Claude) for all three AI agents. You must provide your API key via the `ANTHROPIC_API_KEY` environment variable.

```bash
# Option A: export it in your shell
export ANTHROPIC_API_KEY=sk-ant-your-key-here

# Option B: use a .env file
cp .env.example .env
# Edit .env and set ANTHROPIC_API_KEY=sk-ant-your-key-here
```

The server will exit with an error if this variable is not set.

### 2. Install dependencies

```bash
# Frontend (from pocblock/)
pnpm install

# Sync server
cd sync-server && npm install && cd ..

# Agent server (first build downloads crates)
cd agent-server && cargo build && cd ..
```

### 3. Start all three services

**Terminal 1 — Sync server (port 4444):**

```bash
cd pocblock/sync-server
bash start.sh
```

You should see: `Starting y-websocket on :4444 (callback → http://localhost:3001/on-doc-update)`

**Terminal 2 — Agent server (port 3001):**

```bash
cd pocblock/agent-server
cargo run
# Or, if you didn't export the key globally:
# ANTHROPIC_API_KEY=sk-ant-your-key-here cargo run
```

You should see: `Agent server listening on 0.0.0.0:3001`

If you see `ANTHROPIC_API_KEY environment variable is required`, set the key and try again.

**Terminal 3 — Frontend dev server (port 5173):**

```bash
cd pocblock
pnpm dev
```

You should see: `VITE vX.X.X ready in Xms → Local: http://localhost:5173/`

### 4. Open the editor

Navigate to **http://localhost:5173** in your browser. You'll see a BlockSuite editor with a header showing sync and agent connection status.

## Using the justfile (alternative)

If you have [just](https://github.com/casey/just) installed:

```bash
# Install all dependencies
just install

# Start all three services at once
just dev

# Run agent server tests
just test
```

## What to Try

1. **Type a paragraph** about a person or event (e.g. "Niko Matsakis presented his work on Rust's type system at RustConf 2025 in Portland.")
2. **Wait ~3-5 seconds** for agent notes to appear below your text
3. **Click "Accept"** to mark an agent suggestion as accepted, or **"Dismiss"** to delete it
4. **Open a second browser tab** at the same URL — see real-time sync between tabs
5. **Check the header** for sync and agent server connection status

## Three Agents

| Agent | Note Type | Color | Purpose |
|-------|-----------|-------|---------|
| Entity Detector | `entity` | Blue | Identifies people, places, organizations, events (schema.org types) |
| Summarizer | `summary` | Green | Generates a running one-paragraph document summary |
| Question Generator | `question` | Orange | Suggests follow-up discussion questions |

Each agent writes its output as a `sparkdown:agent-note` block — a custom BlockSuite block type with a distinctive visual style, confidence score, and accept/dismiss buttons.

## Configuration

Environment variables (set in `.env` or pass directly):

| Variable | Default | Description |
|----------|---------|-------------|
| `ANTHROPIC_API_KEY` | (required) | Your Anthropic API key |
| `AGENT_MODEL` | `claude-sonnet-4-5-20250514` | Model to use for all agents |
| `AGENT_PORT` | `3001` | Agent server listen port |
| `SYNC_URL` | `ws://localhost:4444` | y-websocket server URL |
| `DEBOUNCE_MS` | `800` | Agent re-run debounce time |
| `CONFIDENCE_THRESHOLD` | `0.6` | Minimum confidence for entity suggestions |

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `http://localhost:3001/health` | GET | Health check — returns `ok` |
| `http://localhost:3001/on-doc-update` | POST | Called by y-websocket on doc change (triggers agents) |
| `http://localhost:3001/run-agents` | POST | Manually trigger agent analysis |

## Project Structure

```
pocblock/
  src/                         # SvelteKit frontend
    lib/
      editor.ts                # BlockSuite editor setup (createEmptyDoc, mount)
      sync.ts                  # y-websocket provider connection
      blocks/
        agent-note-schema.ts   # sparkdown:agent-note block schema
        agent-note-component.ts # Lit web component (UI rendering)
        agent-note-service.ts  # Block service (accept/dismiss)
        agent-note-spec.ts     # BlockSpec wiring
        index.ts               # Re-exports + component registration
    routes/
      +page.svelte             # Main editor page with status header
      +layout.ts               # Disables SSR (BlockSuite needs DOM)
  sync-server/                 # y-websocket sync server
    package.json               # y-websocket dependency
    start.sh                   # Launch with env vars for callback
  agent-server/                # Rust agent server
    Cargo.toml                 # rig-core, yrs, axum, tokio deps
    src/
      main.rs                  # Axum server, provider init, spawn sync
      config.rs                # Config from env vars
      doc_bridge.rs            # Yrs Doc → DocumentView/BlockView structs
      doc_writer.rs            # Insert/clear agent note blocks in Yrs
      yjs_client.rs            # WebSocket Yjs sync protocol client
      routes.rs                # POST /on-doc-update, /run-agents handlers
      agents/
        mod.rs                 # Shared types: EntitySuggestion, Summary, Question
        entity_detector.rs     # Entity detection agent (rig-core)
        summarizer.rs          # Document summarizer agent
        question_generator.rs  # Discussion question agent
  justfile                     # Task runner (just dev, just test, etc.)
  .env.example                 # Template for environment variables
```

## Running Tests

```bash
# Rust agent server unit tests (doc_bridge, doc_writer)
cd agent-server && cargo test

# Frontend production build check
pnpm build
```

## Troubleshooting

- **Editor doesn't render**: Check the browser console for errors. BlockSuite requires DOM access — SSR must be disabled (handled by `+layout.ts`).
- **Sync not working**: Make sure the y-websocket server is running on port 4444 before starting the frontend or agent server.
- **Agent notes don't appear**: Check the agent server logs. Ensure `ANTHROPIC_API_KEY` is set. Try manually triggering: `curl -X POST http://localhost:3001/run-agents`
- **"Agents: disconnected" in header**: The agent server isn't reachable at port 3001. Check it's running and CORS isn't blocking the health check.
- **Infinite agent loop**: The pipeline hashes text content and skips re-runs when only agent notes changed. Check server logs for "Text unchanged, skipping agent run".
