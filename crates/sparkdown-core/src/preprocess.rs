use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

/// A directive extracted during pre-processing.
#[derive(Debug, Clone)]
pub struct ExtractedDirective {
    pub name: String,
    pub content: Option<String>,
    pub attrs_raw: Option<String>,
    pub is_block: bool,
}

/// Result of pre-processing.
pub struct PreprocessResult {
    /// Modified markdown source for pulldown-cmark.
    pub modified_source: String,
    /// Directives keyed by placeholder ID.
    pub directives: HashMap<String, ExtractedDirective>,
    /// Prefix declarations found as link-reference-definitions `[prefix]: <iri>`.
    pub link_ref_prefixes: Vec<(String, String)>,
}

static INLINE_DIRECTIVE: LazyLock<Regex> = LazyLock::new(|| {
    // :name[content]{attrs} — name must start with a letter
    Regex::new(r":([a-zA-Z][\w-]*)\[([^\]]*)\](?:\{([^}]*)\})?").unwrap()
});

static BLOCK_DIRECTIVE_START: LazyLock<Regex> = LazyLock::new(|| {
    // ::: name {attrs}  or  ::: name
    Regex::new(r"^:::\s+(\S+)(?:\s+\{([^}]*)\})?\s*$").unwrap()
});

static LINK_REF_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    // [prefix]: <http://.../>
    Regex::new(r"^\[([a-zA-Z][\w-]*)\]:\s*<([^>]+)>\s*$").unwrap()
});

/// Pre-process sparkdown source into pulldown-cmark-compatible markdown.
pub fn preprocess(source: &str) -> PreprocessResult {
    let mut directives = HashMap::new();
    let mut link_ref_prefixes = Vec::new();
    let mut counter: u32 = 0;
    let mut output_lines: Vec<String> = Vec::new();

    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check for link-reference-definition prefix
        if let Some(caps) = LINK_REF_PREFIX.captures(line) {
            let prefix = caps[1].to_string();
            let iri = caps[2].to_string();
            link_ref_prefixes.push((prefix, iri));
            // Keep the line so pulldown-cmark sees it as a link ref def (won't render)
            output_lines.push(line.to_string());
            i += 1;
            continue;
        }

        // Check for block directive start
        if let Some(caps) = BLOCK_DIRECTIVE_START.captures(line) {
            let name = caps[1].to_string();
            let attrs_raw = caps.get(2).map(|m| m.as_str().to_string());
            counter += 1;
            let id = format!("SD{counter:04}");

            // Collect lines until closing :::
            let mut block_content = Vec::new();
            i += 1;
            while i < lines.len() {
                if lines[i].trim() == ":::" {
                    i += 1;
                    break;
                }
                block_content.push(lines[i].to_string());
                i += 1;
            }

            directives.insert(
                id.clone(),
                ExtractedDirective {
                    name: name.clone(),
                    content: Some(block_content.join("\n")),
                    attrs_raw,
                    is_block: true,
                },
            );

            // Emit HTML comment markers around the content
            output_lines.push(format!("<!-- sparkdown:block:start id={id} name={name} -->"));
            output_lines.push(String::new()); // blank line for paragraph separation
            for bl in &block_content {
                output_lines.push(bl.clone());
            }
            output_lines.push(String::new());
            output_lines.push(format!("<!-- sparkdown:block:end id={id} -->"));
            continue;
        }

        // Check for inline directives in the line
        if INLINE_DIRECTIVE.is_match(line) {
            let mut new_line = line.to_string();
            // Process all inline directives in this line
            // We need to re-match after each replacement since offsets change
            loop {
                let Some(caps) = INLINE_DIRECTIVE.captures(&new_line) else {
                    break;
                };
                let full_match = caps.get(0).unwrap();
                let name = caps[1].to_string();
                let content = caps[2].to_string();
                let attrs_raw = caps.get(3).map(|m| m.as_str().to_string());
                counter += 1;
                let id = format!("SD{counter:04}");

                directives.insert(
                    id.clone(),
                    ExtractedDirective {
                        name: name.clone(),
                        content: Some(content.clone()),
                        attrs_raw,
                        is_block: false,
                    },
                );

                let replacement =
                    format!("<span data-sd=\"{id}\">{content}</span>");
                new_line = format!(
                    "{}{}{}",
                    &new_line[..full_match.start()],
                    replacement,
                    &new_line[full_match.end()..]
                );
            }
            output_lines.push(new_line);
        } else {
            output_lines.push(line.to_string());
        }

        i += 1;
    }

    PreprocessResult {
        modified_source: output_lines.join("\n"),
        directives,
        link_ref_prefixes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_directive() {
        let source = "Hello :entity[Albert Einstein]{type=schema:Person wikidata=Q937} world.";
        let result = preprocess(source);
        assert!(result.modified_source.contains("data-sd="));
        assert!(result.modified_source.contains("Albert Einstein"));
        assert_eq!(result.directives.len(), 1);
        let d = result.directives.values().next().unwrap();
        assert_eq!(d.name, "entity");
        assert_eq!(d.content.as_deref(), Some("Albert Einstein"));
        assert_eq!(d.attrs_raw.as_deref(), Some("type=schema:Person wikidata=Q937"));
    }

    #[test]
    fn block_directive() {
        let source = "::: schema:Review\nThis was great.\n:::";
        let result = preprocess(source);
        assert!(result.modified_source.contains("sparkdown:block:start"));
        assert!(result.modified_source.contains("sparkdown:block:end"));
        assert_eq!(result.directives.len(), 1);
        let d = result.directives.values().next().unwrap();
        assert_eq!(d.name, "schema:Review");
        assert!(d.is_block);
    }

    #[test]
    fn link_ref_prefix() {
        let source = "[schema]: <http://schema.org/>\n[dc]: <http://purl.org/dc/terms/>";
        let result = preprocess(source);
        assert_eq!(result.link_ref_prefixes.len(), 2);
        assert_eq!(result.link_ref_prefixes[0], ("schema".to_string(), "http://schema.org/".to_string()));
    }

    #[test]
    fn multiple_inline_directives_one_line() {
        let source = ":entity[Alice]{type=schema:Person} met :entity[Bob]{type=schema:Person}.";
        let result = preprocess(source);
        assert_eq!(result.directives.len(), 2);
    }

    #[test]
    fn no_directives() {
        let source = "# Just a heading\n\nPlain paragraph.";
        let result = preprocess(source);
        assert!(result.directives.is_empty());
        assert!(result.link_ref_prefixes.is_empty());
        assert_eq!(result.modified_source, source);
    }
}
