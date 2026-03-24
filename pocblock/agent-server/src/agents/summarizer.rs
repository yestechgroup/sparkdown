use anyhow::Result;
use rig::completion::Prompt;

use super::Summary;
use crate::doc_bridge::DocumentView;

pub struct Summarizer<M: rig::completion::CompletionModel> {
    agent: rig::agent::Agent<M>,
}

impl<M: rig::completion::CompletionModel> Summarizer<M> {
    pub fn new(model: M) -> Self {
        let agent = rig::agent::AgentBuilder::new(model)
            .preamble(
                "You summarize documents concisely.\n\
                 Given document blocks, return a JSON object with:\n\
                 - text: one-paragraph summary\n\
                 - topics: array of key topic strings\n\
                 If the document is too short to summarize (< 2 sentences), \
                 return {\"text\": \"\", \"topics\": []}.\n\
                 Return ONLY the JSON object, no other text.",
            )
            .temperature(0.5)
            .build();

        Self { agent }
    }

    pub async fn summarize(&self, doc: &DocumentView) -> Result<Option<Summary>> {
        let text = doc.text_for_analysis();
        if text.trim().is_empty() {
            return Ok(None);
        }

        let response: String = self
            .agent
            .prompt(&format!("Summarize:\n\n{text}"))
            .await?;

        tracing::debug!("Summarizer raw response: {:?}", &response[..response.len().min(300)]);
        let cleaned = super::strip_code_fences(&response);
        let summary: Summary = serde_json::from_str(cleaned)?;
        if summary.text.is_empty() {
            return Ok(None);
        }

        Ok(Some(summary))
    }
}
