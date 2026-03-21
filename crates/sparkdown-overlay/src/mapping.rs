//! Bidirectional mapping index connecting markdown spans to semantic entities.

use rust_lapper::{Interval, Lapper};
use oxrdf::BlankNode;
use std::collections::HashMap;
use std::ops::Range;

use crate::anchor::AnchorStatus;
use crate::graph::SemanticGraph;

/// A bidirectional index connecting markdown AST node spans to semantic entity IDs.
///
/// This index is **ephemeral** — never persisted, rebuilt on load.
pub struct MappingIndex {
    /// Interval tree for overlap queries: "find all entities whose span overlaps range X..Y."
    md_to_sem: Lapper<usize, Vec<BlankNode>>,
    /// Reverse lookup: entity ID -> markdown span.
    sem_to_md: HashMap<BlankNode, Range<usize>>,
}

impl MappingIndex {
    /// Build a mapping index from a semantic graph.
    ///
    /// Only includes entities with `Synced` or `Stale` status (not `Detached`).
    pub fn build(graph: &SemanticGraph) -> Self {
        let mut intervals = Vec::new();
        let mut sem_to_md = HashMap::new();

        for entity in &graph.entities {
            if entity.status == AnchorStatus::Detached {
                continue;
            }

            let start = entity.anchor.span.start;
            // For the interval tree, clamp open-ended anchors to a large value
            let stop = if entity.anchor.is_open_ended() {
                usize::MAX - 1 // Lapper needs stop > start
            } else {
                entity.anchor.span.end
            };

            if stop <= start {
                continue; // Skip empty spans
            }

            intervals.push(Interval {
                start,
                stop,
                val: vec![entity.id.clone()],
            });
            sem_to_md.insert(entity.id.clone(), entity.anchor.span.clone());
        }

        // Lapper requires sorted intervals
        intervals.sort_by_key(|iv| iv.start);
        let md_to_sem = Lapper::new(intervals);

        Self {
            md_to_sem,
            sem_to_md,
        }
    }

    /// Find all entity IDs whose anchor span overlaps the given range.
    pub fn entities_at(&self, range: Range<usize>) -> Vec<BlankNode> {
        let mut result = Vec::new();
        for interval in self.md_to_sem.find(range.start, range.end) {
            result.extend(interval.val.iter().cloned());
        }
        result
    }

    /// Look up the markdown span for a given entity ID.
    pub fn span_for(&self, id: &BlankNode) -> Option<Range<usize>> {
        self.sem_to_md.get(id).cloned()
    }

    /// Returns the number of indexed entities.
    pub fn len(&self) -> usize {
        self.sem_to_md.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.sem_to_md.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anchor::Anchor;
    use crate::graph::{SemanticEntity, SemanticGraph};
    use oxrdf::NamedNode;

    fn make_graph() -> SemanticGraph {
        let mut g = SemanticGraph::new([0u8; 32]);
        g.entities.push(SemanticEntity {
            id: BlankNode::new("doc").unwrap(),
            anchor: Anchor::new(0..usize::MAX, ""),
            types: vec![NamedNode::new("http://schema.org/Event").unwrap()],
            status: AnchorStatus::Synced,
        });
        g.entities.push(SemanticEntity {
            id: BlankNode::new("e1").unwrap(),
            anchor: Anchor::new(10..50, "hello"),
            types: vec![NamedNode::new("http://schema.org/Person").unwrap()],
            status: AnchorStatus::Synced,
        });
        g.entities.push(SemanticEntity {
            id: BlankNode::new("e2").unwrap(),
            anchor: Anchor::new(100..150, "world"),
            types: vec![NamedNode::new("http://schema.org/Place").unwrap()],
            status: AnchorStatus::Synced,
        });
        g.entities.push(SemanticEntity {
            id: BlankNode::new("e3").unwrap(),
            anchor: Anchor::new(200..250, "detached"),
            types: vec![],
            status: AnchorStatus::Detached,
        });
        g
    }

    #[test]
    fn build_excludes_detached() {
        let g = make_graph();
        let idx = MappingIndex::build(&g);
        assert_eq!(idx.len(), 3); // doc, e1, e2 — not e3
    }

    #[test]
    fn entities_at_overlap_query() {
        let g = make_graph();
        let idx = MappingIndex::build(&g);

        // Query that overlaps e1 (10..50)
        let entities = idx.entities_at(20..30);
        let ids: Vec<_> = entities.iter().map(|bn| bn.as_str().to_string()).collect();
        assert!(ids.contains(&"e1".to_string()));
        assert!(ids.contains(&"doc".to_string())); // doc is 0..MAX

        // Query that overlaps only e2
        let entities = idx.entities_at(110..120);
        let ids: Vec<_> = entities.iter().map(|bn| bn.as_str().to_string()).collect();
        assert!(ids.contains(&"e2".to_string()));
        assert!(ids.contains(&"doc".to_string()));
        assert!(!ids.contains(&"e1".to_string()));
    }

    #[test]
    fn span_for_reverse_lookup() {
        let g = make_graph();
        let idx = MappingIndex::build(&g);

        let e1_id = BlankNode::new("e1").unwrap();
        let span = idx.span_for(&e1_id).unwrap();
        assert_eq!(span, 10..50);

        let e3_id = BlankNode::new("e3").unwrap();
        assert!(idx.span_for(&e3_id).is_none()); // detached, not indexed
    }

    #[test]
    fn no_results_for_gap() {
        let g = make_graph();
        let idx = MappingIndex::build(&g);

        // Between e1 (10..50) and e2 (100..150) — only doc should match
        let entities = idx.entities_at(60..90);
        let ids: Vec<_> = entities.iter().map(|bn| bn.as_str().to_string()).collect();
        assert_eq!(ids.len(), 1); // only doc
        assert!(ids.contains(&"doc".to_string()));
    }
}
