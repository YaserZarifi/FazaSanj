mod commands;
mod tray;
mod watch;

use std::path::Path;
use std::sync::Arc;

use fazasanj_app::{App, Event, EventSink};
use fazasanj_store::Store;
use tauri::{Emitter, Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = Store::open(&data_dir.join("fazasanj.db"))?;
            let handle = app.handle().clone();
            let sink: EventSink = Arc::new(move |e: Event| {
                if let Err(err) = handle.emit(e.name(), e.payload()) {
                    log::warn!("could not emit {}: {err}", e.name());
                }
            });
            let install_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf));
            let core = App::new(store, sink, None, install_dir)
                .map_err(|e| format!("{}: {}", e.code, e.detail.unwrap_or_default()))?;
            let tray_enabled = core.settings().tray_enabled;
            app.manage(core);
            tray::sync(app.handle(), tray_enabled);
            watch::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let keep_running = window.state::<Arc<App>>().settings().tray_enabled;
                if keep_running {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::drives::list_drives,
            commands::scan::start_scan,
            commands::scan::cancel_scan,
            commands::scan::close_scan,
            commands::scan::get_scan_summary,
            commands::scan::get_node,
            commands::scan::get_children,
            commands::scan::get_treemap,
            commands::scan::get_largest_files,
            commands::scan::get_by_type,
            commands::scan::get_access_denied,
            commands::scan::get_story,
            commands::scan::compare_scanners,
            commands::scan::run_heuristics,
            commands::cleanup::build_cleanup_plan,
            commands::cleanup::run_cleanup,
            commands::cleanup::get_cleanup_history,
            commands::cleanup::reveal_in_explorer,
            commands::cleanup::open_recycle_bin,
            commands::cleanup::open_app_setting,
            commands::ai::ai_get_providers,
            commands::ai::ai_set_key,
            commands::ai::ai_delete_key,
            commands::ai::ai_test_key,
            commands::ai::ai_set_model,
            commands::ai::ai_preview_payload,
            commands::ai::ai_explain,
            commands::snapshots::list_snapshots,
            commands::snapshots::compare_snapshots,
            commands::snapshots::compare_with_last,
            commands::snapshots::get_growth,
            commands::app::get_settings,
            commands::app::set_settings,
            commands::app::reset_everything,
            commands::app::get_app_info,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fazasanj failed to start: {e}");
            std::process::exit(1);
        });
}

/// Brings the main window back from the tray.
pub(crate) fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
