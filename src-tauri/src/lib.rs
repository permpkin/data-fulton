pub mod api;
pub mod catalog;
mod commands;
pub mod downloader;
pub mod extractor;
mod keyhelper;
pub mod limiter;
pub mod store;
pub mod types;

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;

use store::Store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data: PathBuf = app.path().app_data_dir().expect("app data dir");
            std::fs::create_dir_all(&app_data).ok();

            // Migrate data written before the gimme → datafulton rename.
            if !app_data.join("fulton.db").exists() {
                if let Some(old) = app_data.parent().map(|p| p.join("com.gimme.extractor")) {
                    for f in ["gimme.db", "gimme.db-wal", "gimme.db-shm"] {
                        let src = old.join(f);
                        if src.exists() {
                            let _ = std::fs::copy(&src, app_data.join(f.replace("gimme", "datafulton").replace("fulton", "datafulton")));
                        }
                    }
                }
            }

            let store = Arc::new(Store::open(&app_data.join("fulton.db")).expect("open db"));
            store.sweep_interrupted().ok();

            // Startup window is created here (not via tauri.conf) so it gets
            // the same hidden-until-page-loaded behaviour as the others.
            commands::open_connections_window(app.handle());

            app.manage(commands::build_state(app.handle(), store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_get,
            commands::settings_set,
            commands::choose_dir,
            commands::connections_list,
            commands::connection_save,
            commands::connection_delete,
            commands::open_connections,
            commands::connection_validate,
            commands::connection_test,
            commands::connection_open,
            commands::connection_current,
            commands::connection_key_helper,
            commands::catalog_list,
            commands::session_create,
            commands::session_start,
            commands::session_pause,
            commands::session_resume,
            commands::session_restart,
            commands::session_cancel,
            commands::session_list,
            commands::session_snapshot,
            commands::session_update_config,
            commands::session_save_config,
            commands::session_delete,
            commands::session_clear_finished,
            commands::session_export_log,
            commands::session_open_export,
        ])
        .build(tauri::generate_context!())
        .expect("error while building datafulton")
        .run(|app, event| {
            // Closing the extractions window reopens the connections launcher.
            if let tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::Destroyed,
                ..
            } = event
            {
                if label == "main" && app.get_webview_window("connections").is_none() {
                    commands::open_connections_window(app);
                }
            }
        });
}
