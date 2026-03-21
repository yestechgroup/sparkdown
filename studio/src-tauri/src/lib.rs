mod commands;
mod events;
mod pack_types;
mod registry;
mod session;
mod types;

use std::sync::Arc;
use tokio::sync::RwLock;

use sparkdown_ontology::registry::ThemeRegistry;

pub type ThemeRegistryState = Arc<RwLock<ThemeRegistry>>;

pub fn run() {
    let registry = Arc::new(registry::SessionRegistry::new());
    let theme_registry: ThemeRegistryState =
        Arc::new(RwLock::new(ThemeRegistry::with_builtins()));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(registry)
        .manage(theme_registry)
        .invoke_handler(tauri::generate_handler![
            commands::open_workspace,
            commands::list_workspace_files,
            commands::open_document,
            commands::close_document,
            commands::update_source,
            commands::get_entities_at,
            commands::export_document,
            commands::save_document,
            commands::create_entity,
            commands::update_stale_anchor,
            commands::get_document_overview,
            commands::get_entity_detail,
            commands::delete_entity,
            commands::list_available_types,
            commands::search_types,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sparkdown Studio");
}
