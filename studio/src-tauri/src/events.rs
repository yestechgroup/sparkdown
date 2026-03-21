use tauri::{AppHandle, Emitter};

use crate::types::{DocId, EntityDto, SidecarStatus, StaleAnchor};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentOpenedPayload {
    pub doc_id: DocId,
    pub entities: Vec<EntityDto>,
    pub sidecar_status: SidecarStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EntitiesUpdatedPayload {
    pub doc_id: DocId,
    pub entities: Vec<EntityDto>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SidecarStatusPayload {
    pub doc_id: DocId,
    pub status: SidecarStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StaleAnchorsPayload {
    pub doc_id: DocId,
    pub anchors: Vec<StaleAnchor>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorPayload {
    pub doc_id: DocId,
    pub message: String,
}

pub fn emit_document_opened(app: &AppHandle, payload: DocumentOpenedPayload) {
    let _ = app.emit("document-opened", payload);
}

pub fn emit_entities_updated(app: &AppHandle, payload: EntitiesUpdatedPayload) {
    let _ = app.emit("entities-updated", payload);
}

pub fn emit_sidecar_status(app: &AppHandle, payload: SidecarStatusPayload) {
    let _ = app.emit("sidecar-status", payload);
}

pub fn emit_stale_anchors(app: &AppHandle, payload: StaleAnchorsPayload) {
    let _ = app.emit("stale-anchors", payload);
}

pub fn emit_parse_error(app: &AppHandle, payload: ErrorPayload) {
    let _ = app.emit("parse-error", payload);
}

pub fn emit_sidecar_error(app: &AppHandle, payload: ErrorPayload) {
    let _ = app.emit("sidecar-error", payload);
}
