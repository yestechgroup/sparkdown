use pulldown_cmark::{Event, Tag, TagEnd};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

use crate::annotation::{parse_attr_string, Annotation, AnnotationKind};
use crate::ast::{NodeKind, SemanticNode, SparkdownDocument};
use crate::frontmatter::SparkdownFrontmatter;
use crate::prefix::PrefixMap;
use crate::preprocess::ExtractedDirective;

static BLOCK_START_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"sparkdown:block:start id=(\w+) name=(\S+)").unwrap()
});

static BLOCK_END_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"sparkdown:block:end id=(\w+)").unwrap()
});

static SPAN_START_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<span data-sd="(\w+)">"#).unwrap()
});

static SPAN_END_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"</span>").unwrap()
});

/// Build a semantic AST from pulldown-cmark events and preprocessor output.
pub fn build_semantic_ast(
    events: Vec<Event<'_>>,
    directives: &HashMap<String, ExtractedDirective>,
    frontmatter: SparkdownFrontmatter,
    prefixes: PrefixMap,
    source: &str,
) -> SparkdownDocument {
    let mut builder = AstBuilder::new(directives, &prefixes);
    builder.process_events(events);

    SparkdownDocument {
        frontmatter,
        prefixes: prefixes.clone(),
        nodes: builder.root_children,
        source: source.to_string(),
    }
}

struct AstBuilder<'a> {
    /// Stack of nodes being built. Each entry is a node with its children accumulating.
    stack: Vec<SemanticNode>,
    /// Completed root-level children.
    root_children: Vec<SemanticNode>,
    /// Preprocessor directives.
    directives: &'a HashMap<String, ExtractedDirective>,
    /// Prefix map for CURIE resolution.
    prefixes: &'a PrefixMap,
    /// Currently active inline directive ID (for span placeholders).
    active_inline_directive: Option<String>,
    /// Currently active block directive ID.
    active_block_directives: Vec<String>,
}

impl<'a> AstBuilder<'a> {
    fn new(directives: &'a HashMap<String, ExtractedDirective>, prefixes: &'a PrefixMap) -> Self {
        Self {
            stack: Vec::new(),
            root_children: Vec::new(),
            directives,
            prefixes,
            active_inline_directive: None,
            active_block_directives: Vec::new(),
        }
    }

    fn process_events(&mut self, events: Vec<Event<'_>>) {
        for event in events {
            match event {
                Event::Start(tag) => self.handle_start(tag),
                Event::End(tag) => self.handle_end(tag),
                Event::Text(text) => self.push_leaf(SemanticNode::new(
                    NodeKind::Text(text.to_string()),
                    0..0,
                )),
                Event::Code(code) => self.push_leaf(SemanticNode::new(
                    NodeKind::InlineCode(code.to_string()),
                    0..0,
                )),
                Event::SoftBreak => {
                    self.push_leaf(SemanticNode::new(NodeKind::SoftBreak, 0..0))
                }
                Event::HardBreak => {
                    self.push_leaf(SemanticNode::new(NodeKind::HardBreak, 0..0))
                }
                Event::Rule => self.push_leaf(SemanticNode::new(NodeKind::Rule, 0..0)),
                Event::Html(html) | Event::InlineHtml(html) => {
                    self.handle_html(&html);
                }
                // MetadataBlock comes as Start/End Tag events; skip them
                // Frontmatter is already handled separately by the parser
                _ => {}
            }
        }
    }

    fn handle_start(&mut self, tag: Tag<'_>) {
        // Skip HtmlBlock — we handle the inner Html events in handle_html.
        // Skip MetadataBlock — frontmatter is handled separately by the parser.
        if matches!(tag, Tag::HtmlBlock | Tag::MetadataBlock(_)) {
            return;
        }

        let node = match tag {
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                let mut node = SemanticNode::new(
                    NodeKind::Section {
                        level: level as u8,
                        id: id.map(|s| s.to_string()),
                    },
                    0..0,
                );

                // Parse heading attributes into annotations
                for class in classes {
                    let class_str = class.to_string();
                    if class_str.contains(':') {
                        let resolved = self.prefixes.try_resolve(&class_str);
                        node.annotations.push(Annotation {
                            kind: AnnotationKind::TypeAssignment {
                                resolved_iri: resolved,
                                raw: class_str,
                            },
                        });
                    } else {
                        node.annotations.push(Annotation {
                            kind: AnnotationKind::CssClass(class_str),
                        });
                    }
                }

                for (key, value) in attrs {
                    let key_str = key.to_string();
                    let value_str = value.map(|v| v.to_string()).unwrap_or_default();
                    if key_str == "type" || key_str == "@type" {
                        let resolved = self.prefixes.try_resolve(&value_str);
                        node.annotations.push(Annotation {
                            kind: AnnotationKind::TypeAssignment {
                                resolved_iri: resolved,
                                raw: value_str,
                            },
                        });
                    } else {
                        let resolved = self.prefixes.try_resolve(&key_str);
                        node.annotations.push(Annotation {
                            kind: AnnotationKind::Property {
                                resolved_iri: resolved,
                                raw_key: key_str,
                                value: value_str,
                            },
                        });
                    }
                }

                node
            }
            Tag::Paragraph => SemanticNode::new(NodeKind::Paragraph, 0..0),
            Tag::BlockQuote(_) => SemanticNode::new(NodeKind::BlockQuote, 0..0),
            Tag::List(start) => SemanticNode::new(NodeKind::List { start }, 0..0),
            Tag::Item => SemanticNode::new(NodeKind::Item, 0..0),
            Tag::Table(_) => SemanticNode::new(NodeKind::Table, 0..0),
            Tag::TableHead => SemanticNode::new(NodeKind::TableHead, 0..0),
            Tag::TableRow => SemanticNode::new(NodeKind::TableRow, 0..0),
            Tag::TableCell => SemanticNode::new(NodeKind::TableCell, 0..0),
            Tag::Emphasis => SemanticNode::new(NodeKind::Emphasis, 0..0),
            Tag::Strong => SemanticNode::new(NodeKind::Strong, 0..0),
            Tag::Strikethrough => SemanticNode::new(NodeKind::Strikethrough, 0..0),
            Tag::Link { dest_url, title, .. } => SemanticNode::new(
                NodeKind::Link {
                    dest: dest_url.to_string(),
                    title: title.to_string(),
                },
                0..0,
            ),
            Tag::Image { dest_url, title, .. } => SemanticNode::new(
                NodeKind::Image {
                    dest: dest_url.to_string(),
                    title: title.to_string(),
                },
                0..0,
            ),
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let l = lang.to_string();
                        if l.is_empty() { None } else { Some(l) }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
                SemanticNode::new(
                    NodeKind::CodeBlock {
                        language,
                        content: String::new(),
                    },
                    0..0,
                )
            }
            _ => SemanticNode::new(NodeKind::Paragraph, 0..0),
        };
        self.stack.push(node);
    }

    fn handle_end(&mut self, tag: TagEnd) {
        // Skip HtmlBlock/MetadataBlock end — we don't push anything for their start.
        if matches!(tag, TagEnd::HtmlBlock | TagEnd::MetadataBlock(_)) {
            return;
        }

        if let Some(completed) = self.stack.pop() {
            self.push_leaf(completed);
        }
    }

    fn handle_html(&mut self, html: &str) {
        // Check for block directive markers
        if let Some(caps) = BLOCK_START_RE.captures(html) {
            let id = caps[1].to_string();
            let name = caps[2].to_string();

            let mut node = SemanticNode::new(NodeKind::DirectiveBlock { name: name.clone() }, 0..0);

            // Parse annotations from the directive
            if let Some(directive) = self.directives.get(&id) {
                if let Some(ref attrs) = directive.attrs_raw {
                    node.annotations = parse_attr_string(attrs, self.prefixes);
                }
                // Also parse the name as a type if it contains ':'
                if name.contains(':') {
                    let resolved = self.prefixes.try_resolve(&name);
                    node.annotations.insert(
                        0,
                        Annotation {
                            kind: AnnotationKind::TypeAssignment {
                                resolved_iri: resolved,
                                raw: name,
                            },
                        },
                    );
                }
            }

            // If we're inside a paragraph or other block, close it first
            // so the directive block lives at the same level.
            if let Some(parent) = self.stack.last() {
                if matches!(&parent.kind, NodeKind::Paragraph) {
                    if let Some(completed) = self.stack.pop() {
                        // Only push non-empty paragraphs
                        if !completed.children.is_empty() {
                            self.push_leaf(completed);
                        }
                    }
                }
            }

            self.active_block_directives.push(id);
            self.stack.push(node);
            return;
        }

        if let Some(caps) = BLOCK_END_RE.captures(html) {
            let id = caps[1].to_string();
            if self.active_block_directives.last() == Some(&id) {
                self.active_block_directives.pop();
                if let Some(completed) = self.stack.pop() {
                    self.push_leaf(completed);
                }
            }
            return;
        }

        // Check for inline directive span placeholders
        if let Some(caps) = SPAN_START_RE.captures(html) {
            let id = caps[1].to_string();
            if let Some(directive) = self.directives.get(&id) {
                let name = directive.name.clone();
                let mut node =
                    SemanticNode::new(NodeKind::InlineDirective { name: name.clone() }, 0..0);

                if let Some(ref attrs) = directive.attrs_raw {
                    node.annotations = parse_attr_string(attrs, self.prefixes);
                }

                self.active_inline_directive = Some(id);
                self.stack.push(node);
            }
            return;
        }

        if SPAN_END_RE.is_match(html) && self.active_inline_directive.is_some() {
            self.active_inline_directive = None;
            if let Some(completed) = self.stack.pop() {
                self.push_leaf(completed);
            }
            return;
        }

        // Regular HTML passthrough
        self.push_leaf(SemanticNode::new(NodeKind::Html(html.to_string()), 0..0));
    }

    fn push_leaf(&mut self, node: SemanticNode) {
        // If building a code block, accumulate text into content
        if let Some(parent) = self.stack.last_mut() {
            if let NodeKind::CodeBlock { ref mut content, .. } = parent.kind {
                if let NodeKind::Text(ref text) = node.kind {
                    content.push_str(text);
                    return;
                }
            }
        }

        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(node);
        } else {
            self.root_children.push(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    fn parse_events(source: &str) -> Vec<Event<'_>> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        Parser::new_ext(source, options).collect()
    }

    #[test]
    fn heading_with_attributes() {
        let source = "# The Conference {.schema:Event startDate=2026-03-20}";
        let events = parse_events(source);
        let mut prefixes = PrefixMap::new();
        prefixes.seed_builtins();
        let directives = HashMap::new();
        let fm = SparkdownFrontmatter::default();

        let doc = build_semantic_ast(events, &directives, fm, prefixes, source);
        assert_eq!(doc.nodes.len(), 1);

        let section = &doc.nodes[0];
        assert!(matches!(&section.kind, NodeKind::Section { level: 1, .. }));
        assert!(section.annotations.len() >= 2);

        // First annotation should be the type
        assert!(matches!(
            &section.annotations[0].kind,
            AnnotationKind::TypeAssignment { raw, .. } if raw == "schema:Event"
        ));
    }

    #[test]
    fn plain_paragraph() {
        let source = "Hello, world!";
        let events = parse_events(source);
        let directives = HashMap::new();
        let prefixes = PrefixMap::new();
        let fm = SparkdownFrontmatter::default();

        let doc = build_semantic_ast(events, &directives, fm, prefixes, source);
        assert_eq!(doc.nodes.len(), 1);
        assert!(matches!(&doc.nodes[0].kind, NodeKind::Paragraph));
    }

    #[test]
    fn inline_directive_reconstruction() {
        let mut directives = HashMap::new();
        directives.insert(
            "SD0001".to_string(),
            ExtractedDirective {
                name: "entity".to_string(),
                content: Some("Einstein".to_string()),
                attrs_raw: Some("type=schema:Person".to_string()),
                is_block: false,
            },
        );

        let source = "<span data-sd=\"SD0001\">Einstein</span>";
        let events = parse_events(source);
        let mut prefixes = PrefixMap::new();
        prefixes.seed_builtins();
        let fm = SparkdownFrontmatter::default();

        let doc = build_semantic_ast(events, &directives, fm, prefixes, source);

        // Find the inline directive node
        fn find_directive(nodes: &[SemanticNode]) -> Option<&SemanticNode> {
            for node in nodes {
                if matches!(&node.kind, NodeKind::InlineDirective { .. }) {
                    return Some(node);
                }
                if let Some(found) = find_directive(&node.children) {
                    return Some(found);
                }
            }
            None
        }

        let directive = find_directive(&doc.nodes).expect("should find inline directive");
        assert!(matches!(
            &directive.kind,
            NodeKind::InlineDirective { name } if name == "entity"
        ));
        assert!(!directive.annotations.is_empty());
    }
}
