use anyhow::Result;
use rig::completion::Prompt;

use super::Question;
use crate::doc_bridge::DocumentView;

pub struct QuestionGenerator<M: rig::completion::CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: rig::completion::CompletionModel> QuestionGenerator<M> {
    pub fn new(model: M) -> Self {
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(
                "You suggest discussion questions based on document content.\n\
                 Given document blocks, return a JSON array of 1-3 questions.\n\
                 Each question has: text, source_block_id (which block prompted it), \
                 and question_type (\"clarification\", \"exploration\", or \"challenge\").\n\
                 Focus on what's interesting, unclear, or worth exploring further.\n\
                 If the document is too short, return [].\n\
                 Return ONLY the JSON array, no other text.",
            )
            .temperature(0.7)
            .build();

        Self { agent }
    }

    pub async fn generate(&self, doc: &DocumentView) -> Result<Vec<Question>> {
        let text = doc.text_for_analysis();
        if text.trim().is_empty() {
            return Ok(vec![]);
        }

        let response: String = self
            .agent
            .prompt(&format!("Generate questions:\n\n{text}"))
            .await?;

        let questions: Vec<Question> = serde_json::from_str(&response).unwrap_or_default();
        Ok(questions)
    }
}
