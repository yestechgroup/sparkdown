//! Sparkdown vocabulary (`sd:`) constants.
//!
//! A small ontology for structural and rhetorical annotation of documents.

use oxrdf::NamedNode;

/// The sparkdown vocabulary namespace IRI.
pub const SD_NS: &str = "urn:sparkdown:vocab/";

/// Helper to build a `NamedNode` in the `sd:` namespace.
fn sd(local: &str) -> NamedNode {
    NamedNode::new(format!("{SD_NS}{local}")).expect("valid sd: IRI")
}

// --- Types ---

/// `sd:Section` — a structural section of the document.
pub fn section() -> NamedNode {
    sd("Section")
}

/// `sd:Paragraph` — a paragraph-level annotation target.
pub fn paragraph() -> NamedNode {
    sd("Paragraph")
}

/// `sd:Review` — rhetorical role: review/opinion.
pub fn review() -> NamedNode {
    sd("Review")
}

/// `sd:Abstract` — rhetorical role: summary/abstract.
pub fn r#abstract() -> NamedNode {
    sd("Abstract")
}

/// `sd:Argument` — rhetorical role: argumentative content.
pub fn argument() -> NamedNode {
    sd("Argument")
}

/// `sd:Summary` — rhetorical role: summarization.
pub fn summary() -> NamedNode {
    sd("Summary")
}

/// `sd:Comparison` — rhetorical role: comparative analysis.
pub fn comparison() -> NamedNode {
    sd("Comparison")
}

/// `sd:Example` — rhetorical role: illustrative example.
pub fn example() -> NamedNode {
    sd("Example")
}

// --- Properties ---

/// `sd:role` — links structure to rhetorical function.
pub fn role() -> NamedNode {
    sd("role")
}

/// `sd:snippet` — short content fingerprint for staleness verification.
pub fn snippet() -> NamedNode {
    sd("snippet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_resolve_to_correct_iris() {
        assert_eq!(section().as_str(), "urn:sparkdown:vocab/Section");
        assert_eq!(paragraph().as_str(), "urn:sparkdown:vocab/Paragraph");
        assert_eq!(role().as_str(), "urn:sparkdown:vocab/role");
        assert_eq!(snippet().as_str(), "urn:sparkdown:vocab/snippet");
        assert_eq!(review().as_str(), "urn:sparkdown:vocab/Review");
        assert_eq!(r#abstract().as_str(), "urn:sparkdown:vocab/Abstract");
        assert_eq!(argument().as_str(), "urn:sparkdown:vocab/Argument");
        assert_eq!(summary().as_str(), "urn:sparkdown:vocab/Summary");
        assert_eq!(comparison().as_str(), "urn:sparkdown:vocab/Comparison");
        assert_eq!(example().as_str(), "urn:sparkdown:vocab/Example");
    }
}
