use anyhow::Result;
use rig::completion::Prompt;

use super::EntitySuggestion;
use crate::doc_bridge::DocumentView;

pub struct EntityDetector<M: rig::completion::CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: rig::completion::CompletionModel> EntityDetector<M> {
    pub fn new(model: M) -> Self {
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(
                "You are a semantic entity detector for a knowledge authoring tool.\n\
                 Given document blocks (format: [block_id] text), identify named entities.\n\
                 For each entity return a JSON object with: block_id, text_span, entity_type \
                 (schema.org type like schema:Person, schema:Place, schema:Organization, schema:Event), \
                 confidence (0-1), and reasoning.\n\
                 Only return entities with confidence >= 0.6.\n\
                 Return ONLY a JSON array of objects. If no entities found, return [].\n\
                 Do not include any text outside the JSON array.",
            )
            .temperature(0.3)
            .build();

        Self { agent }
    }

    pub async fn analyze(&self, doc: &DocumentView) -> Result<Vec<EntitySuggestion>> {
        let text = doc.text_for_analysis();
        if text.trim().is_empty() {
            return Ok(vec![]);
        }

        let response: String = self
            .agent
            .prompt(&format!(
                "Analyze these document blocks for entities:\n\n{text}"
            ))
            .await?;

        tracing::debug!("Entity detector raw response: {:?}", &response[..response.len().min(300)]);
        let cleaned = super::strip_code_fences(&response);
        let suggestions: Vec<EntitySuggestion> =
            serde_json::from_str(cleaned).unwrap_or_default();
        Ok(suggestions)
    }
}
