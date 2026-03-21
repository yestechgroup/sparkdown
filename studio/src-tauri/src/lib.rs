mod commands;
mod events;
mod registry;
mod session;
mod types;

use std::sync::Arc;

pub fn run() {
    let registry = Arc::new(registry::SessionRegistry::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(registry)
        .invoke_handler(tauri::generate_handler![
            commands::open_workspace,
            commands::list_workspace_files,
            commands::open_document,
            commands::close_document,
            commands::update_source,
            commands::get_entities_at,
            commands::export_document,
            commands::save_document,
            // Phase 2
            commands::create_entity,
            commands::update_stale_anchor,
            commands::dismiss_suggestion,
            commands::get_all_entities,
            commands::get_entity_details,
            commands::add_triple,
            commands::check_file_modified,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sparkdown Studio");
}
