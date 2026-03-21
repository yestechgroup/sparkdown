use serde::Deserialize;
use std::collections::HashMap;

use oxrdf::NamedNode;

use crate::registry::{ExpectedType, OntologyProvider, PropertyDef, TypeDef};

// ── pack.toml raw TOML structures ──

#[derive(Debug, Deserialize)]
struct PackToml {
    pack: PackSection,
    prefixes: Option<HashMap<String, String>>,
    categories: Option<HashMap<String, CategorySection>>,
}

#[derive(Debug, Deserialize)]
struct PackSection {
    name: String,
    version: String,
    description: String,
    source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CategorySection {
    label: String,
    types: Vec<String>,
}

// ── types.toml raw TOML structures ──

#[derive(Debug, Deserialize)]
struct TypesToml {
    types: Vec<TypeEntry>,
}

#[derive(Debug, Deserialize)]
struct TypeEntry {
    iri: String,
    curie: String,
    label: String,
    description: Option<String>,
    parent: Option<String>,
    #[serde(default)]
    suggested_properties: Vec<String>,
}

// ── properties.toml raw TOML structures ──

#[derive(Debug, Deserialize)]
struct PropertiesToml {
    properties: Vec<PropertyEntry>,
}

#[derive(Debug, Deserialize)]
struct PropertyEntry {
    iri: String,
    curie: String,
    label: String,
    expected_type: String,
    description: Option<String>,
}

// ── Public types ──

/// Parsed ontology pack metadata.
#[derive(Debug, Clone)]
pub struct OntologyPackMeta {
    pub metadata: PackMetadata,
    pub prefixes: HashMap<String, String>,
    pub categories: Vec<TypeCategory>,
}

#[derive(Debug, Clone)]
pub struct PackMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TypeCategory {
    pub key: String,
    pub label: String,
    pub types: Vec<String>,
}

/// A type definition from a pack's types.toml.
#[derive(Debug, Clone)]
pub struct PackTypeDef {
    pub iri: String,
    pub curie: String,
    pub label: String,
    pub description: Option<String>,
    pub parent: Option<String>,
    pub suggested_properties: Vec<String>,
}

/// A property definition from a pack's properties.toml.
#[derive(Debug, Clone)]
pub struct PackPropertyDef {
    pub iri: String,
    pub curie: String,
    pub label: String,
    pub expected_type: String,
    pub description: Option<String>,
}

// ── Parsers ──

/// Parse a pack.toml string into an OntologyPackMeta.
pub fn parse_pack_toml(input: &str) -> Result<OntologyPackMeta, String> {
    let raw: PackToml = toml::from_str(input).map_err(|e| format!("TOML parse error: {e}"))?;

    let categories = match raw.categories {
        Some(cats) => cats
            .into_iter()
            .map(|(key, section)| TypeCategory {
                key,
                label: section.label,
                types: section.types,
            })
            .collect(),
        None => vec![],
    };

    Ok(OntologyPackMeta {
        metadata: PackMetadata {
            name: raw.pack.name,
            version: raw.pack.version,
            description: raw.pack.description,
            source: raw.pack.source,
        },
        prefixes: raw.prefixes.unwrap_or_default(),
        categories,
    })
}

/// Parse a types.toml string into a list of PackTypeDef.
pub fn parse_types_toml(input: &str) -> Result<Vec<PackTypeDef>, String> {
    let raw: TypesToml = toml::from_str(input).map_err(|e| format!("TOML parse error: {e}"))?;
    Ok(raw
        .types
        .into_iter()
        .map(|t| PackTypeDef {
            iri: t.iri,
            curie: t.curie,
            label: t.label,
            description: t.description,
            parent: t.parent,
            suggested_properties: t.suggested_properties,
        })
        .collect())
}

/// Parse a properties.toml string into a list of PackPropertyDef.
pub fn parse_properties_toml(input: &str) -> Result<Vec<PackPropertyDef>, String> {
    let raw: PropertiesToml =
        toml::from_str(input).map_err(|e| format!("TOML parse error: {e}"))?;
    Ok(raw
        .properties
        .into_iter()
        .map(|p| PackPropertyDef {
            iri: p.iri,
            curie: p.curie,
            label: p.label,
            expected_type: p.expected_type,
            description: p.description,
        })
        .collect())
}

// ── TomlOntologyProvider ──

/// An OntologyProvider backed by TOML pack definitions.
pub struct TomlOntologyProvider {
    prefix_str: String,
    base: String,
    types: HashMap<String, TypeDef>,
    properties: HashMap<String, PropertyDef>,
}

impl TomlOntologyProvider {
    pub fn new(
        prefix: &str,
        base_iri: &str,
        type_defs: Vec<PackTypeDef>,
        prop_defs: Vec<PackPropertyDef>,
    ) -> Self {
        let mut types = HashMap::new();
        for t in type_defs {
            let local = t
                .iri
                .strip_prefix(base_iri)
                .unwrap_or(&t.label)
                .to_string();
            types.insert(
                local,
                TypeDef {
                    iri: NamedNode::new_unchecked(&t.iri),
                    label: t.label,
                    parent_types: t
                        .parent
                        .iter()
                        .map(|p| NamedNode::new_unchecked(p))
                        .collect(),
                    properties: t
                        .suggested_properties
                        .iter()
                        .map(|p| NamedNode::new_unchecked(p))
                        .collect(),
                    comment: t.description,
                },
            );
        }
        let mut properties = HashMap::new();
        for p in prop_defs {
            let local = p
                .iri
                .strip_prefix(base_iri)
                .unwrap_or(&p.label)
                .to_string();
            let expected = match p.expected_type.as_str() {
                "Text" => ExpectedType::Text,
                "Date" => ExpectedType::Date,
                "DateTime" => ExpectedType::DateTime,
                "Integer" => ExpectedType::Integer,
                "Float" => ExpectedType::Float,
                "Boolean" => ExpectedType::Boolean,
                "Url" => ExpectedType::Url,
                "Entity" => ExpectedType::Entity(NamedNode::new_unchecked(
                    "http://www.w3.org/2002/07/owl#Thing",
                )),
                _ => ExpectedType::Text,
            };
            properties.insert(
                local,
                PropertyDef {
                    iri: NamedNode::new_unchecked(&p.iri),
                    label: p.label,
                    expected_type: expected,
                    comment: p.description,
                },
            );
        }
        Self {
            prefix_str: prefix.to_string(),
            base: base_iri.to_string(),
            types,
            properties,
        }
    }
}

impl OntologyProvider for TomlOntologyProvider {
    fn prefix(&self) -> &str {
        &self.prefix_str
    }
    fn base_iri(&self) -> &str {
        &self.base
    }
    fn lookup_type(&self, local_name: &str) -> Option<&TypeDef> {
        self.types.get(local_name)
    }
    fn lookup_property(&self, local_name: &str) -> Option<&PropertyDef> {
        self.properties.get(local_name)
    }
    fn all_types(&self) -> Vec<&TypeDef> {
        self.types.values().collect()
    }
    fn all_properties(&self) -> Vec<&PropertyDef> {
        self.properties.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pack_metadata() {
        let toml_str = r#"
[pack]
name = "test-ontology"
version = "1.0.0"
description = "A test ontology pack"

[prefixes]
test = "http://example.org/test/"

[categories]
people = { label = "People", types = ["test:Person", "test:Agent"] }
"#;
        let pack = parse_pack_toml(toml_str).unwrap();
        assert_eq!(pack.metadata.name, "test-ontology");
        assert_eq!(pack.metadata.version, "1.0.0");
        assert_eq!(
            pack.prefixes.get("test").unwrap(),
            "http://example.org/test/"
        );
        assert_eq!(pack.categories.len(), 1);
        assert_eq!(pack.categories[0].label, "People");
        assert_eq!(
            pack.categories[0].types,
            vec!["test:Person", "test:Agent"]
        );
    }

    #[test]
    fn parse_types_toml_test() {
        let toml_str = r#"
[[types]]
iri = "http://example.org/test/Person"
curie = "test:Person"
label = "Person"
description = "A human being"
suggested_properties = ["test:name", "test:age"]

[[types]]
iri = "http://example.org/test/Agent"
curie = "test:Agent"
label = "Agent"
"#;
        let types = parse_types_toml(toml_str).unwrap();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0].label, "Person");
        assert_eq!(types[0].iri, "http://example.org/test/Person");
        assert_eq!(
            types[0].suggested_properties,
            vec!["test:name", "test:age"]
        );
        assert_eq!(types[1].label, "Agent");
        assert!(types[1].suggested_properties.is_empty());
    }

    #[test]
    fn parse_properties_toml_test() {
        let toml_str = r#"
[[properties]]
iri = "http://example.org/test/name"
curie = "test:name"
label = "Name"
expected_type = "Text"

[[properties]]
iri = "http://example.org/test/knows"
curie = "test:knows"
label = "Knows"
expected_type = "Entity"
"#;
        let props = parse_properties_toml(toml_str).unwrap();
        assert_eq!(props.len(), 2);
        assert_eq!(props[0].label, "Name");
        assert_eq!(props[0].expected_type, "Text");
        assert_eq!(props[1].expected_type, "Entity");
    }

    #[test]
    fn toml_provider_lookups() {
        let types = vec![PackTypeDef {
            iri: "http://example.org/test/Person".to_string(),
            curie: "test:Person".to_string(),
            label: "Person".to_string(),
            description: Some("A human".to_string()),
            parent: None,
            suggested_properties: vec![],
        }];
        let props = vec![PackPropertyDef {
            iri: "http://example.org/test/name".to_string(),
            curie: "test:name".to_string(),
            label: "Name".to_string(),
            expected_type: "Text".to_string(),
            description: None,
        }];
        let provider =
            TomlOntologyProvider::new("test", "http://example.org/test/", types, props);

        assert_eq!(provider.prefix(), "test");
        assert_eq!(provider.base_iri(), "http://example.org/test/");
        assert!(provider.lookup_type("Person").is_some());
        assert!(provider.lookup_type("Unknown").is_none());
        assert!(provider.lookup_property("name").is_some());
        assert_eq!(provider.all_types().len(), 1);
        assert_eq!(provider.all_properties().len(), 1);
    }
}
