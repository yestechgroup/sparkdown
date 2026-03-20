use pulldown_cmark::{Options, Parser};

use crate::ast::SparkdownDocument;
use crate::error::SparkdownError;
use crate::frontmatter::parse_frontmatter;
use crate::postprocess::build_semantic_ast;
use crate::prefix::PrefixMap;
use crate::preprocess::preprocess;

/// Main sparkdown parser. Configurable with prefix mappings.
pub struct SparkdownParser {
    pub prefix_map: PrefixMap,
}

impl Default for SparkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SparkdownParser {
    /// Create a new parser with built-in prefixes seeded.
    pub fn new() -> Self {
        let mut prefix_map = PrefixMap::new();
        prefix_map.seed_builtins();
        Self { prefix_map }
    }

    /// Parse a sparkdown document from source text.
    pub fn parse(&self, source: &str) -> Result<SparkdownDocument, SparkdownError> {
        // 1. Extract and parse frontmatter
        let (frontmatter, content) = parse_frontmatter(source);

        // 2. Merge frontmatter prefixes into prefix map
        let mut prefixes = self.prefix_map.clone();
        for (k, v) in &frontmatter.prefixes {
            prefixes.insert(k.clone(), v.clone());
        }

        // 3. Pre-process the content (directive transformation)
        let pre = preprocess(&content);

        // 4. Merge link-ref-def prefixes
        for (k, v) in &pre.link_ref_prefixes {
            prefixes.insert(k.clone(), v.clone());
        }

        // 5. Parse with pulldown-cmark
        let mut options = Options::empty();
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
        options.insert(Options::ENABLE_TASKLISTS);

        let events: Vec<_> = Parser::new_ext(&pre.modified_source, options).collect();

        // 6. Post-process into semantic AST
        let doc = build_semantic_ast(events, &pre.directives, frontmatter, prefixes, source);

        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotation::AnnotationKind;
    use crate::ast::NodeKind;

    #[test]
    fn full_pipeline() {
        let source = r#"---
title: Test Event
prefixes:
  schema: http://schema.org/
"@type": schema:Event
---

# The Conference {.schema:Event startDate=2026-03-20}

:entity[Albert Einstein]{type=schema:Person wikidata=Q937} spoke at the event.

::: schema:Review
This was a great conference.
:::
"#;
        let parser = SparkdownParser::new();
        let doc = parser.parse(source).unwrap();

        // Check frontmatter
        assert_eq!(doc.frontmatter.title.as_deref(), Some("Test Event"));
        assert_eq!(doc.frontmatter.doc_type.as_deref(), Some("schema:Event"));

        // Check we got nodes
        assert!(!doc.nodes.is_empty());

        // Find the section node
        let section = doc.nodes.iter().find(|n| matches!(&n.kind, NodeKind::Section { .. }));
        assert!(section.is_some(), "should have a section node");
        let section = section.unwrap();

        // Check section has type annotation
        let has_type = section.annotations.iter().any(|a| {
            matches!(&a.kind, AnnotationKind::TypeAssignment { raw, .. } if raw == "schema:Event")
        });
        assert!(has_type, "section should have schema:Event type");

        // Find inline directive
        fn find_node<'a>(nodes: &'a [crate::ast::SemanticNode], pred: &dyn Fn(&crate::ast::NodeKind) -> bool) -> Option<&'a crate::ast::SemanticNode> {
            for n in nodes {
                if pred(&n.kind) {
                    return Some(n);
                }
                if let Some(found) = find_node(&n.children, pred) {
                    return Some(found);
                }
            }
            None
        }

        let directive = find_node(&doc.nodes, &|k| matches!(k, NodeKind::InlineDirective { name } if name == "entity"));
        assert!(directive.is_some(), "should have an inline directive");

        // Find block directive
        let block = find_node(&doc.nodes, &|k| matches!(k, NodeKind::DirectiveBlock { .. }));
        assert!(block.is_some(), "should have a block directive");
    }
}
