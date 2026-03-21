use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::pack_types::{TypeCategoryDto, TypeOptionDto};
use crate::registry::{SessionCommand, SessionRegistry};
use crate::session::DocumentSession;
use crate::types::{DocId, DocumentOverviewDto, EntityDetailDto, EntityDto, FileEntry, RenderFormat, WorkspaceInfo};

#[tauri::command]
pub async fn open_workspace(app: AppHandle) -> Result<WorkspaceInfo, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });

    let folder = rx
        .await
        .map_err(|_| "Dialog cancelled".to_string())?
        .ok_or("No folder selected".to_string())?;
    let path = folder.to_string();

    let files = scan_workspace_files(&path).await?;

    Ok(WorkspaceInfo { path, files })
}

#[tauri::command]
pub async fn list_workspace_files(path: String) -> Result<Vec<FileEntry>, String> {
    scan_workspace_files(&path).await
}

#[tauri::command]
pub async fn open_document(
    app: AppHandle,
    registry: State<'_, Arc<SessionRegistry>>,
    path: String,
) -> Result<DocId, String> {
    let file_path = PathBuf::from(&path);

    // Check if already open
    let canonical = file_path
        .canonicalize()
        .map_err(|e| format!("Cannot resolve path: {e}"))?
        .to_string_lossy()
        .into_owned();

    if registry.is_open(&canonical).await {
        return Ok(canonical);
    }

    let (doc_id, tx) = DocumentSession::open(app, file_path).await?;
    registry.register(doc_id.clone(), tx).await;
    Ok(doc_id)
}

#[tauri::command]
pub async fn close_document(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
) -> Result<(), String> {
    if let Some(tx) = registry.get(&doc_id).await {
        let _ = tx.send(SessionCommand::Close).await;
    }
    registry.unregister(&doc_id).await;
    Ok(())
}

#[tauri::command]
pub async fn update_source(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
    new_source: String,
) -> Result<(), String> {
    let tx = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::UpdateSource {
        new_source,
        reply: reply_tx,
    })
    .await
    .map_err(|_| "Session closed".to_string())?;
    reply_rx.await.map_err(|_| "Session dropped".to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_entities_at(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
    start: usize,
    end: usize,
) -> Result<Vec<EntityDto>, String> {
    let tx = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::GetEntitiesAt {
        start,
        end,
        reply: reply_tx,
    })
    .await
    .map_err(|_| "Session closed".to_string())?;
    reply_rx.await.map_err(|_| "Session dropped".to_string())
}

#[tauri::command]
pub async fn export_document(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
    format: RenderFormat,
) -> Result<String, String> {
    let tx = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::ExportAs {
        format,
        reply: reply_tx,
    })
    .await
    .map_err(|_| "Session closed".to_string())?;
    reply_rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn save_document(
    registry: State<'_, Arc<SessionRegistry>>,
    doc_id: DocId,
) -> Result<(), String> {
    let tx = registry.get(&doc_id).await.ok_or("Document not open".to_string())?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(SessionCommand::Save { reply: reply_tx })
        .await
        .map_err(|_| "Session closed".to_string())?;
    reply_rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn create_entity(
    doc_id: String,
    char_start: usize,
    char_end: usize,
    type_iri: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<EntityDto, String> {
    if char_start >= char_end {
        return Err("Selection must not be empty".into());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open".to_string())?;
    sender
        .send(SessionCommand::CreateEntity {
            span_start: char_start,
            span_end: char_end,
            type_iri,
            reply: tx,
        })
        .await
        .map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn update_stale_anchor(
    doc_id: String,
    entity_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open".to_string())?;
    sender
        .send(SessionCommand::UpdateStaleAnchor {
            entity_id,
            reply: tx,
        })
        .await
        .map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn get_document_overview(
    doc_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<DocumentOverviewDto, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open".to_string())?;
    sender
        .send(SessionCommand::GetDocumentOverview { reply: tx })
        .await
        .map_err(|_| "Session closed".to_string())?;
    Ok(rx.await.map_err(|_| "Session dropped".to_string())?)
}

#[tauri::command]
pub async fn get_entity_detail(
    doc_id: String,
    entity_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<EntityDetailDto, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open".to_string())?;
    sender
        .send(SessionCommand::GetEntityDetail {
            entity_id,
            reply: tx,
        })
        .await
        .map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn delete_entity(
    doc_id: String,
    entity_id: String,
    registry: State<'_, Arc<SessionRegistry>>,
) -> Result<(), String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let sender = registry
        .get(&doc_id)
        .await
        .ok_or("Document not open".to_string())?;
    sender
        .send(SessionCommand::DeleteEntity {
            entity_id,
            reply: tx,
        })
        .await
        .map_err(|_| "Session closed".to_string())?;
    rx.await.map_err(|_| "Session dropped".to_string())?
}

#[tauri::command]
pub async fn list_available_types(
    theme_registry: State<'_, crate::ThemeRegistryState>,
) -> Result<Vec<TypeCategoryDto>, String> {
    let reg = theme_registry.read().await;
    let mut categories = vec![];
    for (prefix, base_iri, types) in reg.all_type_categories() {
        let type_options: Vec<TypeOptionDto> = types
            .into_iter()
            .map(|(curie, _local, tdef)| TypeOptionDto {
                iri: tdef.iri.as_str().to_string(),
                curie,
                label: tdef.label.clone(),
                description: tdef.comment.clone(),
            })
            .collect();
        categories.push(TypeCategoryDto {
            pack_name: prefix,
            category_label: base_iri,
            types: type_options,
        });
    }
    Ok(categories)
}

#[tauri::command]
pub async fn search_types(
    query: String,
    limit: Option<usize>,
    theme_registry: State<'_, crate::ThemeRegistryState>,
) -> Result<Vec<TypeOptionDto>, String> {
    let reg = theme_registry.read().await;
    let results = reg.search_types(&query, limit.unwrap_or(50));
    Ok(results
        .into_iter()
        .map(|(prefix, tdef)| {
            let local = tdef
                .iri
                .as_str()
                .rsplit('/')
                .next()
                .unwrap_or(tdef.iri.as_str());
            TypeOptionDto {
                iri: tdef.iri.as_str().to_string(),
                curie: format!("{prefix}:{local}"),
                label: tdef.label.clone(),
                description: tdef.comment.clone(),
            }
        })
        .collect())
}

/// Recursively scan a directory for .md files.
async fn scan_workspace_files(dir: &str) -> Result<Vec<FileEntry>, String> {
    let mut files = Vec::new();
    let mut stack = vec![PathBuf::from(dir)];

    while let Some(current) = stack.pop() {
        let mut entries = tokio::fs::read_dir(&current)
            .await
            .map_err(|e| format!("Cannot read directory: {e}"))?;

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                // Skip hidden directories
                if !path
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    stack.push(path);
                }
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let sidecar = path.with_extension("sparkdown-sem");
                files.push(FileEntry {
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    path: path.to_string_lossy().into_owned(),
                    has_sidecar: sidecar.exists(),
                });
            }
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
