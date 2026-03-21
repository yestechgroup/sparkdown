use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::registry::{SessionCommand, SessionRegistry};
use crate::session::DocumentSession;
use crate::types::{DocId, EntityDto, FileEntry, RenderFormat, WorkspaceInfo};

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
