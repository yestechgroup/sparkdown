# Sparkdown

A semantic markdown processor written in Rust. Sparkdown lets authors write clean, readable markdown while maintaining rich semantic annotations (RDF) in a separate overlay layer — keeping documents human-friendly and machine-readable at the same time.

## Key Features

- **Semantic Overlay Architecture** — Separates content from semantics using a three-layer design: Markdown layer (clean documents), Semantic layer (RDF metadata in `.sparkdown-sem` sidecar files), and Mapping layer (bidirectional index connecting the two).
- **Multiple Output Formats** — Render to HTML+RDFa, JSON-LD, or Turtle.
- **Built-in Ontology Support** — Ships with schema.org, Dublin Core, FOAF, and a custom `sd:` vocabulary. Extensible ontology registry.
- **AI-Agent Friendly** — Designed for collaborative authoring: humans write markdown, AI agents add semantic annotations via sidecar files, and both can review or adjust without cluttering the source.
- **Overlay Sync** — Intelligent diff-based sync engine that tracks anchor staleness and adjusts byte-span mappings when the markdown changes.
- **CLI Tooling** — Full-featured command-line interface for rendering, validation, extraction, and overlay management.

## Architecture

Sparkdown is organized as a Rust workspace with five crates:

| Crate | Purpose |
|---|---|
| `sparkdown-core` | Parsing pipeline (frontmatter → preprocess → pulldown-cmark → postprocess), AST types, annotation parsing, prefix maps |
| `sparkdown-ontology` | Ontology registry and validation, builtin providers |
| `sparkdown-render` | Output renderers: HTML+RDFa, JSON-LD, Turtle |
| `sparkdown-overlay` | Semantic overlay implementation: graph, anchoring, sidecar file parsing, sync engine |
| `sparkdown-cli` | Command-line interface built with clap |

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024)

### Build

```bash
cargo build
cargo build --release  # optimized build
```

### Test

```bash
cargo test               # run all tests
cargo test -p sparkdown-core  # test a specific crate
```

### Run

```bash
cargo run -p sparkdown-cli -- <COMMAND>
```

## CLI Usage

```
sparkdown <COMMAND>

Commands:
  render     Render a document to an output format (html, jsonld, turtle)
  validate   Validate semantic annotations
  extract    Extract RDF triples (turtle or jsonld)
  init       Initialize a new document with frontmatter template
  overlay    Manage semantic overlay sidecar files
```

### Examples

```bash
# Render a document to HTML
sparkdown render doc.md --format html

# Validate annotations
sparkdown validate doc.md

# Extract RDF triples as Turtle
sparkdown extract doc.md --format turtle

# Initialize a new article
sparkdown init my-article.md --doc-type article

# Create a sidecar overlay file
sparkdown overlay init doc.md

# Sync overlay after markdown edits
sparkdown overlay sync doc.md

# Check overlay status (stale/detached entities)
sparkdown overlay status doc.md

# Export sidecar to valid Turtle
sparkdown overlay export doc.md

# Migrate inline annotations to sidecar
sparkdown overlay import doc.md
```

## Technologies

- **Rust** (2024 edition) — core language
- **pulldown-cmark** — CommonMark parsing
- **oxrdf** — RDF data model
- **clap** — CLI argument parsing
- **serde / serde_json / serde_yaml_ng** — serialization
- **similar** — diffing for overlay sync
- **thiserror / anyhow** — error handling

## License

Apache 2.0 — see [LICENSE](LICENSE) for details.
