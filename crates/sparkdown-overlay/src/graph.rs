//! Core semantic graph types: SemanticGraph, SemanticEntity, Triple, and errors.

use oxrdf::{BlankNode, NamedNode};
use sparkdown_core::prefix::PrefixMap;
use thiserror::Error;

use crate::anchor::{Anchor, AnchorStatus};

/// Convenience constructor for `BlankNode` that panics on invalid input.
pub fn blank_node(id: &str) -> BlankNode {
    BlankNode::new(id).expect("valid blank node ID")
}

/// Error type for overlay operations.
#[derive(Debug, Error)]
pub enum OverlayError {
    #[error("parse error at line {line}, col {col}: {message}")]
    Parse {
        line: usize,
        col: usize,
        message: String,
    },

    #[error("unresolved prefix: {0}")]
    UnresolvedPrefix(String),

    #[error("invalid anchor for entity {entity}: {reason}")]
    InvalidAnchor { entity: String, reason: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// The semantic graph for a single document's overlay.
#[derive(Debug, Clone)]
pub struct SemanticGraph {
    /// SHA-256 of the `.md` file when last synced.
    pub source_hash: [u8; 32],
    /// Prefix mappings for CURIE resolution.
    pub prefixes: PrefixMap,
    /// All entities with their anchors and types.
    pub entities: Vec<SemanticEntity>,
    /// All relationship/property triples.
    pub triples: Vec<Triple>,
}

/// A semantic entity anchored to a span in the markdown source.
#[derive(Debug, Clone)]
pub struct SemanticEntity {
    /// Blank node identifier (e.g., `_:e1`, `_:doc`).
    pub id: BlankNode,
    /// Positional anchor into the markdown source.
    pub anchor: Anchor,
    /// `rdf:type` values for this entity.
    pub types: Vec<NamedNode>,
    /// Current sync status of this entity's anchor.
    pub status: AnchorStatus,
}

/// An RDF triple in the semantic graph.
#[derive(Debug, Clone)]
pub struct Triple {
    pub subject: BlankNode,
    pub predicate: NamedNode,
    pub object: TripleObject,
}

/// The object of a triple — either another entity or a literal value.
#[derive(Debug, Clone)]
pub enum TripleObject {
    /// Reference to another entity by blank node ID.
    Entity(BlankNode),
    /// A literal value with optional datatype.
    Literal {
        value: String,
        datatype: Option<NamedNode>,
    },
}

impl SemanticGraph {
    /// Create an empty graph with the given source hash.
    pub fn new(source_hash: [u8; 32]) -> Self {
        let mut prefixes = PrefixMap::new();
        prefixes.seed_builtins();
        Self {
            source_hash,
            prefixes,
            entities: Vec::new(),
            triples: Vec::new(),
        }
    }

    /// Find an entity by its blank node ID.
    pub fn entity_by_id(&self, id: &BlankNode) -> Option<&SemanticEntity> {
        self.entities.iter().find(|e| &e.id == id)
    }

    /// Find an entity mutably by its blank node ID.
    pub fn entity_by_id_mut(&mut self, id: &BlankNode) -> Option<&mut SemanticEntity> {
        self.entities.iter_mut().find(|e| &e.id == id)
    }

    /// Get all triples where the given ID is the subject.
    pub fn triples_for_subject(&self, id: &BlankNode) -> Vec<&Triple> {
        self.triples.iter().filter(|t| &t.subject == id).collect()
    }

    /// Get all triples where the given ID appears as subject or object.
    pub fn triples_referencing(&self, id: &BlankNode) -> Vec<&Triple> {
        self.triples
            .iter()
            .filter(|t| {
                &t.subject == id
                    || matches!(&t.object, TripleObject::Entity(obj_id) if obj_id == id)
            })
            .collect()
    }

    /// Get all entities with the given status.
    pub fn entities_with_status(&self, status: AnchorStatus) -> Vec<&SemanticEntity> {
        self.entities
            .iter()
            .filter(|e| e.status == status)
            .collect()
    }

    /// Format the source hash as a hex string.
    pub fn source_hash_hex(&self) -> String {
        self.source_hash
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_graph() -> SemanticGraph {
        let mut g = SemanticGraph::new([0u8; 32]);
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
        g.triples.push(Triple {
            subject: BlankNode::new("e1").unwrap(),
            predicate: NamedNode::new("http://schema.org/name").unwrap(),
            object: TripleObject::Literal {
                value: "Alice".into(),
                datatype: None,
            },
        });
        g.triples.push(Triple {
            subject: BlankNode::new("e1").unwrap(),
            predicate: NamedNode::new("http://schema.org/location").unwrap(),
            object: TripleObject::Entity(BlankNode::new("e2").unwrap()),
        });
        g
    }

    #[test]
    fn entity_lookup() {
        let g = make_graph();
        let e1 = BlankNode::new("e1").unwrap();
        assert!(g.entity_by_id(&e1).is_some());
        assert_eq!(g.entity_by_id(&e1).unwrap().types.len(), 1);

        let e3 = BlankNode::new("e3").unwrap();
        assert!(g.entity_by_id(&e3).is_none());
    }

    #[test]
    fn triples_for_subject() {
        let g = make_graph();
        let e1 = BlankNode::new("e1").unwrap();
        let triples = g.triples_for_subject(&e1);
        assert_eq!(triples.len(), 2);
    }

    #[test]
    fn triples_referencing_includes_objects() {
        let g = make_graph();
        let e2 = BlankNode::new("e2").unwrap();
        let triples = g.triples_referencing(&e2);
        // e2 appears as object in the location triple
        assert_eq!(triples.len(), 1);
    }

    #[test]
    fn source_hash_hex() {
        let mut hash = [0u8; 32];
        hash[0] = 0xa1;
        hash[1] = 0xb2;
        let g = SemanticGraph::new(hash);
        assert!(g.source_hash_hex().starts_with("a1b2"));
    }
}
