use oxrdf::NamedNode;
use std::collections::HashMap;

use crate::registry::{ExpectedType, OntologyProvider, PropertyDef, TypeDef};

const BASE: &str = "http://xmlns.com/foaf/0.1/";

pub struct FoafProvider {
    types: HashMap<String, TypeDef>,
    properties: HashMap<String, PropertyDef>,
}

impl FoafProvider {
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
            ("name", Text, "A name for some thing"),
            ("mbox", Url, "A personal mailbox (mailto: URI)"),
            ("homepage", Url, "A homepage for some thing"),
            ("knows", Entity(Self::iri("Person")), "A person known by this person"),
            ("member", Entity(Self::iri("Agent")), "A member of a group"),
            ("depiction", Url, "A depiction of some thing"),
            ("nick", Text, "A short informal nickname"),
            ("title", Text, "A title (Mr, Ms, Dr, etc.)"),
            ("firstName", Text, "The first name of a person"),
            ("lastName", Text, "The last name of a person"),
            ("age", Integer, "The age in years"),
            ("interest", Entity(Self::iri("Document")), "A page about a topic of interest"),
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
        let agent_props: Vec<NamedNode> = ["name", "mbox", "homepage"]
            .iter()
            .map(|p| Self::iri(p))
            .collect();

        self.types.insert(
            "Agent".to_string(),
            TypeDef {
                iri: Self::iri("Agent"),
                label: "Agent".to_string(),
                parent_types: vec![],
                properties: agent_props.clone(),
                comment: Some("An agent (person, group, software, etc.)".to_string()),
            },
        );

        self.types.insert(
            "Person".to_string(),
            TypeDef {
                iri: Self::iri("Person"),
                label: "Person".to_string(),
                parent_types: vec![Self::iri("Agent")],
                properties: [
                    "name", "mbox", "homepage", "knows", "nick", "title",
                    "firstName", "lastName", "age", "depiction", "interest",
                ]
                .iter()
                .map(|p| Self::iri(p))
                .collect(),
                comment: Some("A person".to_string()),
            },
        );

        self.types.insert(
            "Organization".to_string(),
            TypeDef {
                iri: Self::iri("Organization"),
                label: "Organization".to_string(),
                parent_types: vec![Self::iri("Agent")],
                properties: ["name", "mbox", "homepage", "member"]
                    .iter()
                    .map(|p| Self::iri(p))
                    .collect(),
                comment: Some("An organization".to_string()),
            },
        );

        self.types.insert(
            "Document".to_string(),
            TypeDef {
                iri: Self::iri("Document"),
                label: "Document".to_string(),
                parent_types: vec![],
                properties: vec![],
                comment: Some("A document".to_string()),
            },
        );

        self.types.insert(
            "Image".to_string(),
            TypeDef {
                iri: Self::iri("Image"),
                label: "Image".to_string(),
                parent_types: vec![Self::iri("Document")],
                properties: vec![Self::iri("depiction")],
                comment: Some("An image".to_string()),
            },
        );
    }
}

impl OntologyProvider for FoafProvider {
    fn prefix(&self) -> &str {
        "foaf"
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
