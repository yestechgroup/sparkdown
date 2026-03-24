use axum::{extract::State, Json};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agents::entity_detector::EntityDetector;
use crate::agents::question_generator::QuestionGenerator;
use crate::agents::summarizer::Summarizer;
use crate::doc_bridge::{self, DocumentView};
use crate::{config, doc_writer};

pub struct AppState<M: rig::completion::CompletionModel> {
    pub entity_detector: EntityDetector<M>,
    pub summarizer: Summarizer<M>,
    pub question_gen: QuestionGenerator<M>,
    pub doc: Arc<Mutex<yrs::Doc>>,
    pub config: config::Config,
    /// Track the last text hash to avoid re-running on agent-only changes
    pub last_text_hash: Mutex<u64>,
}

/// Called by y-websocket HTTP callback when the document changes.
pub async fn handle_doc_update<
    M: rig::completion::CompletionModel + Send + Sync + 'static,
>(
    State(state): State<Arc<AppState<M>>>,
) -> Json<serde_json::Value> {
    tracing::info!("Document update received, running agents...");

    // 1. Read current document state
    let doc_view: DocumentView = {
        let doc = state.doc.lock().await;
        doc_bridge::read_document(&doc)
    };

    // Skip if document has no meaningful content
    let analysis_text = doc_view.text_for_analysis();
    if analysis_text.trim().is_empty() {
        tracing::debug!("Document is empty, skipping agent run");
        return Json(serde_json::json!({ "status": "skipped", "reason": "empty" }));
    }

    // Check if text content changed (avoid infinite loops from agent notes)
    let text_hash = hash_text(&analysis_text);
    {
        let mut last = state.last_text_hash.lock().await;
        if *last == text_hash {
            tracing::debug!("Text unchanged, skipping agent run");
            return Json(
                serde_json::json!({ "status": "skipped", "reason": "unchanged" }),
            );
        }
        *last = text_hash;
    }

    // 2. Run all three agents in parallel
    let (entities, summary, questions) = tokio::join!(
        state.entity_detector.analyze(&doc_view),
        state.summarizer.summarize(&doc_view),
        state.question_gen.generate(&doc_view),
    );

    // 3. Clear old agent notes
    {
        let doc = state.doc.lock().await;
        match doc_writer::clear_agent_notes(&doc) {
            Ok(n) => tracing::debug!("Cleared {n} old agent notes"),
            Err(e) => tracing::warn!("Failed to clear agent notes: {e}"),
        }
    }

    // 4. Write new agent notes to the document
    let mut notes_written = 0;
    {
        let doc = state.doc.lock().await;

        if let Ok(entities) = entities {
            for entity in entities {
                if entity.confidence >= state.config.confidence_threshold {
                    if doc_writer::insert_agent_note(
                        &doc,
                        Some(&entity.block_id),
                        "entity-detector",
                        "Entity Detector",
                        "entity",
                        &format!(
                            "{} \u{2014} {} ({})",
                            entity.entity_type, entity.text_span, entity.reasoning
                        ),
                        entity.confidence,
                    )
                    .is_ok()
                    {
                        notes_written += 1;
                    }
                }
            }
        }

        if let Ok(Some(summary)) = summary {
            if doc_writer::insert_agent_note(
                &doc,
                doc_view.last_content_block_id(),
                "summarizer",
                "Summarizer",
                "summary",
                &summary.text,
                1.0,
            )
            .is_ok()
            {
                notes_written += 1;
            }
        }

        if let Ok(questions) = questions {
            for question in questions {
                if doc_writer::insert_agent_note(
                    &doc,
                    Some(&question.source_block_id),
                    "question-gen",
                    "Question Generator",
                    "question",
                    &question.text,
                    0.8,
                )
                .is_ok()
                {
                    notes_written += 1;
                }
            }
        }
    }

    tracing::info!("Wrote {notes_written} agent notes");
    Json(serde_json::json!({ "status": "ok", "notes_written": notes_written }))
}

/// Manual trigger endpoint for testing
pub async fn run_agents_manually<
    M: rig::completion::CompletionModel + Send + Sync + 'static,
>(
    State(state): State<Arc<AppState<M>>>,
) -> Json<serde_json::Value> {
    handle_doc_update(State(state)).await
}

fn hash_text(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
