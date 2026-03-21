use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};

use crate::types::DocId;

/// Commands that can be sent to a DocumentSession actor.
/// Each variant carries a oneshot reply channel where applicable.
pub enum SessionCommand {
    UpdateSource {
        new_source: String,
        reply: tokio::sync::oneshot::Sender<()>,
    },
    GetEntitiesAt {
        start: usize,
        end: usize,
        reply: tokio::sync::oneshot::Sender<Vec<crate::types::EntityDto>>,
    },
    ExportAs {
        format: crate::types::RenderFormat,
        reply: tokio::sync::oneshot::Sender<Result<String, String>>,
    },
    Save {
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    Close,
    // Phase 1.5: Entity creation and management
    CreateEntity {
        span_start: usize,
        span_end: usize,
        type_iri: String,
        reply: tokio::sync::oneshot::Sender<Result<crate::types::EntityDto, String>>,
    },
    UpdateStaleAnchor {
        entity_id: String,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    GetDocumentOverview {
        reply: tokio::sync::oneshot::Sender<crate::types::DocumentOverviewDto>,
    },
    GetEntityDetail {
        entity_id: String,
        reply: tokio::sync::oneshot::Sender<Result<crate::types::EntityDetailDto, String>>,
    },
    DeleteEntity {
        entity_id: String,
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
}

/// Routes commands to the correct DocumentSession by DocId.
pub struct SessionRegistry {
    sessions: RwLock<HashMap<DocId, mpsc::Sender<SessionCommand>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register(&self, doc_id: DocId, sender: mpsc::Sender<SessionCommand>) {
        self.sessions.write().await.insert(doc_id, sender);
    }

    pub async fn unregister(&self, doc_id: &str) {
        self.sessions.write().await.remove(doc_id);
    }

    pub async fn get(&self, doc_id: &str) -> Option<mpsc::Sender<SessionCommand>> {
        self.sessions.read().await.get(doc_id).cloned()
    }

    pub async fn is_open(&self, doc_id: &str) -> bool {
        self.sessions.read().await.contains_key(doc_id)
    }

    pub async fn open_count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_and_get_session() {
        let registry = SessionRegistry::new();
        let (tx, _rx) = mpsc::channel(16);
        registry.register("/test/doc.md".into(), tx).await;

        assert!(registry.is_open("/test/doc.md").await);
        assert!(registry.get("/test/doc.md").await.is_some());
        assert_eq!(registry.open_count().await, 1);
    }

    #[tokio::test]
    async fn unregister_removes_session() {
        let registry = SessionRegistry::new();
        let (tx, _rx) = mpsc::channel(16);
        registry.register("/test/doc.md".into(), tx).await;
        registry.unregister("/test/doc.md").await;

        assert!(!registry.is_open("/test/doc.md").await);
        assert_eq!(registry.open_count().await, 0);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let registry = SessionRegistry::new();
        assert!(registry.get("/nonexistent.md").await.is_none());
    }
}
