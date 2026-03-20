use oxrdf::NamedNode;
use std::collections::HashMap;

use crate::registry::{ExpectedType, OntologyProvider, PropertyDef, TypeDef};

const BASE: &str = "http://schema.org/";

pub struct SchemaOrgProvider {
    types: HashMap<String, TypeDef>,
    properties: HashMap<String, PropertyDef>,
}

impl SchemaOrgProvider {
    pub fn new() -> Self {
        let mut provider = Self {
            types: HashMap::new(),
            properties: HashMap::new(),
        };
        provider.register_properties();
        provider.register_types();
        provider
    }

    fn iri(&self, local: &str) -> NamedNode {
        NamedNode::new(format!("{BASE}{local}")).unwrap()
    }

    fn prop(&mut self, local: &str, expected: ExpectedType, comment: &str) {
        self.properties.insert(
            local.to_string(),
            PropertyDef {
                iri: self.iri(local),
                label: local.to_string(),
                expected_type: expected,
                comment: Some(comment.to_string()),
            },
        );
    }

    fn typ(
        &mut self,
        local: &str,
        parents: &[&str],
        props: &[&str],
        comment: &str,
    ) {
        let parent_iris: Vec<NamedNode> = parents.iter().map(|p| self.iri(p)).collect();
        let prop_iris: Vec<NamedNode> = props.iter().map(|p| self.iri(p)).collect();
        self.types.insert(
            local.to_string(),
            TypeDef {
                iri: self.iri(local),
                label: local.to_string(),
                parent_types: parent_iris,
                properties: prop_iris,
                comment: Some(comment.to_string()),
            },
        );
    }

    fn register_properties(&mut self) {
        use ExpectedType::*;
        self.prop("name", Text, "The name of the item");
        self.prop("description", Text, "A description of the item");
        self.prop("url", Url, "URL of the item");
        self.prop("image", Url, "An image of the item");
        self.prop("datePublished", Date, "Date of first publication");
        self.prop("dateModified", Date, "Date of last modification");
        self.prop("dateCreated", Date, "Date of creation");
        self.prop("author", Entity(self.iri("Person")), "The author");
        self.prop("creator", Entity(self.iri("Person")), "The creator");
        self.prop("publisher", Entity(self.iri("Organization")), "The publisher");
        self.prop("startDate", Date, "The start date");
        self.prop("endDate", Date, "The end date");
        self.prop("location", Entity(self.iri("Place")), "The location");
        self.prop("performer", Entity(self.iri("Person")), "A performer");
        self.prop("attendee", Entity(self.iri("Person")), "An attendee");
        self.prop("organizer", Entity(self.iri("Organization")), "The organizer");
        self.prop("reviewBody", Text, "The body of the review");
        self.prop("ratingValue", Float, "A rating value");
        self.prop("bestRating", Float, "Best possible rating");
        self.prop("worstRating", Float, "Worst possible rating");
        self.prop("itemReviewed", Entity(self.iri("Thing")), "The item reviewed");
        self.prop("price", Text, "The price");
        self.prop("priceCurrency", Text, "The currency of the price");
        self.prop("brand", Entity(self.iri("Organization")), "The brand");
        self.prop("headline", Text, "Headline of the article");
        self.prop("articleBody", Text, "The body of the article");
        self.prop("wordCount", Integer, "Word count");
        self.prop("genre", Text, "Genre of the work");
        self.prop("inLanguage", Text, "The language of the content");
        self.prop("keywords", Text, "Keywords or tags");
        self.prop("about", Entity(self.iri("Thing")), "The subject matter");
        self.prop("email", Text, "Email address");
        self.prop("telephone", Text, "Telephone number");
        self.prop("address", Entity(self.iri("PostalAddress")), "Physical address");
        self.prop("jobTitle", Text, "Job title");
        self.prop("memberOf", Entity(self.iri("Organization")), "Member of organization");
        self.prop("duration", Text, "Duration (ISO 8601)");
        self.prop("identifier", Text, "An identifier");
        self.prop("sameAs", Url, "URL of a reference page that identifies the item");
    }

    fn register_types(&mut self) {
        let thing_props = &[
            "name", "description", "url", "image", "identifier", "sameAs",
        ];

        self.typ("Thing", &[], thing_props, "The most generic type");

        self.typ(
            "CreativeWork",
            &["Thing"],
            &[
                "author", "creator", "publisher", "datePublished", "dateModified",
                "dateCreated", "headline", "keywords", "inLanguage", "genre", "about",
            ],
            "The most generic kind of creative work",
        );

        self.typ(
            "Article",
            &["CreativeWork"],
            &["articleBody", "wordCount"],
            "An article",
        );

        self.typ(
            "BlogPosting",
            &["Article"],
            &[],
            "A blog post",
        );

        self.typ(
            "Event",
            &["Thing"],
            &[
                "startDate", "endDate", "location", "performer", "attendee",
                "organizer", "duration",
            ],
            "An event",
        );

        self.typ(
            "Person",
            &["Thing"],
            &["email", "telephone", "jobTitle", "memberOf"],
            "A person",
        );

        self.typ(
            "Organization",
            &["Thing"],
            &["email", "telephone", "address"],
            "An organization",
        );

        self.typ(
            "Place",
            &["Thing"],
            &["address", "telephone"],
            "A place",
        );

        self.typ(
            "Product",
            &["Thing"],
            &["brand", "price", "priceCurrency"],
            "A product",
        );

        self.typ(
            "Review",
            &["CreativeWork"],
            &["reviewBody", "ratingValue", "bestRating", "worstRating", "itemReviewed"],
            "A review",
        );

        self.typ("HowTo", &["CreativeWork"], &["duration"], "How-to instructions");
        self.typ("FAQPage", &["CreativeWork"], &[], "A FAQ page");
        self.typ("Recipe", &["HowTo"], &["duration"], "A recipe");
        self.typ("Course", &["CreativeWork"], &[], "A course");
        self.typ("Book", &["CreativeWork"], &[], "A book");
        self.typ("Movie", &["CreativeWork"], &["duration"], "A movie");
        self.typ("MusicRecording", &["CreativeWork"], &["duration"], "A music recording");
        self.typ("SoftwareApplication", &["CreativeWork"], &[], "A software application");
        self.typ("WebPage", &["CreativeWork"], &[], "A web page");
        self.typ("ImageObject", &["CreativeWork"], &[], "An image file");
        self.typ("VideoObject", &["CreativeWork"], &["duration"], "A video file");
        self.typ("PostalAddress", &["Thing"], &[], "A postal address");
    }
}

impl OntologyProvider for SchemaOrgProvider {
    fn prefix(&self) -> &str {
        "schema"
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
