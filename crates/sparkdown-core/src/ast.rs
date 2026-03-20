use crate::annotation::Annotation;
use crate::frontmatter::SparkdownFrontmatter;
use crate::prefix::PrefixMap;
use std::ops::Range;

/// A fully parsed sparkdown document.
#[derive(Debug, Clone)]
pub struct SparkdownDocument {
    pub frontmatter: SparkdownFrontmatter,
    pub prefixes: PrefixMap,
    pub nodes: Vec<SemanticNode>,
    /// Raw source text, retained for round-trip.
    pub source: String,
}

/// A node in the semantic AST.
#[derive(Debug, Clone)]
pub struct SemanticNode {
    pub kind: NodeKind,
    pub annotations: Vec<Annotation>,
    pub children: Vec<SemanticNode>,
    /// Byte range in source for round-trip fidelity.
    pub span: Range<usize>,
}

impl SemanticNode {
    pub fn new(kind: NodeKind, span: Range<usize>) -> Self {
        Self {
            kind,
            annotations: Vec::new(),
            children: Vec::new(),
            span,
        }
    }

    /// Collect all text content from this node and its children.
    pub fn text_content(&self) -> String {
        let mut buf = String::new();
        self.collect_text(&mut buf);
        buf
    }

    fn collect_text(&self, buf: &mut String) {
        if let NodeKind::Text(ref s) = self.kind {
            buf.push_str(s);
        }
        for child in &self.children {
            child.collect_text(buf);
        }
    }
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    /// Document root.
    Document,
    /// Section introduced by a heading.
    Section {
        level: u8,
        id: Option<String>,
    },
    /// A paragraph of inline content.
    Paragraph,
    /// A container directive block (`::: type` ... `:::`).
    DirectiveBlock {
        name: String,
    },
    /// A fenced code block.
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    /// Blockquote.
    BlockQuote,
    /// Ordered or unordered list.
    List {
        start: Option<u64>,
    },
    /// List item.
    Item,
    /// Table.
    Table,
    /// Table head row.
    TableHead,
    /// Table row.
    TableRow,
    /// Table cell.
    TableCell,
    /// Inline text run.
    Text(String),
    /// Inline code.
    InlineCode(String),
    /// Inline directive `:name[content]{attrs}`.
    InlineDirective {
        name: String,
    },
    /// Emphasis.
    Emphasis,
    /// Strong emphasis.
    Strong,
    /// Strikethrough.
    Strikethrough,
    /// Link.
    Link {
        dest: String,
        title: String,
    },
    /// Image.
    Image {
        dest: String,
        title: String,
    },
    /// Soft break.
    SoftBreak,
    /// Hard break.
    HardBreak,
    /// Horizontal rule.
    Rule,
    /// Raw HTML passthrough.
    Html(String),
}
