use oxrdf::NamedNode;
use std::collections::BTreeMap;

use crate::error::SparkdownError;

/// Maps prefix strings to base IRIs for CURIE resolution.
#[derive(Debug, Clone, Default)]
pub struct PrefixMap {
    map: BTreeMap<String, String>,
}

impl PrefixMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a prefix binding.
    pub fn insert(&mut self, prefix: impl Into<String>, iri: impl Into<String>) {
        self.map.insert(prefix.into(), iri.into());
    }

    /// Resolve a CURIE like "schema:Person" to a full NamedNode IRI.
    pub fn resolve(&self, curie: &str) -> Result<NamedNode, SparkdownError> {
        if let Some((prefix, local)) = curie.split_once(':') {
            // Check for full IRIs (http://, https://)
            if prefix == "http" || prefix == "https" {
                return NamedNode::new(curie)
                    .map_err(|_| SparkdownError::InvalidIri(curie.to_string()));
            }
            if let Some(base) = self.map.get(prefix) {
                let iri = format!("{base}{local}");
                NamedNode::new(&iri).map_err(|_| SparkdownError::InvalidIri(iri))
            } else {
                Err(SparkdownError::UnknownPrefix(prefix.to_string()))
            }
        } else {
            NamedNode::new(curie).map_err(|_| SparkdownError::InvalidIri(curie.to_string()))
        }
    }

    /// Try to resolve; returns None for unknown prefixes instead of error.
    pub fn try_resolve(&self, curie: &str) -> Option<NamedNode> {
        self.resolve(curie).ok()
    }

    /// Seed with well-known prefixes.
    pub fn seed_builtins(&mut self) {
        self.insert("schema", "http://schema.org/");
        self.insert("dc", "http://purl.org/dc/terms/");
        self.insert("foaf", "http://xmlns.com/foaf/0.1/");
        self.insert("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
        self.insert("rdfs", "http://www.w3.org/2000/01/rdf-schema#");
        self.insert("owl", "http://www.w3.org/2002/07/owl#");
        self.insert("xsd", "http://www.w3.org/2001/XMLSchema#");
        self.insert("wikidata", "http://www.wikidata.org/entity/");
        self.insert("skos", "http://www.w3.org/2004/02/skos/core#");
    }

    /// Iterate over all prefix bindings.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Get the base IRI for a prefix.
    pub fn get(&self, prefix: &str) -> Option<&str> {
        self.map.get(prefix).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_known_curie() {
        let mut pm = PrefixMap::new();
        pm.seed_builtins();
        let node = pm.resolve("schema:Person").unwrap();
        assert_eq!(node.as_str(), "http://schema.org/Person");
    }

    #[test]
    fn resolve_unknown_prefix_errors() {
        let pm = PrefixMap::new();
        assert!(pm.resolve("unknown:Thing").is_err());
    }

    #[test]
    fn resolve_full_iri() {
        let pm = PrefixMap::new();
        let node = pm.resolve("http://example.org/Thing").unwrap();
        assert_eq!(node.as_str(), "http://example.org/Thing");
    }

    #[test]
    fn insert_override() {
        let mut pm = PrefixMap::new();
        pm.insert("ex", "http://example.org/");
        pm.insert("ex", "http://example.com/");
        let node = pm.resolve("ex:Foo").unwrap();
        assert_eq!(node.as_str(), "http://example.com/Foo");
    }
}
