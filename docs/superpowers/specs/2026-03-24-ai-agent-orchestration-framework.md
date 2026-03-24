# Sparkdown Studio: AI Agent Orchestration Framework

> Subsystem 1 — the foundational layer that powers all intelligent features in Sparkdown Studio: ambient writing assistance, transcription, semantic search, and ontology recommendations.
> Companion to: `2026-03-21-sparkdown-studio-phase2-design.md` (Phase 1.5), `2026-03-21-sparkdown-studio-ui-design.md` (UI vision).
> GitHub issue: [#6](https://github.com/yestechgroup/sparkdown/issues/6)

---

## 1. Crate Evaluation Framework

Before committing to build-vs-buy, we conducted a structured evaluation of six Rust LLM orchestration crates. The goal was to learn what patterns exist, what works, and what Sparkdown should adopt or build.

### 1.1 Evaluation Criteria

Each crate was scored against six criteria weighted for Sparkdown Studio's context — a Tauri 2 desktop app with an existing tokio runtime, Svelte 5 frontend, and session-actor architecture.

| # | Criterion | Weight | What we measured |
|---|-----------|--------|-----------------|
| 1 | **Integration fit** | Critical | Can it run inside Tauri's tokio runtime without owning the event loop? Does it conflict with our session-actor model? |
| 2 | **Composability** | High | Can we use only what we need (e.g., just the provider layer), or is it all-or-nothing? |
| 3 | **Streaming support** | High | Token-by-token streaming to the frontend via Tauri events? |
| 4 | **Maturity & maintenance** | High | Release cadence, contributor count, download volume, API stability |
| 5 | **Dependency footprint** | Medium | What does it drag in? Binary size and compile time matter for a desktop app |
| 6 | **Extensibility** | Medium | Can we define custom agent types (entity detector, ontology suggester) without fighting the framework? |

### 1.2 Candidates Evaluated

| Crate | Version | Stars | Downloads/mo | Contributors | Last active |
|-------|---------|-------|-------------|-------------|-------------|
| **rig-core** | 0.33.0 | ~6,600 | ~116,000 | ~180 | 2026-03-17 (1 week ago) |
| **autoagents** | 0.3.6 | 469 | — | ~10 | Active |
| **cloudllm** | 0.14.0 | 24 | ~600 | 1 | 2026-03-18 |
| **graph-flow** | 0.4.0 | 267 | — | 1 | 2025-09 (6 months ago) |
| **swarms-rs** | 0.2.1 | 135 | ~28 | 1 (+4 minor) | 2025-10 (5 months ago) |
| **llm-agent-runtime** | 1.74.0 | 3 | ~10 | 1 | 2026-03-20 (4 days old) |

### 1.3 Detailed Findings

#### rig-core (0xPlaygrounds) — RECOMMENDED

**Strengths:**
- **20 LLM providers** built in (OpenAI, Anthropic, Gemini, Ollama, DeepSeek, Groq, Mistral, etc.)
- **First-class streaming** with `StreamingPrompt`, `StreamingChat`, `PauseControl`
- **Structured output** via `schemars` — `agent.prompt_typed::<T>()` returns deserialized Rust types
- **Tool trait** with compile-time type safety (associated types for args, output, error)
- **Agent-as-tool** composition — agents can call other agents as tools
- **Pipeline module** (DAG-based) inspired by Airflow/Dagster with `Op` trait, `chain()`, `parallel!`
- **Ollama + Llamafile** for fully offline local inference
- **Minimal tokio requirements** (`rt` + `sync` only, not `full`) — plays well with Tauri's runtime
- **10 vector store integrations** via companion crates (MongoDB, SQLite, Qdrant, etc.)
- **WASM support** for potential frontend-side inference
- **Real adoption:** 116K downloads/month, 163 dependent crates, used by St. Jude Children's Research Hospital, Neon, Nethermind
- **Active maintenance:** 180 contributors, weekly releases

**Concerns:**
- **API instability:** 32 breaking changes across 50 releases (pre-1.0). Must pin versions carefully.
- **37% docs.rs coverage.** Will need to read source for advanced usage.
- **Tool trait not dyn-compatible** (associated types + `impl Future`). Needs wrapper types for heterogeneous tool collections.
- **All 20 providers compiled in by default** (2.5MB package). No feature gates to exclude unused ones.

**Verdict:** Best candidate by a wide margin. The only crate with real community adoption, active multi-contributor maintenance, and an architecture that fits Tauri's async model. API churn is manageable with version pinning.

#### autoagents — NOT RECOMMENDED (architectural conflict)

**Strengths:** Most mature community (469 stars), full ReAct executor, WASM-sandboxed tools, OpenTelemetry observability, 10+ providers.

**Fatal flaw:** Monolithic architecture with its own actor system (ractor), vector store, embedding pipeline, and document readers. Cannot opt out of subsystems. The actor framework **directly conflicts** with Tauri's async command system — two event loops, two state management systems. Would require extensive adaptation.

#### cloudllm — NOT RECOMMENDED (coupling + single maintainer)

**Strengths:** 7 orchestration modes (Parallel, RoundRobin, Moderated, Hierarchical, Debate, Ralph, AnthropicAgentTeams). Rich concept.

**Fatal flaws:** Mandatory `mentisdb` dependency even if unused. `tokio` with `full` features required. 48 releases with unstable API. Single maintainer, 24 stars. The orchestration modes are interesting to study but the coupling makes it unsuitable.

#### graph-flow — NOT RECOMMENDED AS DEPENDENCY (but design pattern is valuable)

**Strengths:** Clean `Task` trait + `NextAction` enum (Continue, WaitForInput, GoTo, End) maps naturally to Tauri IPC. `GraphBuilder` for DAG definition. Focused, ~500-800 lines of core logic.

**Fatal flaws:** Single maintainer, 6 months without code changes. sqlx (Postgres) compiled even for in-memory usage. Pre-1.0.

**Recommendation:** Don't depend on the crate, but adopt the `Task` + `NextAction` pattern in our own orchestration layer. The core idea is ~400 lines to implement.

#### swarms-rs — NOT RECOMMENDED

**Strengths:** DAGWorkflow with petgraph, MCP protocol support via rmcp.

**Fatal flaws:** Single maintainer (Kye Gomez), 5 months inactive, 33 open PRs being ignored, 37% docs, bloated deps (dual HTTP stacks, dual logging frameworks, `dotenv` + `zstd` in a library), 28 downloads/month. Marketing ("Enterprise-Grade Production-Ready") does not match reality.

#### llm-agent-runtime — NOT RECOMMENDED (not production-ready)

**Strengths:** Interesting concept (ReAct + memory + knowledge graph + orchestrator in one crate).

**Fatal flaws:** **6 days old.** Jumped from v1.0.0 to v1.74.0 in 2 days. 34K lines likely AI-generated in bulk. 52 total downloads, 3 stars. No LLM integration built in — you must implement `LlmProvider` yourself. Interesting to study as a reference design but absolutely not adoption-ready.

### 1.4 Comparative Scorecard

| Criterion | rig-core | autoagents | cloudllm | graph-flow | swarms-rs | llm-agent-runtime |
|-----------|----------|------------|----------|------------|-----------|-------------------|
| Integration fit | **A** | D | C | B | D | C |
| Composability | **B+** | D | D | **A** | D | B |
| Streaming | **A** | B | B | N/A | D | B |
| Maturity | **B+** | B | D | C | D | **F** |
| Dep footprint | **B** | D | D | B | D | B |
| Extensibility | **A** | C | C | **A** | C | B |
| **Overall** | **A-** | C | D+ | C+ | D | F |

### 1.5 Decision

**Primary foundation: `rig-core`** for LLM provider abstraction, agent building, tool calling, streaming, and structured output.

**Patterns to adopt from others:**
- `graph-flow`'s `Task` + `NextAction` enum for step-by-step workflow execution with human-in-the-loop
- `cloudllm`'s orchestration mode taxonomy (Parallel, Moderated, Hierarchical) as conceptual design — implemented over rig agents, not using cloudllm itself

**Build ourselves:**
- Agent lifecycle management (tied to Tauri's `DocumentSession`)
- Orchestration layer (scheduling, debouncing, result aggregation)
- Message protocol between agents and the Svelte frontend
- The "observer agent" that writes to `sparkdown-overlay`

---

## 2. Architecture

### 2.1 System Context

```
┌─────────────────────────────────────────────────────────┐
│ Sparkdown Studio (Tauri 2)                              │
│                                                         │
│  ┌─────────────┐    IPC     ┌────────────────────────┐  │
│  │ Svelte 5    │◄──events──►│ Tauri Backend          │  │
│  │ Frontend    │            │                        │  │
│  │             │            │  ┌──────────────────┐  │  │
│  │ - Editor    │            │  │ DocumentSession  │  │  │
│  │ - Suggest.  │            │  │ (existing actor) │  │  │
│  │   tray      │            │  └────────┬─────────┘  │  │
│  │ - Whisper   │            │           │             │  │
│  │   cards     │            │  ┌────────▼─────────┐  │  │
│  │ - Gutter    │            │  │ AgentOrchestrator│  │  │
│  │             │            │  │ (new)            │  │  │
│  └─────────────┘            │  └────────┬─────────┘  │  │
│                             │           │             │  │
│                             │  ┌────────▼─────────┐  │  │
│                             │  │ rig-core agents  │  │  │
│                             │  │ (entity, onto,   │  │  │
│                             │  │  discussion, NL) │  │  │
│                             │  └────────┬─────────┘  │  │
│                             │           │             │  │
│                             │  ┌────────▼─────────┐  │  │
│                             │  │ LLM Providers    │  │  │
│                             │  │ (Anthropic,      │  │  │
│                             │  │  OpenAI, Ollama) │  │  │
│                             │  └──────────────────┘  │  │
│                             └────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 2.2 Key Components

#### AgentOrchestrator

The central coordinator. Lives alongside `DocumentSession` in the Tauri backend. Responsible for:

- **Agent lifecycle:** Creating, configuring, and tearing down agent instances per document session
- **Scheduling:** Debounced invocation — waits N ms after typing stops before running agents
- **Workflow execution:** Runs agents in configured patterns (parallel entity+ontology scan, sequential refinement)
- **Result aggregation:** Collects suggestions from multiple agents, deduplicates, ranks by confidence
- **Flow control:** Implements `NextAction`-style step execution for human-in-the-loop workflows (transcription "Go deeper" / "Done")

```rust
pub struct AgentOrchestrator {
    /// Active agent workflows per document
    workflows: HashMap<DocumentId, Vec<WorkflowHandle>>,
    /// Shared rig provider clients
    providers: Arc<ProviderRegistry>,
    /// Configuration (debounce timing, confidence thresholds, etc.)
    config: OrchestratorConfig,
    /// Channel to send suggestions to the frontend
    event_tx: mpsc::Sender<AgentEvent>,
}
```

#### ProviderRegistry

Manages LLM provider clients. Wraps rig-core's provider initialization with Sparkdown-specific concerns: API key storage, provider selection preferences, fallback chains.

```rust
pub struct ProviderRegistry {
    /// Named provider clients, initialized lazily
    clients: HashMap<String, ProviderClient>,
    /// User preference for default provider
    default_provider: String,
    /// Fallback chain: if primary fails, try secondary
    fallback_chain: Vec<String>,
}

pub enum ProviderClient {
    OpenAI(rig::providers::openai::Client),
    Anthropic(rig::providers::anthropic::Client),
    Ollama(rig::providers::ollama::Client),
}
```

#### Agent Types

Four specialized agents, each built on rig-core's `Agent`:

| Agent | Input | Output | Trigger |
|-------|-------|--------|---------|
| **EntityDetector** | Markdown paragraph(s) | `Vec<EntitySuggestion>` | Debounced on text change |
| **OntologySuggester** | Document content + active ontologies | `Vec<OntologySuggestion>` | On document open, periodic |
| **DiscussionAgent** | Transcript text | `Vec<DiscussionPoint>` | During transcription |
| **NLQueryAgent** | Natural language query | SPARQL string | On search bar input |

Each agent is a rig-core `Agent` with a typed preamble, tools, and structured output:

```rust
// Example: EntityDetector returns structured suggestions
#[derive(Deserialize, JsonSchema)]
pub struct EntitySuggestion {
    pub text_span: String,
    pub entity_type: String,       // e.g., "schema:Person"
    pub confidence: f64,           // 0.0 - 1.0
    pub properties: Vec<PropertySuggestion>,
    pub reasoning: String,         // why this was identified
}

// Built with rig
let entity_agent = provider
    .agent("claude-sonnet-4-5-20250514")
    .preamble(ENTITY_DETECTOR_PROMPT)
    .temperature(0.3)
    .build();

let suggestions: Vec<EntitySuggestion> = entity_agent
    .prompt_typed(&format!("Analyze this text:\n\n{paragraph}"))
    .await?;
```

#### Workflow Patterns

Inspired by graph-flow's `NextAction` and cloudllm's orchestration modes:

```rust
/// What happens after an agent step completes
pub enum NextAction {
    /// Emit results and wait for next trigger (e.g., text change)
    Idle,
    /// Run the next agent in the pipeline
    Continue(AgentId),
    /// Pause and wait for user input ("Go deeper" / "Done")
    WaitForInput { prompt: String },
    /// Branch to a different agent based on a condition
    Branch { condition: String, targets: Vec<(String, AgentId)> },
}

/// Orchestration patterns
pub enum WorkflowPattern {
    /// Run agents simultaneously, merge results
    Parallel(Vec<AgentId>),
    /// Run agents in sequence, each receives prior output
    Sequential(Vec<AgentId>),
    /// One agent reviews another's output before emitting
    Moderated { workers: Vec<AgentId>, moderator: AgentId },
}
```

### 2.3 Message Protocol

Events flow from agents to the frontend via Tauri's event system:

```rust
/// Events emitted to the Svelte frontend
#[derive(Serialize, Clone)]
#[serde(tag = "type")]
pub enum AgentEvent {
    /// New entity suggestion ready for display
    EntitySuggestion {
        doc_id: DocumentId,
        suggestions: Vec<EntitySuggestion>,
    },
    /// Ontology recommendation
    OntologySuggestion {
        doc_id: DocumentId,
        ontology_id: String,
        reason: String,
    },
    /// Discussion point during transcription
    DiscussionPoint {
        doc_id: DocumentId,
        point: DiscussionPoint,
    },
    /// Agent started/finished working (for UI loading indicators)
    AgentStatus {
        doc_id: DocumentId,
        agent_id: String,
        status: AgentStatusKind,
    },
    /// Streaming token (for real-time display)
    StreamingToken {
        doc_id: DocumentId,
        agent_id: String,
        token: String,
    },
}

#[derive(Serialize, Clone)]
pub enum AgentStatusKind {
    Thinking,
    Streaming,
    Complete,
    Error(String),
}
```

Commands flow from the frontend to agents via Tauri IPC:

```rust
#[tauri::command]
async fn accept_suggestion(doc_id: DocumentId, suggestion_id: String) -> Result<(), Error>;

#[tauri::command]
async fn dismiss_suggestion(doc_id: DocumentId, suggestion_id: String) -> Result<(), Error>;

#[tauri::command]
async fn expand_suggestion(doc_id: DocumentId, suggestion_id: String) -> Result<(), Error>;

#[tauri::command]
async fn set_active_ontologies(doc_id: DocumentId, ontology_ids: Vec<String>) -> Result<(), Error>;

#[tauri::command]
async fn configure_providers(config: ProviderConfig) -> Result<(), Error>;
```

### 2.4 Integration with Existing Architecture

The orchestrator integrates with Sparkdown's existing systems:

- **DocumentSession** (existing): Owns the document state. The orchestrator subscribes to text-change events from the session and emits suggestions back. The session writes accepted suggestions to the overlay.
- **sparkdown-overlay** (existing): The `accept_suggestion` command writes entities to the `.sparkdown-sem` sidecar via the overlay's graph API.
- **sparkdown-ontology** (existing): Agent prompts include active ontology type information so suggestions use the correct vocabulary.
- **Tauri event bus** (existing): All agent→frontend communication uses `app_handle.emit()`. No new transport needed.

---

## 3. Proposed Crate Structure

```
crates/
  sparkdown-agents/
    Cargo.toml
    src/
      lib.rs                  # Public API
      orchestrator.rs         # AgentOrchestrator
      providers.rs            # ProviderRegistry
      workflow.rs             # WorkflowPattern, NextAction
      events.rs               # AgentEvent, AgentStatusKind
      config.rs               # OrchestratorConfig, debounce settings
      agents/
        mod.rs
        entity_detector.rs    # EntityDetector agent
        ontology_suggester.rs # OntologySuggester agent
        discussion.rs         # DiscussionAgent (for transcription)
        nl_query.rs           # NLQueryAgent (NL → SPARQL)
      prompts/
        mod.rs
        entity.rs             # System prompts for entity detection
        ontology.rs           # System prompts for ontology suggestion
        discussion.rs         # System prompts for discussion points
        query.rs              # System prompts for NL → SPARQL
```

**Cargo.toml dependencies:**

```toml
[dependencies]
rig-core = "0.33"
sparkdown-core = { path = "../sparkdown-core" }
sparkdown-ontology = { path = "../sparkdown-ontology" }
sparkdown-overlay = { path = "../sparkdown-overlay" }
tokio = { version = "1", features = ["rt", "sync", "time", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"
thiserror = "2"
tracing = "0.1"
```

---

## 4. Implementation Plan

### Phase A: Foundation (rig integration + provider registry)

1. Create `crates/sparkdown-agents/` workspace member
2. Implement `ProviderRegistry` with support for Anthropic, OpenAI, and Ollama
3. Build a minimal `AgentOrchestrator` that can create and run a single rig agent
4. Wire into Tauri: add `configure_providers` command, test round-trip from frontend
5. **Deliverable:** Can send a prompt from the Svelte frontend, get a response displayed

### Phase B: Entity Detection Agent

1. Write the `EntityDetector` agent with structured output (`Vec<EntitySuggestion>`)
2. Implement debounced triggering from `DocumentSession` text-change events
3. Emit `AgentEvent::EntitySuggestion` via Tauri events
4. Frontend: display suggestions in the existing suggestion tray
5. Implement `accept_suggestion` → write to sidecar via overlay
6. **Deliverable:** Type markdown, see entity suggestions appear, accept them into the sidecar

### Phase C: Streaming + Orchestration Patterns

1. Add streaming support: `StreamingToken` events to frontend
2. Implement `WorkflowPattern::Parallel` — run EntityDetector + OntologySuggester concurrently
3. Implement `NextAction::WaitForInput` for "Go deeper" / "Done" interaction
4. **Deliverable:** Multiple agents run in parallel, results merge in the suggestion tray

### Phase D: Remaining Agents

1. `OntologySuggester` — recommends ontologies based on document content
2. `DiscussionAgent` — suggests discussion points during transcription (Subsystem 3 integration)
3. `NLQueryAgent` — translates natural language to SPARQL (Subsystem 4 integration)

---

## 5. Testing Strategy

### 5.1 Unit Tests

Each agent module includes unit tests with **mock LLM responses**. rig-core's `CompletionModel` trait can be implemented as a test double that returns canned responses:

```rust
#[cfg(test)]
mod tests {
    struct MockProvider {
        response: String,
    }

    // Implement rig's CompletionModel for MockProvider
    // to return deterministic responses in tests

    #[tokio::test]
    async fn entity_detector_finds_person() {
        let mock = MockProvider::new(json!([{
            "text_span": "Niko Matsakis",
            "entity_type": "schema:Person",
            "confidence": 0.95,
            "properties": [],
            "reasoning": "Proper noun, known Rust contributor"
        }]));

        let detector = EntityDetector::new(mock);
        let suggestions = detector.analyze("Niko Matsakis presented at RustConf.").await.unwrap();

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].entity_type, "schema:Person");
        assert!(suggestions[0].confidence > 0.9);
    }

    #[tokio::test]
    async fn entity_detector_handles_empty_text() {
        let mock = MockProvider::new(json!([]));
        let detector = EntityDetector::new(mock);
        let suggestions = detector.analyze("").await.unwrap();
        assert!(suggestions.is_empty());
    }
}
```

**What we test at this level:**
- Agent prompt construction (correct system prompt, context injection)
- Response parsing (structured output deserialization)
- Edge cases (empty input, malformed LLM response, timeout)
- Confidence threshold filtering
- Deduplication logic

### 5.2 Integration Tests

Tests that exercise the full pipeline from orchestrator to overlay, using mock LLM providers but real Sparkdown crates:

```
tests/
  orchestrator_integration.rs   # AgentOrchestrator + DocumentSession + Overlay
  workflow_patterns.rs          # Parallel, Sequential, Moderated execution
  provider_fallback.rs          # Primary provider fails → fallback engages
  debounce.rs                   # Rapid text changes → single agent invocation
```

**Key integration scenarios:**

| Test | What it verifies |
|------|------------------|
| `accept_writes_to_sidecar` | Accepting an entity suggestion creates a valid entry in the `.sparkdown-sem` sidecar |
| `dismiss_persists_across_session` | Dismissed suggestions don't reappear on next agent run |
| `parallel_merge_deduplicates` | Two agents suggesting the same entity → one suggestion |
| `debounce_coalesces_rapid_edits` | 10 keystrokes in 200ms → 1 agent invocation (not 10) |
| `provider_fallback_on_error` | Primary provider returns 500 → fallback provider handles request |
| `stale_anchor_triggers_rescan` | Editing text under an existing entity → agent re-evaluates |
| `streaming_emits_tokens` | Streaming agent emits `StreamingToken` events in order |
| `workflow_wait_for_input` | Workflow pauses at `WaitForInput`, resumes on user action |

### 5.3 Prompt Regression Tests

LLM outputs are non-deterministic, but prompt quality can be regression-tested:

```rust
#[test]
fn entity_prompt_includes_ontology_context() {
    let prompt = EntityDetector::build_prompt(
        "Some text",
        &["schema.org", "foaf"],
    );
    assert!(prompt.contains("schema:Person"));
    assert!(prompt.contains("foaf:Person"));
    assert!(prompt.contains("Return JSON"));
}

#[test]
fn entity_prompt_handles_no_ontologies() {
    let prompt = EntityDetector::build_prompt("Some text", &[]);
    assert!(prompt.contains("common ontologies"));
    // Should fall back to suggesting ontologies, not crash
}
```

### 5.4 End-to-End Tests (Tauri)

Tauri provides a test harness for IPC commands. These tests verify the full path from frontend command to backend response:

```rust
#[cfg(test)]
mod e2e {
    use tauri::test::{mock_builder, MockRuntime};

    #[tokio::test]
    async fn configure_providers_roundtrip() {
        let app = mock_builder().build().unwrap();
        // Call configure_providers IPC command
        // Verify provider registry is updated
        // Call a simple agent prompt
        // Verify response arrives via event
    }
}
```

### 5.5 Manual / Exploratory Testing

For LLM-dependent behavior that cannot be deterministically tested:

| Scenario | What to verify | How |
|----------|---------------|-----|
| Entity detection quality | Agent finds meaningful entities, not noise | Open sample markdown docs, review suggestions |
| Ontology relevance | Suggested ontologies match document domain | Write a recipe → expect schema:Recipe suggestion |
| Confidence calibration | High-confidence = correct, low = speculative | Review 50+ suggestions, check correlation |
| Streaming UX | Tokens appear smoothly, no jank | Trigger agent, observe suggestion tray |
| Provider switching | Switching from Anthropic to Ollama works seamlessly | Change provider in settings, verify agent still works |
| Offline mode | Ollama/Llamafile agents work without internet | Disconnect network, verify local agent responds |

### 5.6 Performance Benchmarks

```rust
// benches/agent_throughput.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_debounce_coalescing(c: &mut Criterion) {
    // Measure: time from last keystroke to agent invocation
    // Target: < 500ms debounce + < 200ms overhead
}

fn bench_suggestion_dedup(c: &mut Criterion) {
    // Measure: deduplication of 100 suggestions from 3 parallel agents
    // Target: < 5ms
}

fn bench_sidecar_write(c: &mut Criterion) {
    // Measure: time to write accepted entity to sidecar
    // Target: < 50ms (must feel instant)
}
```

**Performance targets:**

| Metric | Target | Rationale |
|--------|--------|-----------|
| Debounce-to-suggestion latency | < 3s | Must feel responsive but not jarring |
| Suggestion tray update | < 16ms | Must not drop frames |
| Sidecar write on accept | < 50ms | Must feel instant |
| Memory per active agent | < 10MB | Desktop app, not a server |
| Concurrent agents per doc | 3-5 | Parallel entity + ontology + discussion |

---

## 6. Configuration

User-facing configuration stored in Sparkdown Studio settings:

```toml
# ~/.config/sparkdown-studio/agents.toml

[providers.anthropic]
api_key = "sk-ant-..."        # Or reference to system keychain
default_model = "claude-sonnet-4-5-20250514"

[providers.openai]
api_key = "sk-..."
default_model = "gpt-4o"

[providers.ollama]
base_url = "http://localhost:11434"
default_model = "llama3.1:8b"

[orchestrator]
default_provider = "anthropic"
fallback_provider = "ollama"
debounce_ms = 800              # Wait after typing stops
max_concurrent_agents = 3
confidence_threshold = 0.6     # Hide suggestions below this

[agents.entity_detector]
enabled = true
temperature = 0.3
max_tokens = 2048

[agents.ontology_suggester]
enabled = true
temperature = 0.5
trigger = "on_open"            # "on_open", "periodic", "manual"
period_seconds = 300

[agents.discussion]
enabled = false                # Only active in transcription mode
temperature = 0.7
```

---

## 7. Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| rig-core breaking changes on upgrade | Medium | Pin exact version (`=0.33.0`), upgrade deliberately with changelog review |
| LLM response quality varies by provider | Medium | Prompt regression tests, provider-specific prompt tuning |
| Agent costs (API usage) in a desktop app | Medium | Default to Ollama (free, local), show cost estimates in UI, usage caps |
| Streaming backpressure (slow frontend) | Low | rig's `PauseControl`, bounded channels between backend and frontend |
| LLM hallucinations in entity detection | Medium | Confidence scoring, user review before accepting, "verify" action |
| Latency on first agent invocation (cold start) | Low | Lazy provider init, preload on document open |

---

## 8. Open Questions

1. **API key storage:** System keychain (OS-native) vs encrypted config file vs environment variables?
2. **Cost tracking:** Should we track and display per-session LLM API costs?
3. **Prompt versioning:** How do we version and A/B test prompt templates?
4. **Agent memory across sessions:** Should agents remember dismissed suggestions / user preferences across document sessions?
5. **MCP integration:** rig-core supports MCP via `rmcp`. Should agents expose their capabilities as MCP tools for external consumption?

---

## 9. References

- [rig-core on crates.io](https://crates.io/crates/rig-core) — v0.33.0
- [rig GitHub](https://github.com/0xPlaygrounds/rig) — 6.6K stars, 180 contributors
- [graph-flow](https://github.com/a-agmon/rs-graph-llm) — `Task` + `NextAction` pattern reference
- [cloudllm orchestration modes](https://lib.rs/crates/cloudllm) — conceptual reference for Parallel/Moderated/Hierarchical
- [Sparkdown overlay design](docs/superpowers/specs/2026-03-21-semantic-overlay-design.md)
- [Sparkdown Studio UI design](docs/superpowers/specs/2026-03-21-sparkdown-studio-ui-design.md)
- [Sparkdown Studio Phase 1.5](docs/superpowers/specs/2026-03-21-sparkdown-studio-phase2-design.md)
