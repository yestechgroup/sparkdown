use oxrdf::NamedNode;
use std::collections::HashMap;

use crate::registry::{ExpectedType, OntologyProvider, PropertyDef, TypeDef};

const BASE: &str = "urn:sparkdown:vocab/";

pub struct SparkdownProvider {
    types: HashMap<String, TypeDef>,
    properties: HashMap<String, PropertyDef>,
}

impl SparkdownProvider {
    pub fn new() -> Self {
        let mut provider = Self {
            types: HashMap::new(),
            properties: HashMap::new(),
        };
        provider.register_properties();
        provider.register_types();
        provider
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new(format!("{BASE}{local}")).unwrap()
    }

    fn register_properties(&mut self) {
        use ExpectedType::*;
        let props = [
            ("role", Entity(Self::iri("Section")), "Links structure to rhetorical function"),
            ("snippet", Text, "Short content fingerprint for staleness verification"),
        ];

        for (local, expected, comment) in props {
            self.properties.insert(
                local.to_string(),
                PropertyDef {
                    iri: Self::iri(local),
                    label: local.to_string(),
                    expected_type: expected,
                    comment: Some(comment.to_string()),
                },
            );
        }
    }

    fn register_types(&mut self) {
        let section_props: Vec<NamedNode> = ["role", "snippet"]
            .iter()
            .map(|p| Self::iri(p))
            .collect();

        self.types.insert(
            "Section".to_string(),
            TypeDef {
                iri: Self::iri("Section"),
                label: "Section".to_string(),
                parent_types: vec![],
                properties: section_props.clone(),
                comment: Some("A structural section of the document".to_string()),
            },
        );

        self.types.insert(
            "Paragraph".to_string(),
            TypeDef {
                iri: Self::iri("Paragraph"),
                label: "Paragraph".to_string(),
                parent_types: vec![],
                properties: vec![Self::iri("role"), Self::iri("snippet")],
                comment: Some("A paragraph-level annotation target".to_string()),
            },
        );

        // Rhetorical role types
        let roles = [
            ("Review", "Rhetorical role: review/opinion"),
            ("Abstract", "Rhetorical role: summary/abstract"),
            ("Argument", "Rhetorical role: argumentative content"),
            ("Summary", "Rhetorical role: summarization"),
            ("Comparison", "Rhetorical role: comparative analysis"),
            ("Example", "Rhetorical role: illustrative example"),
        ];

        for (local, comment) in roles {
            self.types.insert(
                local.to_string(),
                TypeDef {
                    iri: Self::iri(local),
                    label: local.to_string(),
                    parent_types: vec![],
                    properties: vec![],
                    comment: Some(comment.to_string()),
                },
            );
        }
    }
}

impl OntologyProvider for SparkdownProvider {
    fn prefix(&self) -> &str {
        "sd"
    }

    fn base_iri(&self) -> &str {
        BASE
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
