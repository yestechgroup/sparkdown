use std::io::Write;

use sparkdown_core::annotation::AnnotationKind;
use sparkdown_core::ast::{NodeKind, SemanticNode, SparkdownDocument};

use crate::traits::{OutputRenderer, RenderError};

pub struct HtmlRdfaRenderer;

impl HtmlRdfaRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HtmlRdfaRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputRenderer for HtmlRdfaRenderer {
    fn render(
        &self,
        doc: &SparkdownDocument,
        out: &mut dyn Write,
    ) -> Result<(), RenderError> {
        // Build prefix string for html tag
        let prefix_str: String = doc
            .prefixes
            .iter()
            .map(|(p, iri)| format!("{p}: {iri}"))
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(out, "<!DOCTYPE html>")?;
        writeln!(out, "<html prefix=\"{prefix_str}\">")?;
        writeln!(out, "<head>")?;
        writeln!(out, "<meta charset=\"utf-8\">")?;
        if let Some(ref title) = doc.frontmatter.title {
            writeln!(out, "<title>{}</title>", escape_html(title))?;
        }
        writeln!(out, "</head>")?;

        // Body with optional document-level type
        if let Some(ref doc_type) = doc.frontmatter.doc_type {
            let resolved = doc.prefixes.try_resolve(doc_type);
            let type_attr = resolved
                .as_ref()
                .map(|n| n.as_str().to_string())
                .unwrap_or_else(|| doc_type.clone());
            writeln!(out, "<body typeof=\"{type_attr}\">")?;
        } else {
            writeln!(out, "<body>")?;
        }

        for node in &doc.nodes {
            render_node(node, out, 0)?;
        }

        writeln!(out, "</body>")?;
        writeln!(out, "</html>")?;

        Ok(())
    }

    fn content_type(&self) -> &str {
        "text/html"
    }

    fn file_extension(&self) -> &str {
        "html"
    }
}

fn render_node(node: &SemanticNode, out: &mut dyn Write, depth: usize) -> Result<(), RenderError> {
    let indent = "  ".repeat(depth);

    match &node.kind {
        NodeKind::Section { level, id } => {
            let tag = format!("h{level}");
            let mut attrs = Vec::new();

            if let Some(id) = id {
                attrs.push(format!("id=\"{id}\""));
            }

            // Collect semantic attributes from annotations
            let (type_attrs, prop_attrs, css_classes) = collect_rdfa_attrs(&node.annotations);

            if !type_attrs.is_empty() {
                attrs.push(format!("typeof=\"{}\"", type_attrs.join(" ")));
            }
            if !css_classes.is_empty() {
                attrs.push(format!("class=\"{}\"", css_classes.join(" ")));
            }

            // Wrap in a section element if there are semantic annotations
            if !type_attrs.is_empty() || !prop_attrs.is_empty() {
                let section_attrs = attrs.join(" ");
                let section_prefix = if section_attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {section_attrs}")
                };
                writeln!(out, "{indent}<section{section_prefix}>")?;
                write!(out, "{indent}  <{tag}>")?;
            } else {
                let h_attrs = attrs.join(" ");
                let h_prefix = if h_attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {h_attrs}")
                };
                write!(out, "{indent}<{tag}{h_prefix}>")?;
            }

            // Render heading text children
            for child in &node.children {
                render_inline(child, out)?;
            }

            writeln!(out, "</{tag}>")?;

            // Render property meta tags
            for (key, value) in &prop_attrs {
                writeln!(
                    out,
                    "{indent}  <meta property=\"{key}\" content=\"{}\" />",
                    escape_html(value)
                )?;
            }

            if !type_attrs.is_empty() || !prop_attrs.is_empty() {
                writeln!(out, "{indent}</section>")?;
            }
        }

        NodeKind::Paragraph => {
            write!(out, "{indent}<p>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            writeln!(out, "</p>")?;
        }

        NodeKind::DirectiveBlock { name } => {
            let mut attrs = Vec::new();
            let (type_attrs, prop_attrs, css_classes) = collect_rdfa_attrs(&node.annotations);

            if !type_attrs.is_empty() {
                attrs.push(format!("typeof=\"{}\"", type_attrs.join(" ")));
            }
            if !css_classes.is_empty() {
                attrs.push(format!("class=\"{}\"", css_classes.join(" ")));
            }
            attrs.push(format!("data-directive=\"{name}\""));

            let attr_str = attrs.join(" ");
            writeln!(out, "{indent}<div {attr_str}>")?;

            for child in &node.children {
                render_node(child, out, depth + 1)?;
            }

            for (key, value) in &prop_attrs {
                writeln!(
                    out,
                    "{indent}  <meta property=\"{key}\" content=\"{}\" />",
                    escape_html(value)
                )?;
            }

            writeln!(out, "{indent}</div>")?;
        }

        NodeKind::InlineDirective { name: _ } => {
            let (type_attrs, _prop_attrs, _css_classes) = collect_rdfa_attrs(&node.annotations);

            let mut attrs = Vec::new();
            if !type_attrs.is_empty() {
                attrs.push(format!("typeof=\"{}\"", type_attrs.join(" ")));
            }

            // Check for external IDs (sameAs)
            for ann in &node.annotations {
                if let AnnotationKind::ExternalId {
                    system,
                    identifier,
                } = &ann.kind
                {
                    let about = match system.as_str() {
                        "wikidata" => format!("http://www.wikidata.org/entity/{identifier}"),
                        "doi" => format!("https://doi.org/{identifier}"),
                        "orcid" => format!("https://orcid.org/{identifier}"),
                        _ => identifier.clone(),
                    };
                    attrs.push(format!("about=\"{about}\""));
                }
            }

            let attr_str = if attrs.is_empty() {
                String::new()
            } else {
                format!(" {}", attrs.join(" "))
            };

            write!(out, "<span{attr_str}>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            write!(out, "</span>")?;
        }

        NodeKind::CodeBlock { language, content } => {
            if let Some(lang) = language {
                writeln!(
                    out,
                    "{indent}<pre><code class=\"language-{lang}\">{}</code></pre>",
                    escape_html(content)
                )?;
            } else {
                writeln!(
                    out,
                    "{indent}<pre><code>{}</code></pre>",
                    escape_html(content)
                )?;
            }
        }

        NodeKind::BlockQuote => {
            writeln!(out, "{indent}<blockquote>")?;
            for child in &node.children {
                render_node(child, out, depth + 1)?;
            }
            writeln!(out, "{indent}</blockquote>")?;
        }

        NodeKind::List { start } => {
            if let Some(start_num) = start {
                writeln!(out, "{indent}<ol start=\"{start_num}\">")?;
                for child in &node.children {
                    render_node(child, out, depth + 1)?;
                }
                writeln!(out, "{indent}</ol>")?;
            } else {
                writeln!(out, "{indent}<ul>")?;
                for child in &node.children {
                    render_node(child, out, depth + 1)?;
                }
                writeln!(out, "{indent}</ul>")?;
            }
        }

        NodeKind::Item => {
            write!(out, "{indent}<li>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            writeln!(out, "</li>")?;
        }

        NodeKind::Rule => {
            writeln!(out, "{indent}<hr />")?;
        }

        NodeKind::Html(html) => {
            writeln!(out, "{indent}{html}")?;
        }

        NodeKind::Table => {
            writeln!(out, "{indent}<table>")?;
            for child in &node.children {
                render_node(child, out, depth + 1)?;
            }
            writeln!(out, "{indent}</table>")?;
        }

        NodeKind::TableHead => {
            writeln!(out, "{indent}<thead>")?;
            write!(out, "{indent}  <tr>")?;
            for child in &node.children {
                write!(out, "<th>")?;
                for gc in &child.children {
                    render_inline(gc, out)?;
                }
                write!(out, "</th>")?;
            }
            writeln!(out, "</tr>")?;
            writeln!(out, "{indent}</thead>")?;
        }

        NodeKind::TableRow => {
            write!(out, "{indent}<tr>")?;
            for child in &node.children {
                write!(out, "<td>")?;
                for gc in &child.children {
                    render_inline(gc, out)?;
                }
                write!(out, "</td>")?;
            }
            writeln!(out, "</tr>")?;
        }

        _ => {
            // Fallback: render children
            for child in &node.children {
                render_node(child, out, depth)?;
            }
        }
    }

    Ok(())
}

fn render_inline(node: &SemanticNode, out: &mut dyn Write) -> Result<(), RenderError> {
    match &node.kind {
        NodeKind::Text(text) => write!(out, "{}", escape_html(text))?,
        NodeKind::InlineCode(code) => write!(out, "<code>{}</code>", escape_html(code))?,
        NodeKind::Emphasis => {
            write!(out, "<em>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            write!(out, "</em>")?;
        }
        NodeKind::Strong => {
            write!(out, "<strong>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            write!(out, "</strong>")?;
        }
        NodeKind::Strikethrough => {
            write!(out, "<del>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            write!(out, "</del>")?;
        }
        NodeKind::Link { dest, title } => {
            let title_attr = if title.is_empty() {
                String::new()
            } else {
                format!(" title=\"{}\"", escape_html(title))
            };
            write!(out, "<a href=\"{dest}\"{title_attr}>")?;
            for child in &node.children {
                render_inline(child, out)?;
            }
            write!(out, "</a>")?;
        }
        NodeKind::Image { dest, title } => {
            let alt = node.text_content();
            let title_attr = if title.is_empty() {
                String::new()
            } else {
                format!(" title=\"{}\"", escape_html(title))
            };
            write!(
                out,
                "<img src=\"{dest}\" alt=\"{}\"{title_attr} />",
                escape_html(&alt)
            )?;
        }
        NodeKind::SoftBreak => writeln!(out)?,
        NodeKind::HardBreak => write!(out, "<br />")?,
        NodeKind::InlineDirective { .. } | NodeKind::DirectiveBlock { .. } => {
            // Delegate to block renderer
            render_node(node, out, 0)?;
        }
        _ => {
            for child in &node.children {
                render_inline(child, out)?;
            }
        }
    }
    Ok(())
}

/// Collect RDFa attributes from annotations.
/// Returns (type_iris, property_key_values, css_classes).
fn collect_rdfa_attrs(
    annotations: &[sparkdown_core::annotation::Annotation],
) -> (Vec<String>, Vec<(String, String)>, Vec<String>) {
    let mut types = Vec::new();
    let mut props = Vec::new();
    let mut classes = Vec::new();

    for ann in annotations {
        match &ann.kind {
            AnnotationKind::TypeAssignment { resolved_iri, raw } => {
                let type_str = resolved_iri
                    .as_ref()
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| raw.clone());
                types.push(type_str);
            }
            AnnotationKind::Property {
                resolved_iri,
                raw_key,
                value,
            } => {
                let key = resolved_iri
                    .as_ref()
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| raw_key.clone());
                props.push((key, value.clone()));
            }
            AnnotationKind::CssClass(c) => {
                classes.push(c.clone());
            }
            _ => {}
        }
    }

    (types, props, classes)
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
