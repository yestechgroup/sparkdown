use oxrdf::NamedNode;

use crate::prefix::PrefixMap;

/// A semantic annotation attached to an AST node.
#[derive(Debug, Clone)]
pub struct Annotation {
    pub kind: AnnotationKind,
}

#[derive(Debug, Clone)]
pub enum AnnotationKind {
    /// A semantic type assignment (from `.schema:Event` or `type=schema:Person`).
    TypeAssignment {
        resolved_iri: Option<NamedNode>,
        raw: String,
    },
    /// A property-value pair (from `startDate=2026-03-20`).
    Property {
        resolved_iri: Option<NamedNode>,
        raw_key: String,
        value: String,
    },
    /// An external identifier (from `wikidata=Q937`).
    ExternalId {
        system: String,
        identifier: String,
    },
    /// A CSS class (no colon, not semantic).
    CssClass(String),
    /// An explicit ID (`#my-id`).
    Id(String),
}

/// Well-known external ID systems that get special treatment.
const EXTERNAL_ID_SYSTEMS: &[&str] = &["wikidata", "doi", "orcid", "isbn", "issn"];

/// Parse an attribute string like `type=schema:Person wikidata=Q937 .schema:Event #my-id`
/// into a list of annotations.
pub fn parse_attr_string(raw: &str, prefixes: &PrefixMap) -> Vec<Annotation> {
    let mut annotations = Vec::new();

    for token in tokenize_attrs(raw) {
        match token {
            AttrToken::Id(id) => {
                annotations.push(Annotation {
                    kind: AnnotationKind::Id(id),
                });
            }
            AttrToken::Class(class) => {
                if class.contains(':') {
                    // Class with colon = semantic type
                    let resolved = prefixes.try_resolve(&class);
                    annotations.push(Annotation {
                        kind: AnnotationKind::TypeAssignment {
                            resolved_iri: resolved,
                            raw: class,
                        },
                    });
                } else {
                    annotations.push(Annotation {
                        kind: AnnotationKind::CssClass(class),
                    });
                }
            }
            AttrToken::KeyValue(key, value) => {
                if key == "type" || key == "@type" {
                    let resolved = prefixes.try_resolve(&value);
                    annotations.push(Annotation {
                        kind: AnnotationKind::TypeAssignment {
                            resolved_iri: resolved,
                            raw: value,
                        },
                    });
                } else if EXTERNAL_ID_SYSTEMS.contains(&key.as_str()) {
                    annotations.push(Annotation {
                        kind: AnnotationKind::ExternalId {
                            system: key,
                            identifier: value,
                        },
                    });
                } else {
                    let resolved = prefixes.try_resolve(&key);
                    annotations.push(Annotation {
                        kind: AnnotationKind::Property {
                            resolved_iri: resolved,
                            raw_key: key,
                            value,
                        },
                    });
                }
            }
        }
    }

    annotations
}

#[derive(Debug)]
enum AttrToken {
    Id(String),
    Class(String),
    KeyValue(String, String),
}

/// Tokenize an attribute string, handling quoted values.
fn tokenize_attrs(raw: &str) -> Vec<AttrToken> {
    let mut tokens = Vec::new();
    let mut chars = raw.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '#' {
            chars.next();
            let id = collect_word(&mut chars);
            if !id.is_empty() {
                tokens.push(AttrToken::Id(id));
            }
        } else if ch == '.' {
            chars.next();
            let class = collect_word(&mut chars);
            if !class.is_empty() {
                tokens.push(AttrToken::Class(class));
            }
        } else {
            let word = collect_word(&mut chars);
            if word.is_empty() {
                chars.next();
                continue;
            }
            if chars.peek() == Some(&'=') {
                chars.next(); // consume '='
                let value = if chars.peek() == Some(&'"') {
                    collect_quoted(&mut chars)
                } else {
                    collect_word(&mut chars)
                };
                tokens.push(AttrToken::KeyValue(word, value));
            } else {
                // Bare word — treat as class
                tokens.push(AttrToken::Class(word));
            }
        }
    }

    tokens
}

fn collect_word(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut word = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == '=' {
            break;
        }
        word.push(ch);
        chars.next();
    }
    word
}

fn collect_quoted(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut value = String::new();
    chars.next(); // consume opening quote
    while let Some(&ch) = chars.peek() {
        chars.next();
        if ch == '"' {
            break;
        }
        if ch == '\\' {
            if let Some(&escaped) = chars.peek() {
                value.push(escaped);
                chars.next();
            }
        } else {
            value.push(ch);
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_type_and_property() {
        let mut pm = PrefixMap::new();
        pm.seed_builtins();
        let anns = parse_attr_string("type=schema:Person wikidata=Q937", &pm);
        assert_eq!(anns.len(), 2);
        match &anns[0].kind {
            AnnotationKind::TypeAssignment { raw, resolved_iri } => {
                assert_eq!(raw, "schema:Person");
                assert_eq!(
                    resolved_iri.as_ref().unwrap().as_str(),
                    "http://schema.org/Person"
                );
            }
            other => panic!("expected TypeAssignment, got {other:?}"),
        }
        match &anns[1].kind {
            AnnotationKind::ExternalId {
                system,
                identifier,
            } => {
                assert_eq!(system, "wikidata");
                assert_eq!(identifier, "Q937");
            }
            other => panic!("expected ExternalId, got {other:?}"),
        }
    }

    #[test]
    fn parse_class_with_colon() {
        let mut pm = PrefixMap::new();
        pm.seed_builtins();
        let anns = parse_attr_string(".schema:Event startDate=2026-03-20", &pm);
        assert_eq!(anns.len(), 2);
        match &anns[0].kind {
            AnnotationKind::TypeAssignment { raw, .. } => {
                assert_eq!(raw, "schema:Event");
            }
            other => panic!("expected TypeAssignment, got {other:?}"),
        }
        match &anns[1].kind {
            AnnotationKind::Property {
                raw_key, value, ..
            } => {
                assert_eq!(raw_key, "startDate");
                assert_eq!(value, "2026-03-20");
            }
            other => panic!("expected Property, got {other:?}"),
        }
    }

    #[test]
    fn parse_id_and_css_class() {
        let pm = PrefixMap::new();
        let anns = parse_attr_string("#my-id .highlight", &pm);
        assert_eq!(anns.len(), 2);
        assert!(matches!(&anns[0].kind, AnnotationKind::Id(id) if id == "my-id"));
        assert!(matches!(&anns[1].kind, AnnotationKind::CssClass(c) if c == "highlight"));
    }

    #[test]
    fn parse_quoted_value() {
        let pm = PrefixMap::new();
        let anns = parse_attr_string("speaker=\"Dr. Frankenstein\"", &pm);
        assert_eq!(anns.len(), 1);
        match &anns[0].kind {
            AnnotationKind::Property { raw_key, value, .. } => {
                assert_eq!(raw_key, "speaker");
                assert_eq!(value, "Dr. Frankenstein");
            }
            other => panic!("expected Property, got {other:?}"),
        }
    }
}
