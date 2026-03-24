# Sparkdown


# User journey

A user may choose to use sparkdown in a variety of ways.

1. Transcribe a meeting
    1.1 Transcription just listens, calls a speech-to-text api, for example Deepgram, and replies with text.
    1.2 LLM agent suggests points to discuss. A button along side gives options of "Go deeper" which in turn asks the LLM agent to come back with even more related questions, or "Done", meaning that line of questioning is complete.

2. Writing, journaling, documentation
    2.1 Writing a specific type of documentation where the user explicity selects on ore more ontologies
    2.2 Writing and in the background LLM agents suggest ontologies 
    2.3 Writing documentation in a specific format and LLM agents contribute using https://github.com/taylordotfish/eips

3. Researching
    3.1 user can search using human interface / LLM improved interace to sparql using https://grafeo.dev/user-guide/

4. Ontology selection

Styled similar to an appstore, a user can turn on and off ontologies that they want avaiable. 



## Components.

We want to leverage all of the componets on https://blocksuite.io/components/overview.html including:

https://blocksuite.io/components/blocks/database-block.html

packages/backend/server/src/plugins/copilot/resolver.ts

blocksuite.io also provides a frame, a side panel, perfect for showing related semantic information, links, questions, active LLM agents etc. 

# Data management

The user should always be able to write, unobstructed, in markdown.

If the user wants to export data, they can do that with the  semantic annotations. See https://blog.sparna.fr/2020/02/20/semantic-markdown/
This allows import and export of single files in a transparent format. 

However, for ease of internal operations, we should use grafeo. This will make searching and other operations faster. 


## Text editing interface

https://blocksuite.io/guide/block-spec.html


# Ambient UI features

## AI Agents
Here is some Rust based AI agent orchestration frameworks. 

There are now several Rust crates and projects focused specifically on LLM agent orchestration and multi‑agent workflows. [dasroot](https://dasroot.net/posts/2026/02/rust-libraries-llm-orchestration-2026/)

## Notable orchestration-focused crates

- **CloudLLM** (`cloudllm` on crates.io) – “batteries‑included” toolkit for intelligent agents with built‑in multi‑agent orchestration modes (Parallel, RoundRobin, Moderated, Hierarchical, Debate), multi‑provider support (OpenAI, Anthropic, etc.), and durable memory via MentisDB. [lib](https://lib.rs/crates/cloudllm)  
- **swarms-rs** (`swarms-rs`) – positions itself as an enterprise‑grade, production multi‑agent orchestration framework in Rust, with workflow abstractions like `ConcurrentWorkflow` and an agent builder around LLM providers (OpenAI, DeepSeek, etc.). [lib](https://lib.rs/crates/swarms-rs)  
- **llm-agent-runtime** (`llm-agent-runtime`) – async runtime that combines orchestration primitives, episodic/semantic memory, an in‑memory knowledge graph, and a ReAct loop, consolidating several lower‑level crates into one Tokio‑based agent runtime. [lib](https://lib.rs/crates/llm-agent-runtime)  
- **AutoAgents** (`autoagents`) – an “Agent Framework for Building Autonomous Agents” on lib.rs; aimed at defining and running autonomous agents, suitable as a starting point for building orchestrated systems. [lib](https://lib.rs/crates/autoagents)  
- **rs-graph-llm / graph-flow** – GitHub project providing a high‑performance, type‑safe multi‑agent workflow framework in Rust, focused on graph‑based, stateful AI agent orchestration. [github](https://github.com/a-agmon/rs-graph-llm)  

### Example capabilities (CloudLLM & swarms-rs)

- CloudLLM exposes an `Orchestration` object where you add multiple agents (each wrapping an LLM client) and choose an `OrchestrationMode` like RoundRobin or Hierarchical, then call `.run(prompt, max_iterations)` to coordinate the agents until tasks complete. [lib](https://lib.rs/crates/cloudllm)  
- swarms-rs lets you define several specialized agents (e.g., “Market Analysis Agent”, “Trade Strategy Agent”, “Risk Assessment Agent”) and plug them into a `ConcurrentWorkflow` that runs them concurrently on a shared task, returning structured JSON results. [lib](https://lib.rs/crates/swarms-rs)  

### Other relevant ecosystem pieces

- Articles like “Rust Libraries for LLM Orchestration in 2026” survey these crates and highlight graph‑based workflows and distributed execution as an emerging pattern in the Rust LLM orchestration space. [dasroot](https://dasroot.net/posts/2026/02/rust-libraries-llm-orchestration-2026/)  
- Zectonal published an architecture write‑up describing their in‑house Rust “agentic framework” for multimodal data quality monitoring, using multiple specialized agents and multi‑provider LLM support; while not a reusable crate, it’s a concrete reference design. [zenml](https://www.zenml.io/llmops-database/building-a-rust-based-ai-agentic-framework-for-multimodal-data-quality-monitoring)  

### Quick comparison

| Project / crate      | Focus                                | Key features (orchestration)                                      |
|----------------------|--------------------------------------|-------------------------------------------------------------------|
| `cloudllm`           | Agent toolkit & orchestration        | Multiple orchestration modes, multi‑provider, durable memory. [lib](https://lib.rs/crates/cloudllm) |
| `swarms-rs`          | Enterprise multi‑agent workflows     | Concurrent workflows, agent builder, tracing/logging. [lib](https://lib.rs/crates/swarms-rs)      |
| `llm-agent-runtime`  | Unified agent runtime                | ReAct loop, memory, knowledge graph, Tokio orchestration. [lib](https://lib.rs/crates/llm-agent-runtime)  |
| `autoagents`         | Autonomous agents framework          | Agent definitions and execution primitives. [lib](https://lib.rs/crates/autoagents)                |
| `rs-graph-llm`       | Graph-based multi‑agent workflows    | Type‑safe graph workflows, high‑performance orchestration. [github](https://github.com/a-agmon/rs-graph-llm) |



