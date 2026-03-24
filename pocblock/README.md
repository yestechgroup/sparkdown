# Sparkdown Agent PoC

Proof-of-concept: AI agents collaboratively editing a BlockSuite document in real time.

## Architecture

```
SvelteKit (BlockSuite editor)
        ↕ WebSocket (Yjs CRDT sync)
y-websocket server (port 4444)
        ↕ WebSocket (Yjs sync protocol)
Rust Agent Server (Axum + rig-core + Yrs, port 3001)
```

- **Browser**: BlockSuite editor connected to y-websocket as a Yjs peer
- **Sync server**: Stock y-websocket server bridging all peers
- **Agent server**: Rust backend with rig-core LLM agents that read/write blocks via Yrs CRDT

## Prerequisites

- Node.js 20+
- Rust 1.85+ (2024 edition)
- pnpm
- An Anthropic API key

## Quick Start

```bash
cp .env.example .env
# Edit .env with your ANTHROPIC_API_KEY

# Terminal 1: sync server
cd sync-server && npm install && bash start.sh

# Terminal 2: agent server
cd agent-server && cargo run

# Terminal 3: frontend
pnpm install && pnpm dev
```

Open http://localhost:5173 and start typing.

## What to Try

1. Write a paragraph about a person or event
2. Wait ~3-5 seconds for agent notes to appear
3. Click "Accept" or "Dismiss" on agent notes
4. Open a second browser tab — see real-time sync

## Three Agents

| Agent | Output | Purpose |
|-------|--------|---------|
| Entity Detector | `sparkdown:agent-note` (entity) | Identifies people, places, orgs, events |
| Summarizer | `sparkdown:agent-note` (summary) | Running document summary |
| Question Generator | `sparkdown:agent-note` (question) | Suggests follow-up questions |

## Project Structure

```
pocblock/
  src/                     # SvelteKit frontend
    lib/
      editor.ts            # BlockSuite editor setup
      sync.ts              # y-websocket provider
      blocks/              # Custom agent-note block
    routes/
      +page.svelte         # Main editor page
      +layout.ts           # SSR disabled
  sync-server/             # y-websocket sync server
    start.sh               # Launch script with env vars
  agent-server/            # Rust agent server
    src/
      main.rs              # Axum server, provider init
      config.rs            # Env-based configuration
      doc_bridge.rs        # Yrs → DocumentView decoder
      doc_writer.rs        # Agent note block insertion
      yjs_client.rs        # WebSocket sync client
      routes.rs            # HTTP handlers
      agents/              # rig-core agent implementations
```

## Running Tests

```bash
# Rust agent server tests
cd agent-server && cargo test

# Frontend build check
pnpm build
```
