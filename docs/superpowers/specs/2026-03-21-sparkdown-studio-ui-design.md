# Sparkdown Studio: Knowledge UI Design Specification

## Context

Sparkdown is a semantic markdown processor whose killer feature is the **sidecar architecture**: your markdown stays clean while a `.sparkdown-sem` file holds a full RDF knowledge graph anchored to exact byte-spans in your text. The mapping index answers "what entities exist at this cursor position?" in O(log n). The sync engine tracks when edits make anchors stale. Four ontologies (schema.org, Dublin Core, FOAF, Sparkdown vocab) provide a rich type system out of the box.

This design turns that engine into a **personal and organizational knowledge application** — codenamed **Sparkdown Studio** — where writing markdown *is* building a knowledge graph, and reading your notes reveals the hidden structure connecting everything you know.

---

## Design Philosophy

**The Three Laws of Sparkdown Studio:**

1. **The editor is sacred.** Writing flow is never interrupted. Semantic awareness is ambient — like peripheral vision.
2. **Knowledge reveals itself.** In reading mode, the graph isn't something you query — it's something you *see*. Connections surface naturally.
3. **Enrichment is a gift, not a chore.** The system suggests; you accept with a keystroke or ignore with no penalty.

---

## Part 1: Writing Mode — "The Quiet Companion"

### 1.1 The Editor Surface

A clean, distraction-free markdown editor. No toolbars. No clutter. Just your text, a gentle monospace font, and a thin **semantic gutter** on the left edge.

**The Semantic Gutter** (4px wide, left margin):
- Tiny colored dots appear next to lines containing recognized entities
- Color encodes entity type: `schema:Person` = warm amber, `schema:Place` = teal, `schema:Event` = violet, `schema:Organization` = slate blue
- Dot density tells you at a glance how semantically rich a paragraph is
- A line with no dots is just prose. A line with 3 dots is a knowledge-dense sentence
- Think of it like a heat signature for meaning

### 1.2 Inline Entity Shimmer

When the Sparkdown engine recognizes an entity in your text (either from the existing sidecar or from AI suggestion), the entity text receives a **subtle underline** — not a harsh highlight, but a gentle colored underline that matches the gutter dot color.

- Existing/confirmed entities: solid subtle underline (1px, 20% opacity of type color)
- AI-suggested entities (not yet in sidecar): dotted underline (same style, even lighter)
- Stale entities (sync engine flagged): the underline gains a faint amber pulse, like a heartbeat — "I noticed something changed here"

The underlines are **only visible on hover proximity** — when your cursor is within ~3 lines of an entity, the underlines in that zone gently fade in. Move away, they fade out. You're never staring at a christmas tree of highlights.

### 1.3 Cursor Hover: The Entity Whisper

When your cursor rests on an entity for 300ms, a **whisper card** appears — a minimal floating tooltip, not a modal:

```
┌─────────────────────────────┐
│ ● Niko Matsakis             │
│   schema:Person             │
│   performerIn → RustConf    │
│                    ↗ Open   │
└─────────────────────────────┘
```

- Shows entity name, type, and top 1-2 relationships
- "Open" arrow navigates to the entity in the Knowledge Panel (reading mode)
- Appears below/beside cursor, never overlapping your current line
- Disappears when you start typing (zero friction)
- Keyboard shortcut `Cmd+K` pins the whisper card and expands to a mini-editor where you can add/edit properties inline

### 1.4 The Suggestion Tray

At the bottom of the editor, a **single-line tray** (think VS Code status bar, but smarter):

```
  ✦ 3 entities detected  ·  1 suggestion  ·  sidecar: synced  ·  12 triples
```

- **Entity count**: How many anchored entities exist in the current document
- **Suggestions**: AI-detected entities not yet confirmed. Click to review.
- **Sidecar status**: `synced` / `2 stale` / `1 detached` — at-a-glance health
- **Triple count**: How large your document's knowledge graph is

Clicking "1 suggestion" opens a **suggestion ribbon** that slides up from the tray:

```
┌──────────────────────────────────────────────────────────────┐
│  Suggested: "Portland" → schema:Place                    ✓ ✕ │
│  Suggested: "keynote" → schema:Event (rhetorical: Review) ✓ ✕ │
└──────────────────────────────────────────────────────────────┘
```

Accept (✓) writes to the sidecar. Dismiss (✕) trains future suggestions. One keystroke each. No forms, no dialogs.

### 1.5 The Stale Anchor Nudge

When the sync engine detects stale anchors after an edit, a gentle inline annotation appears:

```markdown
Niko Matsakis will deliver the opening talk.
                              ~~~~~~~~~~~
                              ↑ "keynote" → "opening talk" — update entity? [y/n]
```

This appears as a subtle inline decoration (like a spell-check suggestion). Press `y` to update the snippet in the sidecar. Press `n` or just keep typing to dismiss. The sync engine handles the byte-span adjustment automatically.

### 1.6 Quick Entity Creation

Select any text + press `Cmd+E`:

```
┌─────────────────────────────┐
│ "Dr. Sarah Chen"            │
│                             │
│  ● Person      ○ Org       │
│  ○ Place       ○ Event     │
│  ○ Article     ○ Custom... │
│                             │
│  ↳ Wikidata: Q______       │
│              [Tab to skip]  │
└─────────────────────────────┘
```

- Type is auto-suggested based on context (NLP from surrounding text)
- One click/keystroke confirms
- Optional Wikidata/DOI/ORCID linking right there
- Entity is written to sidecar immediately — the text itself is untouched

---

## Part 2: Reading Mode — "The Knowledge Constellation"

Triggered by: `Cmd+Shift+R`, clicking a document in the navigator when not in edit mode, or opening a shared/published view.

### 2.1 The Split View

The screen splits into three zones:

```
┌─────────────────────────┬──────────────────────┐
│                         │                      │
│     Document View       │   Knowledge Panel    │
│   (rendered markdown    │   (contextual,       │
│    with entity          │    changes based on   │
│    highlights)          │    what you're        │
│                         │    looking at)        │
│                         │                      │
├─────────────────────────┴──────────────────────┤
│              Constellation Bar                  │
│     (graph minimap / timeline / connections)    │
└─────────────────────────────────────────────────┘
```

### 2.2 Document View (Left Panel)

The rendered markdown, but now entities are **fully illuminated**:

- Entity text is highlighted with type-colored backgrounds (soft pastels, not harsh)
- Hovering an entity **pulses its connections** — all related entities in the document glow
- Clicking an entity selects it and populates the Knowledge Panel
- Rhetorical sections (Review, Abstract, Argument) get a colored left-border and a small label badge
- Stale entities show an amber left-border: "This section has changed since semantic review"

**Entity density heatmap**: A subtle gradient overlay on the right margin shows which paragraphs are most semantically rich — darker = more entities/triples. Helps you spot under-annotated sections.

### 2.3 Knowledge Panel (Right Panel)

Context-sensitive. Changes based on what's selected.

**When nothing is selected — Document Overview:**

```
┌──────────────────────────┐
│ RustConf 2026            │
│    schema:Event           │
│                          │
│ ── Properties ────────── │
│ startDate   2026-09-10   │
│ endDate     2026-09-12   │
│ location    Portland ↗   │
│                          │
│ ── Entities (4) ──────── │
│ ● Niko Matsakis  Person  │
│ ● Portland       Place   │
│ ● _:s1          Review   │
│                          │
│ ── Connections ────────── │
│ performerIn: Niko → this │
│ location: this → Portland│
│                          │
│ ── Appears in ────────── │
│ meeting-notes.md (3x)    │
│ rust-ecosystem.md (1x)   │
│ 2026-travel-plans.md     │
└──────────────────────────┘
```

**When an entity is selected — Entity Deep Dive:**

```
┌──────────────────────────┐
│ ● Niko Matsakis          │
│   schema:Person           │
│   wikidata:Q28553578 ↗   │
│                          │
│ ── In this document ──── │
│ performerIn → RustConf   │
│ Mentioned at: line 5     │
│                          │
│ ── Across all docs ───── │
│ 7 mentions in 4 docs     │
│ ├ meeting-notes.md (3)   │
│ ├ rust-ecosystem.md (2)  │
│ ├ conference-plan.md (1) │
│ └ this document (1)      │
│                          │
│ ── Knowledge Graph ───── │
│ knows → Josh Triplett    │
│ memberOf → Rust Project  │
│ author → "MIR RFC"       │
│                          │
│ ── External ──────────── │
│ Wikidata: Software dev.. │
│ [Fetch latest from WD]   │
└──────────────────────────┘
```

The **"Across all docs"** section is the magic — it shows everywhere this entity appears in your entire knowledge base. Click any document to navigate there, with the entity pre-highlighted.

### 2.4 The Constellation Bar (Bottom Panel)

A horizontal strip that provides three switchable views:

**View 1: Knowledge Graph (default)**

A force-directed mini-graph showing entities in the current document as nodes and triples as edges. Nodes are colored by type. Clicking a node selects it in both the Document View and Knowledge Panel.

- Current document entities are bright; entities from other documents that connect to these are shown as dimmer "ghost nodes" at the periphery
- Dragging a ghost node "into" the graph opens that document
- The graph breathes — nodes gently float, edges have subtle animation. It feels alive.

**View 2: Timeline**

For documents with temporal entities (Events with dates, Dublin Core dates), a horizontal timeline:

```
2026-09-10        2026-09-11        2026-09-12
    ├─── RustConf 2026 ───────────────┤
    │                                  │
  Keynote                          Closing
  (Niko M.)                        Ceremony
```

Entities are placed on the timeline by their date properties. Click to navigate.

**View 3: Connections Web**

Shows how the current document connects to OTHER documents through shared entities:

```
                    ┌─ rust-ecosystem.md (via: Niko, Rust)
  THIS DOCUMENT ────┤
                    ├─ 2026-travel-plans.md (via: Portland)
                    └─ meeting-notes.md (via: Niko, RustConf)
```

Each connection line is labeled with the shared entities. Thicker lines = more shared entities. This is how you discover unexpected connections between your notes.

---

## Part 3: The "Wow" Moments

### 3.1 Semantic Search

The search bar (`Cmd+P`) understands semantics:

- `type:Person` — find all people across all documents
- `related:RustConf` — find everything connected to RustConf via any triple
- `stale:true` — find documents with stale anchors needing review
- `role:Review` — find all review sections across your knowledge base
- `property:startDate>2026-06-01` — find events after June 2026
- Plain text search still works, but results are **ranked by semantic relevance** — a document mentioning "Niko" that has a `schema:Person` entity for him ranks higher than one with just the text

### 3.2 The Discovery Feed

On the home screen (no document open), a feed of insights:

```
┌────────────────────────────────────────────┐
│  ✦ Discovery Feed                          │
│                                            │
│  "Portland" appears in 4 docs but has no   │
│  properties yet. Add schema:geo?       [+] │
│                                            │
│  "Niko Matsakis" and "Josh Triplett" both   │
│  appear in 3 docs. Are they connected? [+] │
│                                            │
│  Your notes from last week mention 2        │
│  events with no dates. Add dates?      [+] │
│                                            │
│  3 entities became stale after your edits   │
│  to meeting-notes.md. Review?          [→] │
│                                            │
└────────────────────────────────────────────┘
```

The system proactively suggests enrichments, connections, and maintenance tasks. Each is one click to act on.

### 3.3 Cross-Document Entity Unification

When you write "Niko" in a new document and the system recognizes it matches `_:e1 schema:Person "Niko Matsakis"` from another document, it suggests linking:

```
  "Niko" — link to Niko Matsakis (4 docs, 12 triples)? [Tab to accept]
```

One keystroke and your new document is connected to the entire knowledge web around that person. No manual linking, no wiki-style `[[brackets]]` required.

### 3.4 Export and Publish

From any document or collection:
- **Publish as HTML+RDFa** — semantic web page with embedded structured data
- **Export JSON-LD** — feed into any knowledge graph system
- **Export Turtle** — standard RDF for academic/semantic web tools
- **Export collection as knowledge graph** — merge multiple documents' sidecar files into a single unified graph

### 3.5 Team Knowledge Mode

For organizational use:
- Shared entity registry — when anyone on the team creates `schema:Person "Sarah Chen"`, it's available to everyone
- Entity merging — two people annotated the same entity differently? Merge with a click
- Knowledge graph dashboard — see the team's collective knowledge graph, find gaps, spot clusters

---

## Part 4: Mode Transition — The Graceful Shift

The transition between writing and reading mode is not a hard switch. It's a **gradient**:

1. **Deep Writing** — minimal UI, only gutter dots visible, whisper cards on hover
2. **Light Writing** — you pause typing for 2 seconds, entity underlines gently fade in within your viewport
3. **Review** — you scroll without typing for 5 seconds, the Knowledge Panel slides in from the right (25% width), entity highlights strengthen
4. **Full Reading** — `Cmd+Shift+R` or click the mode toggle, full three-panel layout with Constellation Bar

At any point, starting to type collapses back toward Deep Writing mode. The UI breathes with your workflow.

---

## Part 5: Visual Language

### Color System (Entity Types)

| Type | Color | Hex |
|------|-------|-----|
| Person | Warm Amber | `#F59E0B` |
| Place | Teal | `#14B8A6` |
| Event | Violet | `#8B5CF6` |
| Organization | Slate Blue | `#6366F1` |
| Article/Document | Sage Green | `#22C55E` |
| Review/Rhetorical | Rose | `#F43F5E` |
| Custom | Gray | `#6B7280` |

### Typography

- Editor: `JetBrains Mono` or `Berkeley Mono` at 14px
- UI chrome: `Inter` at 13px
- Entity labels in graph: `Inter` at 11px, medium weight

### Dark Mode (Default)

- Background: `#0F0F0F`
- Editor surface: `#1A1A1A`
- Entity highlights use the colors above at 15% opacity for backgrounds, full saturation for dots/underlines
- The knowledge graph uses a dark canvas with glowing nodes

---

## Part 6: Sparkdown API Surface for the UI

The UI is built directly on Sparkdown's existing Rust API:

| UI Feature | Sparkdown API | Crate |
|------------|---------------|-------|
| Cursor entity lookup | `MappingIndex::entities_at(range)` | sparkdown-overlay |
| Entity details | `SemanticGraph.entities` + `SemanticGraph.triples` | sparkdown-overlay |
| Staleness indicators | `Anchor::verify_snippet()` / `AnchorStatus` | sparkdown-overlay |
| Post-edit sync | `SyncEngine::sync(old, new, graph)` | sparkdown-overlay |
| Sidecar read/write | `sidecar::parse()` / `sidecar::serialize()` | sparkdown-overlay |
| Type validation | `ThemeRegistry::lookup_type()` | sparkdown-ontology |
| HTML+RDFa export | `HtmlRdfaRenderer::render()` | sparkdown-render |
| JSON-LD export | `JsonLdRenderer::render()` | sparkdown-render |
| Turtle export | `TurtleRenderer::render()` | sparkdown-render |
| Prefix resolution | `PrefixMap::resolve()` | sparkdown-core |
| Document parsing | `parse_document()` | sparkdown-core |
