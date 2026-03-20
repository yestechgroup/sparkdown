# Semantic Overlay Architecture

**Date:** 2026-03-21
**Status:** Draft
**Scope:** New `sparkdown-overlay` crate + minimal changes to existing crates

## Problem

Sparkdown embeds semantic annotations inline in markdown (e.g., `# Title {.schema:Event startDate=2026-03-20}`, `:entity[Name]{type=schema:Person}`). As semantics grow richer, the markdown becomes cluttered and hard to read or edit as plain text.

## Solution: Layered Architecture

Inspired by Entity Framework's conceptual/storage/mapping layers. Three layers separate concerns:

1. **Markdown Layer** — the structural AST (headings, paragraphs, lists). Produced by `sparkdown-core` from a clean `.md` file.
2. **Semantic Layer** — an RDF-rooted graph of entities, relationships, and rhetorical annotations. Stored in a `.sparkdown-sem` sidecar file.
3. **Mapping Layer** — a bidirectional index connecting markdown AST nodes to semantic graph nodes. Ephemeral, rebuilt on load.

```
+--------------+          +--------------+
|  Markdown    |<-------->|   Mapping    |
|  AST         |  spans   |   Index      |
| (from .md)   |          | (ephemeral)  |
+--------------+          +------+-------+
                                 |
                          +------v-------+
                          |  Semantic    |
                          |  Graph       |
                          | (RDF/OWL)    |
                          | (.sparkdown-sem)
                          +--------------+
```

### Authoring Model

- A **human author** writes clean markdown. No semantic syntax required.
- An **AI agent observer** reads the markdown and writes the semantic overlay.
- The human can **review and adjust** the AI's annotations via an overlay mode in an editor.
- The overlay can be **toggled on/off** for reading, without affecting the markdown source.

### Use Cases

- **Editor experience:** IDE shows/hides semantic annotations as a visual overlay on clean markdown.
- **Inference engine ingestion:** The semantic graph exports to standard RDF for external knowledge graph tools.
- **Enhanced UI visualization:** Semantic info rendered on a separate plane or overlay in rich editors.

---

## Layer 1: Markdown AST (Existing)

The `sparkdown-core` parsing pipeline is unchanged:

```
Source Text → Frontmatter → Preprocess → pulldown-cmark → Postprocess → SemanticNode tree
```

The existing `annotations` field on `SemanticNode` becomes optional/deprecated in the overlay workflow. Inline directive syntax (`:entity[...]{...}`) is still parsed for backward compatibility, but the canonical home for semantics is the sidecar overlay.

No breaking changes to `sparkdown-core`.

---

## Layer 2: Semantic Graph

An in-memory RDF graph where nodes are typed entities, edges are RDF properties, and every entity carries a positional anchor into the markdown source.

### Core Types

```rust
struct SemanticGraph {
    source_hash: [u8; 32],       // SHA-256 of .md file when last synced
    prefixes: PrefixMap,
    entities: Vec<SemanticEntity>,
    triples: Vec<Triple>,
}

struct SemanticEntity {
    id: BlankNodeId,             // _:e1, _:e2, ...
    anchor: Anchor,
    types: Vec<Iri>,             // rdf:type values
    status: AnchorStatus,
}

struct Anchor {
    span: Range<usize>,          // byte range in markdown source
    snippet: String,             // first ~40 chars for diagnostics
}

enum AnchorStatus {
    Synced,     // anchor verified against current source
    Stale,      // source changed under this anchor, needs AI review
    Detached,   // anchored text was deleted entirely
}

struct Triple {
    subject: BlankNodeId,
    predicate: Iri,
    object: TripleObject,        // BlankNodeId or literal value
}

enum TripleObject {
    Entity(BlankNodeId),
    Literal { value: String, datatype: Option<Iri> },
}
```

### Annotation Scope

The semantic graph captures:

- **Entities** — typed using OWL/RDF classes from registered ontologies (schema:Person, schema:Event, etc.)
- **Properties** — key-value metadata on entities (schema:name, schema:startDate, etc.)
- **Relationships** — triples connecting entities (schema:performerIn, schema:location, etc.)
- **Rhetorical structure** — using a sparkdown-specific vocabulary (`sd:`) to annotate document structure

### Sparkdown Vocabulary (`sd:`)

A small ontology for structural/rhetorical annotation:

| Type/Property | Purpose |
|--------------|---------|
| `sd:Section` | A structural section of the document |
| `sd:Paragraph` | A paragraph-level annotation target |
| `sd:role` | Property linking structure to rhetorical function |
| `sd:Review` | Rhetorical role: review/opinion |
| `sd:Abstract` | Rhetorical role: summary/abstract |
| `sd:Argument` | Rhetorical role: argumentative content |
| `sd:Summary` | Rhetorical role: summarization |
| `sd:Comparison` | Rhetorical role: comparative analysis |
| `sd:Example` | Rhetorical role: illustrative example |

---

## Layer 3: Mapping Index

A bidirectional index connecting markdown AST node spans to semantic entity IDs:

```rust
struct MappingIndex {
    md_to_sem: BTreeMap<Range<usize>, Vec<BlankNodeId>>,
    sem_to_md: HashMap<BlankNodeId, Range<usize>>,
}
```

The mapping index is **never persisted**. It is rebuilt whenever both the markdown AST and semantic graph are loaded. It answers:

- "What semantics exist for this paragraph?" (editor overlay query)
- "Which markdown text does this entity refer to?" (rendering query)

`BTreeMap` is used for the markdown-to-semantic direction so that range lookups are efficient (find all entities overlapping a given span).

---

## Sidecar Format: `.sparkdown-sem`

The file lives alongside the markdown: `article.md` -> `article.md.sparkdown-sem`

### Format

Turtle-inspired syntax extended with span anchors. Turtle was chosen because:

- Most human-readable RDF serialization
- Git diffs are meaningful (one triple per line)
- Familiar to the ontology community
- Only one extension needed: the `[start..end]` anchor syntax

### Example

```turtle
@source-hash "sha256:a1b2c3d4e5f6..." .
@prefix schema: <http://schema.org/> .
@prefix sd: <sparkdown:vocab/> .
@prefix dc: <http://purl.org/dc/elements/1.1/> .

# Document-level
_:doc [0..] a schema:Event ;
    schema:name "RustConf 2026" ;
    schema:startDate "2026-09-10" ;
    schema:endDate "2026-09-12" .

# Entities
_:e1 [142..158] a schema:Person ;
    schema:name "Niko Matsakis" ;
    sd:snippet "Niko Matsakis" .

_:e2 [210..218] a schema:Place ;
    schema:name "Portland" ;
    sd:snippet "Portland" .

# Relationships
_:e1 schema:performerIn _:doc .
_:doc schema:location _:e2 .

# Rhetorical structure
_:s1 [1200..1450] a sd:Section ;
    sd:role sd:Review ;
    sd:snippet "An excellent conference..." .
```

### Format Rules

- `@source-hash` — SHA-256 of the `.md` file at last sync. Detects drift.
- `[start..end]` — byte span anchor after the blank node ID. `[0..]` means whole document. Only on entity declarations, not relationship triples.
- `sd:snippet` — short content fingerprint (~40 chars). For diagnostics and readability only, never used for anchoring logic.
- Blank node IDs (`_:e1`) are stable within the file. The AI agent assigns them and they persist across edits.
- Standard Turtle rules: `;` continues subject, `.` ends statement, `#` for comments.

### What This Is NOT

- Not valid Turtle (the `[start..end]` syntax is a sparkdown extension). A preprocessor can strip anchors to produce valid Turtle for standard RDF tools.
- Not intended for direct human authoring. The AI writes it; the human adjusts via overlay mode.

---

## Sync & Staleness Mechanism

When the human edits the markdown, the semantic overlay must adapt.

### Edit Cycle

```
1. Load article.md            -> Markdown AST
2. Load article.md.sparkdown-sem -> Semantic Graph
3. Compare source hash         -> Match? Synced. Mismatch? Run sync.
4. Human edits article.md     -> New source, new hash
5. Run sync                   -> Adjust anchors, flag stale entities
6. AI reviews stale entities  -> Updates/removes/adds annotations
7. Save .sparkdown-sem        -> New source hash, updated anchors
```

### Sync Algorithm

**Input:** old source (from git or cache), new source, semantic graph.
**Output:** updated graph with adjusted anchors and status flags.

**Step 1 — Diff the markdown sources.** Produce edit operations:

```rust
enum EditOp {
    Insert { at: usize, len: usize },
    Delete { at: usize, len: usize },
    Replace { at: usize, old_len: usize, new_len: usize },
}
```

Uses byte-level diffing (e.g., `similar` crate).

**Step 2 — Adjust all anchors.** Walk edit ops and shift every anchor span:

- Anchors **before** the edit: unchanged
- Anchors **after** the edit: shift by delta (`new_len - old_len`)
- Anchors **overlapping** the edit: mark `Stale`
- Anchors **fully inside** a deletion: mark `Detached`

**Step 3 — Verify snippets.** For each `Synced` anchor, check that `sd:snippet` still matches text at the adjusted span. Mismatch downgrades to `Stale`. Catches silent rewording without length change.

**Step 4 — Update source hash and save.**

### Stale/Detached Entity Resolution

The sync engine is deterministic and fast (pure offset math). It flags but never deletes. The AI agent handles resolution:

- **Stale:** Re-read anchored text, decide if annotation is still correct, update or remove.
- **Detached:** Decide if entity should be re-anchored elsewhere, removed, or kept as unanchored fact.

### Old Source Recovery

To compute diffs, the old source is needed:

- **Primary:** `git show HEAD:<file>` for the last-committed version.
- **Fallback:** If git unavailable, mark all anchors `Stale` (safe but requires full AI re-review).

The `@source-hash` in the sidecar detects when a diff is needed.

---

## Crate Architecture

### New: `sparkdown-overlay`

```
crates/sparkdown-overlay/
├── src/
│   ├── lib.rs
│   ├── graph.rs       # SemanticGraph, SemanticEntity, Triple, TripleObject
│   ├── anchor.rs      # Anchor, AnchorStatus, span arithmetic
│   ├── sync.rs        # Diff engine, anchor adjustment, snippet verification
│   ├── mapping.rs     # MappingIndex (bidirectional, ephemeral)
│   ├── sidecar.rs     # .sparkdown-sem parser and serializer
│   └── vocab.rs       # sd: vocabulary constants
├── Cargo.toml
```

**Dependencies:** `sparkdown-core` (for AST types, PrefixMap), `sparkdown-ontology` (for validation), `similar` (diffing), `oxrdf` (IRI handling).

### Changes to Existing Crates

**sparkdown-core** — No breaking changes. `annotations` on `SemanticNode` stays for legacy mode.

**sparkdown-ontology** — Small addition: the `sd:` vocabulary as a built-in ontology provider alongside schema.org, Dublin Core, and FOAF.

**sparkdown-render** — Alternate entry point: render from `SemanticGraph` directly (for RDF export). HTML+RDFa renderer can merge markdown AST + overlay when both are present.

**sparkdown-cli** — New subcommands:

| Command | Purpose |
|---------|---------|
| `sparkdown overlay init` | Create empty `.sparkdown-sem` for a `.md` file |
| `sparkdown overlay sync` | Run sync engine after markdown edits |
| `sparkdown overlay status` | Show stale/detached entities |
| `sparkdown overlay export` | Strip anchors, produce valid Turtle |
| `sparkdown overlay merge` | Combined view (markdown + inline annotations) for debugging |
| `sparkdown overlay import` | Extract inline annotations from legacy `.md` into sidecar |

---

## Migration Path

Three operating modes:

1. **Legacy mode** — inline annotations in markdown, no sidecar. Works exactly like today.
2. **Overlay mode** — clean markdown + `.sparkdown-sem` sidecar. The primary new workflow.
3. **Hybrid mode** — inline annotations parsed and merged into semantic graph on load.

Migration via `sparkdown overlay import`:

```
sparkdown overlay import article.md
# -> reads inline annotations from article.md
# -> writes article.md.sparkdown-sem
# -> outputs cleaned article.md (no inline annotations)
```

---

## Testing Strategy

### Unit Tests (per module in `sparkdown-overlay`)

| Module | Coverage |
|--------|----------|
| `sidecar.rs` | Round-trip parse/serialize, invalid syntax errors |
| `anchor.rs` | Overlap detection, span arithmetic edge cases, empty spans, `[0..]` |
| `sync.rs` | Insert before/after/inside anchor, delete spanning multiple anchors, replace changing length, status transitions, snippet verification |
| `mapping.rs` | Build from graph + AST, query both directions, overlapping spans |
| `graph.rs` | Triple construction, prefix resolution, ontology validation |
| `vocab.rs` | `sd:` constants resolve to correct IRIs |

### Integration Tests

- **Full round-trip:** markdown + sidecar -> load -> build mapping -> edit markdown -> sync -> verify anchors -> save -> reload -> verify consistency
- **Migration:** legacy document with inline annotations -> `import` -> verify sidecar, verify cleaned markdown, verify identical rendering output
- **Export:** load sidecar -> strip anchors -> verify output is valid Turtle parseable by standard RDF tools

### Test Fixtures

```
tests/fixtures/
├── basic.md                    # existing
├── event.md                    # existing
├── event.md.sparkdown-sem      # hand-written sidecar for event.md
├── edits/
│   ├── insert_paragraph.md     # event.md with paragraph added
│   ├── delete_section.md       # event.md with review section removed
│   ├── reword_heading.md       # event.md with heading text changed
│   └── expected/               # expected .sparkdown-sem after sync
└── legacy/
    └── inline_annotated.md     # document with inline directives for migration test
```

---

## Error Handling

Two categories:

### Parse Errors (hard failures)

Malformed sidecar syntax, invalid blank node IDs, unresolved prefixes. Return `Result<SemanticGraph, OverlayError>` with line/column information.

```rust
enum OverlayError {
    Parse { line: usize, col: usize, message: String },
    UnresolvedPrefix(String),
    InvalidAnchor { entity: String, reason: String },
    Io(std::io::Error),
}
```

### Sync Errors (recoverable, lenient)

- Source hash mismatch without git: warn, mark all anchors `Stale`
- Anchor beyond end of file: mark `Detached`
- Snippet mismatch: downgrade to `Stale`

The sync engine always produces a result. It never fails the operation.

**Principle: parsing is strict, syncing is lenient.** A corrupt sidecar is an error. A stale overlay is a normal state the AI agent resolves.
