use oxrdf::NamedNode;
use std::collections::HashMap;

use crate::registry::{ExpectedType, OntologyProvider, PropertyDef, TypeDef};

const BASE: &str = "http://purl.org/dc/terms/";

pub struct DublinCoreProvider {
    types: HashMap<String, TypeDef>,
    properties: HashMap<String, PropertyDef>,
}

impl DublinCoreProvider {
    pub fn new() -> Self {
        let mut provider = Self {
            types: HashMap::new(),
            properties: HashMap::new(),
        };
        provider.register_properties();
        provider
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new(format!("{BASE}{local}")).unwrap()
    }

    fn register_properties(&mut self) {
        use ExpectedType::*;
        let props = [
            ("title", Text, "A name given to the resource"),
            ("creator", Text, "An entity primarily responsible for making the resource"),
            ("subject", Text, "The topic of the resource"),
            ("description", Text, "An account of the resource"),
            ("publisher", Text, "An entity responsible for making the resource available"),
            ("contributor", Text, "An entity responsible for making contributions"),
            ("date", Date, "A point or period of time associated with an event"),
            ("type", Text, "The nature or genre of the resource"),
            ("format", Text, "The file format or physical medium"),
            ("identifier", Text, "An unambiguous reference to the resource"),
            ("source", Url, "A related resource from which the resource is derived"),
            ("language", Text, "A language of the resource"),
            ("relation", Url, "A related resource"),
            ("coverage", Text, "The spatial or temporal topic of the resource"),
            ("rights", Text, "Information about rights held in and over the resource"),
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
}

impl OntologyProvider for DublinCoreProvider {
    fn prefix(&self) -> &str {
        "dc"
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
