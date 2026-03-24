Committed and pushed. Here's a summary of the design:

---

## Sparkdown Studio — The Design

**The core idea:** Writing markdown *is* building a knowledge graph. The sidecar architecture means your text stays clean while a living semantic layer grows alongside it.

### Writing Mode: "The Quiet Companion"
- **Semantic gutter** — tiny colored dots in the left margin signal entity density per line, like a heat signature for meaning
- **Entity shimmer** — subtle underlines fade in only when your cursor is nearby, fade out when you move away
- **Whisper cards** — hover 300ms on an entity and a minimal tooltip shows its type and top relationships. Start typing and it vanishes
- **Suggestion tray** — a single status line at the bottom: `✦ 3 entities · 1 suggestion · sidecar: synced · 12 triples`. One-click accept/dismiss for AI-suggested entities
- **Stale anchor nudges** — spell-check-style hints when you've reworded text under a semantic anchor
- **Quick create** — `Cmd+E` on selected text opens a type picker; entity goes straight to sidecar, text untouched

### Reading Mode: "The Knowledge Constellation"
- **Three-panel layout**: rendered document (with illuminated entities) + contextual knowledge panel + constellation bar
- **Knowledge panel** shows entity properties, cross-document mentions ("7 mentions in 4 docs"), relationships, and external data (Wikidata)
- **Constellation bar** switches between: force-directed knowledge graph, timeline (for dated entities), and a connections web showing how *documents* link through shared entities
- **Semantic search**: `type:Person`, `related:RustConf`, `property:startDate>2026-06-01`
- **Discovery feed**: proactive suggestions — "Portland appears in 4 docs but has no properties. Add schema:geo?"
- **Cross-document entity unification**: write "Niko" and it auto-suggests linking to `Niko Matsakis (4 docs, 12 triples)` — one keystroke

### The Graceful Shift
Mode isn't a toggle — it's a **gradient**. Pause typing → underlines fade in. Stop scrolling → knowledge panel slides in. Start typing → everything recedes. The UI breathes with your workflow.

The full spec is at `docs/superpowers/specs/2026-03-21-sparkdown-studio-ui-design.md`.