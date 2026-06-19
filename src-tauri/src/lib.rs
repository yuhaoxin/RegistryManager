pub mod audit;
pub mod commands;
pub mod credentials;
pub mod docker;
pub mod registry;
pub mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = tauri::async_runtime::block_on(commands::AppState::new())
        .expect("failed to initialize registry-manager application state");

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::docker::get_docker_status,
            commands::registry::list_registry_profiles,
            commands::registry::create_registry_profile,
            commands::registry::update_registry_profile,
            commands::registry::delete_registry_profile,
            commands::registry::select_registry_profile,
            commands::registry::set_registry_credentials,
            commands::registry::clear_registry_credentials,
            commands::registry::get_selected_registry_profile,
            commands::registry::check_registry_health,
            commands::registry::list_catalog,
            commands::registry::list_tags,
            commands::registry::get_manifest,
            commands::registry::refresh_registry,
            commands::registry::cancel_refresh,
            commands::cache::get_cached_repositories,
            commands::cache::get_cached_tags,
            commands::delete::get_delete_impact,
            commands::delete::delete_manifest,
            commands::delete::delete_repository,
            commands::gc::run_local_gc,
            commands::audit::list_audit_events,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
