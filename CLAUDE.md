# CLAUDE.md

Project guidance for Claude Code sessions working on Sparkdown.

## Project Overview

Sparkdown is a semantic markdown processor in Rust. It parses markdown with semantic annotations and renders to HTML+RDFa, JSON-LD, or Turtle. The key architectural concept is the **semantic overlay** — separating clean markdown from RDF metadata using sidecar files (`.sparkdown-sem`).

## Build & Test Commands

```bash
cargo build                    # build all crates
cargo test                     # run all tests
cargo test -p sparkdown-core   # test a specific crate
cargo run -p sparkdown-cli -- <COMMAND>  # run the CLI
```

All five crates must compile and pass tests before committing.

## Workspace Structure

```
crates/
  sparkdown-core/       # Parsing, AST, annotations, frontmatter, prefix maps
  sparkdown-ontology/   # Ontology registry (schema.org, Dublin Core, FOAF, sd:)
  sparkdown-render/     # Renderers: HTML+RDFa, JSON-LD, Turtle
  sparkdown-overlay/    # Semantic overlay: graph, anchors, sidecar, sync engine
  sparkdown-cli/        # CLI (clap-based): render, validate, extract, init, overlay
```

## Code Conventions

- Rust 2024 edition.
- Use `thiserror` for library error types in crates; `anyhow` for CLI error handling.
- Each crate exposes a public API via `lib.rs`; keep internal modules private where possible.
- Serde derives for types that cross serialization boundaries.
- Tests live in `#[cfg(test)] mod tests` blocks within source files, plus integration tests in `tests/`.

## Key Concepts

- **Sidecar file** (`.sparkdown-sem`): Turtle-inspired format storing semantic entities with byte-span anchors mapping them back to the markdown source.
- **Anchor staleness**: When markdown is edited, the sync engine (`sparkdown-overlay/src/sync.rs`) diffs the old and new text, adjusts anchors, and marks entities as stale if their anchored text changed.
- **Three modes**: Legacy (inline annotations only), Overlay (sidecar only), Hybrid (both). Mode is declared in YAML frontmatter.
- **Prefix maps**: Namespace prefixes (e.g., `schema:`, `dc:`) are declared in frontmatter and resolved by `sparkdown-core`.

## Design Spec

The full architecture spec lives at `docs/superpowers/specs/2026-03-21-semantic-overlay-design.md`.
