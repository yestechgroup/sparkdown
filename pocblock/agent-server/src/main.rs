mod agents;
mod config;
mod doc_bridge;
mod doc_writer;
mod routes;
mod yjs_client;

use std::sync::Arc;
use tokio::sync::Mutex;

use axum::routing::{get, post};
use axum::Router;
use rig::client::CompletionClient;
use rig::providers::anthropic;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("sparkdown_agent_poc=debug,info")
        .init();

    let config = config::Config::from_env()?;
    tracing::info!("Config: model={}", config.model);

    // Initialize Anthropic LLM provider using ANTHROPIC_API_KEY env var
    let client = anthropic::Client::new(&config.anthropic_api_key)?;
    let model = client.completion_model(&config.model);

    // Create shared Yrs doc
    let doc = Arc::new(Mutex::new(yrs::Doc::new()));

    // Spawn Yjs sync client
    let sync_url = config.sync_url.clone();
    let sync_doc = doc.clone();
    tokio::spawn(async move {
        loop {
            match yjs_client::connect_and_sync(&sync_url, sync_doc.clone()).await {
                Ok(()) => break,
                Err(e) => {
                    tracing::error!("Yjs sync error: {e}, reconnecting in 3s...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                }
            }
        }
    });

    // Create agents and app state
    let state = Arc::new(routes::AppState {
        entity_detector: agents::entity_detector::EntityDetector::new(model.clone()),
        summarizer: agents::summarizer::Summarizer::new(model.clone()),
        question_gen: agents::question_generator::QuestionGenerator::new(model),
        doc,
        config: config.clone(),
        last_text_hash: Mutex::new(0),
    });

    // CORS layer — allow browser requests from the frontend dev server
    let cors = tower_http::cors::CorsLayer::permissive();

    // Routes
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/on-doc-update", post(routes::handle_doc_update))
        .route("/run-agents", post(routes::run_agents_manually))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.agent_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Agent server listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
