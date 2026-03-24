pub mod entity_detector;
pub mod question_generator;
pub mod summarizer;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Strip markdown code fences (```json ... ```) that LLMs often wrap around JSON.
pub fn strip_code_fences(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        // Skip optional language tag on first line
        let rest = rest
            .find('\n')
            .map(|i| &rest[i + 1..])
            .unwrap_or(rest);
        rest.strip_suffix("```").unwrap_or(rest).trim()
    } else {
        trimmed
    }
}

/// An entity identified in the document
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntitySuggestion {
    /// The block ID where the entity appears
    pub block_id: String,
    /// The exact text span matched
    pub text_span: String,
    /// Schema.org type (e.g., "schema:Person")
    pub entity_type: String,
    /// 0.0 to 1.0
    pub confidence: f64,
    /// Why this was identified
    pub reasoning: String,
}

/// A document summary
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    /// One-paragraph summary
    pub text: String,
    /// Key topics covered
    pub topics: Vec<String>,
}

/// A suggested discussion question
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Question {
    /// The question text
    pub text: String,
    /// Which block prompted this question
    pub source_block_id: String,
    /// "clarification", "exploration", "challenge"
    pub question_type: String,
}
