use oxrdf::NamedNode;
use std::collections::HashMap;

/// Describes a single property from an ontology.
#[derive(Debug, Clone)]
pub struct PropertyDef {
    pub iri: NamedNode,
    pub label: String,
    pub expected_type: ExpectedType,
    pub comment: Option<String>,
}

/// Expected value type for a property.
#[derive(Debug, Clone)]
pub enum ExpectedType {
    Text,
    Date,
    DateTime,
    Integer,
    Float,
    Boolean,
    Url,
    /// Another typed entity.
    Entity(NamedNode),
}

/// Describes a class/type from an ontology.
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub iri: NamedNode,
    pub label: String,
    pub parent_types: Vec<NamedNode>,
    pub properties: Vec<NamedNode>,
    pub comment: Option<String>,
}

/// An ontology provider supplies type and property definitions.
pub trait OntologyProvider: Send + Sync {
    fn prefix(&self) -> &str;
    fn base_iri(&self) -> &str;
    fn lookup_type(&self, local_name: &str) -> Option<&TypeDef>;
    fn lookup_property(&self, local_name: &str) -> Option<&PropertyDef>;
    fn all_types(&self) -> Vec<&TypeDef>;
    fn all_properties(&self) -> Vec<&PropertyDef>;
}

/// Validation result for property-type checking.
#[derive(Debug)]
pub enum ValidationResult {
    /// Property is valid for this type.
    Valid,
    /// Property exists but is not defined for this type.
    NotDefinedForType {
        property: NamedNode,
        type_iri: NamedNode,
    },
    /// Property is unknown in all registered ontologies.
    UnknownProperty(NamedNode),
    /// Type is unknown in all registered ontologies.
    UnknownType(NamedNode),
}

/// Central registry of ontology providers.
#[derive(Default)]
pub struct ThemeRegistry {
    providers: HashMap<String, Box<dyn OntologyProvider>>,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Box<dyn OntologyProvider>) {
        self.providers
            .insert(provider.prefix().to_string(), provider);
    }

    /// Create a registry with built-in ontologies (Schema.org, Dublin Core, FOAF).
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(
            crate::builtins::schema_org::SchemaOrgProvider::new(),
        ));
        reg.register(Box::new(
            crate::builtins::dublin_core::DublinCoreProvider::new(),
        ));
        reg.register(Box::new(crate::builtins::foaf::FoafProvider::new()));
        reg.register(Box::new(
            crate::builtins::sparkdown::SparkdownProvider::new(),
        ));
        reg
    }

    /// Look up a type by prefix and local name.
    pub fn lookup_type(&self, prefix: &str, local: &str) -> Option<&TypeDef> {
        self.providers.get(prefix)?.lookup_type(local)
    }

    /// Look up a property by prefix and local name.
    pub fn lookup_property(&self, prefix: &str, local: &str) -> Option<&PropertyDef> {
        self.providers.get(prefix)?.lookup_property(local)
    }

    /// Validate that a property is appropriate for a given type.
    pub fn validate_property_for_type(
        &self,
        type_iri: &NamedNode,
        prop_iri: &NamedNode,
    ) -> ValidationResult {
        // Find the type
        let type_str = type_iri.as_str();
        let mut found_type = false;

        for provider in self.providers.values() {
            let base = provider.base_iri();
            if type_str.starts_with(base) {
                let local = &type_str[base.len()..];
                if let Some(typedef) = provider.lookup_type(local) {
                    found_type = true;
                    if typedef.properties.contains(prop_iri) {
                        return ValidationResult::Valid;
                    }
                    // Check parent types
                    for parent in &typedef.parent_types {
                        let parent_str = parent.as_str();
                        if parent_str.starts_with(base) {
                            let parent_local = &parent_str[base.len()..];
                            if let Some(parent_def) = provider.lookup_type(parent_local) {
                                if parent_def.properties.contains(prop_iri) {
                                    return ValidationResult::Valid;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !found_type {
            return ValidationResult::UnknownType(type_iri.clone());
        }

        // Check if property exists at all
        let prop_str = prop_iri.as_str();
        for provider in self.providers.values() {
            let base = provider.base_iri();
            if prop_str.starts_with(base) {
                let local = &prop_str[base.len()..];
                if provider.lookup_property(local).is_some() {
                    return ValidationResult::NotDefinedForType {
                        property: prop_iri.clone(),
                        type_iri: type_iri.clone(),
                    };
                }
            }
        }

        ValidationResult::UnknownProperty(prop_iri.clone())
    }

    /// Get all registered prefixes.
    pub fn prefixes(&self) -> Vec<(&str, &str)> {
        self.providers
            .values()
            .map(|p| (p.prefix(), p.base_iri()))
            .collect()
    }

    /// Returns all type categories across all registered providers,
    /// structured for UI consumption.
    /// Returns (prefix, base_iri, Vec<(curie, local_name, &TypeDef)>) per provider.
    pub fn all_type_categories(&self) -> Vec<(String, String, Vec<(String, String, &TypeDef)>)> {
        self.providers
            .iter()
            .map(|(prefix, provider)| {
                let types: Vec<_> = provider
                    .all_types()
                    .into_iter()
                    .map(|t| {
                        let local = t
                            .iri
                            .as_str()
                            .strip_prefix(provider.base_iri())
                            .unwrap_or(t.iri.as_str())
                            .to_string();
                        let curie = format!("{}:{}", prefix, local);
                        (curie, local, t)
                    })
                    .collect();
                (prefix.clone(), provider.base_iri().to_string(), types)
            })
            .collect()
    }

    /// Search types by query string across all providers. Returns up to `limit` results.
    pub fn search_types(&self, query: &str, limit: usize) -> Vec<(&str, &TypeDef)> {
        let query_lower = query.to_lowercase();
        let mut results = vec![];
        for (prefix, provider) in &self.providers {
            for t in provider.all_types() {
                if results.len() >= limit {
                    return results;
                }
                if t.label.to_lowercase().contains(&query_lower)
                    || t.iri.as_str().to_lowercase().contains(&query_lower)
                {
                    results.push((prefix.as_str(), t));
                }
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_schema_org_type() {
        let reg = ThemeRegistry::with_builtins();
        let t = reg.lookup_type("schema", "Event");
        assert!(t.is_some());
        assert_eq!(t.unwrap().label, "Event");
    }

    #[test]
    fn lookup_schema_org_property() {
        let reg = ThemeRegistry::with_builtins();
        let p = reg.lookup_property("schema", "name");
        assert!(p.is_some());
    }

    #[test]
    fn lookup_unknown_type() {
        let reg = ThemeRegistry::with_builtins();
        assert!(reg.lookup_type("schema", "NonExistent").is_none());
    }

    #[test]
    fn lookup_dc_property() {
        let reg = ThemeRegistry::with_builtins();
        let p = reg.lookup_property("dc", "title");
        assert!(p.is_some());
    }

    #[test]
    fn lookup_foaf_type() {
        let reg = ThemeRegistry::with_builtins();
        let t = reg.lookup_type("foaf", "Person");
        assert!(t.is_some());
    }
}
