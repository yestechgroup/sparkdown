use serde::Deserialize;
use std::collections::BTreeMap;

/// Parsed sparkdown frontmatter.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SparkdownFrontmatter {
    /// JSON-LD @context URL or inline context.
    #[serde(rename = "@context", default)]
    pub context: Option<serde_json::Value>,

    /// Prefix declarations.
    #[serde(default)]
    pub prefixes: BTreeMap<String, String>,

    /// Document-level semantic type (e.g., "schema:Article").
    #[serde(rename = "@type", default)]
    pub doc_type: Option<String>,

    /// Template name for rendering.
    #[serde(default)]
    pub template: Option<String>,

    /// Title.
    #[serde(default)]
    pub title: Option<String>,

    /// Arbitrary extra fields.
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// Extract and parse frontmatter from source text.
/// Returns (frontmatter, content_after_frontmatter).
pub fn parse_frontmatter(source: &str) -> (SparkdownFrontmatter, String) {
    let matter = gray_matter::Matter::<gray_matter::engine::YAML>::new();
    let result = matter.parse(source);

    let frontmatter = result
        .data
        .and_then(|d| {
            // gray_matter returns a Pod; convert to serde_json::Value then deserialize
            let json = pod_to_json(&d);
            serde_json::from_value::<SparkdownFrontmatter>(json).ok()
        })
        .unwrap_or_default();

    (frontmatter, result.content)
}

/// Convert gray_matter Pod to serde_json::Value.
fn pod_to_json(pod: &gray_matter::Pod) -> serde_json::Value {
    match pod {
        gray_matter::Pod::Null => serde_json::Value::Null,
        gray_matter::Pod::Boolean(b) => serde_json::Value::Bool(*b),
        gray_matter::Pod::Integer(i) => serde_json::Value::Number((*i).into()),
        gray_matter::Pod::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        gray_matter::Pod::String(s) => serde_json::Value::String(s.clone()),
        gray_matter::Pod::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(pod_to_json).collect())
        }
        gray_matter::Pod::Hash(map) => {
            let obj = map
                .iter()
                .map(|(k, v)| (k.clone(), pod_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_frontmatter() {
        let source = r#"---
title: Test Document
prefixes:
  schema: http://schema.org/
"@type": schema:Article
---

# Hello World
"#;
        let (fm, content) = parse_frontmatter(source);
        assert_eq!(fm.title.as_deref(), Some("Test Document"));
        assert_eq!(fm.doc_type.as_deref(), Some("schema:Article"));
        assert_eq!(
            fm.prefixes.get("schema").map(|s| s.as_str()),
            Some("http://schema.org/")
        );
        assert!(content.contains("# Hello World"));
    }

    #[test]
    fn parse_empty_frontmatter() {
        let source = "# No frontmatter\n\nJust content.";
        let (fm, content) = parse_frontmatter(source);
        assert!(fm.title.is_none());
        assert!(content.contains("# No frontmatter"));
    }
}
