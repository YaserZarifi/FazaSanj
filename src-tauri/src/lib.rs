mod background;
mod commands;
mod scans;
mod state;
mod tray;

use fazasanj_rules::RuleSet;
use fazasanj_safety::SafetyContext;
use fazasanj_store::Store;
use tauri::{Manager, WindowEvent};

use crate::state::AppState;

fn init_state(app: &tauri::App) -> Result<AppState, Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let store = Store::open(&data_dir.join("fazasanj.db"))?;
    let rules = RuleSet::load_embedded()?;
    let install_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let mut safety = SafetyContext::from_env(None);
    // In dev the exe sits in target\debug, which is fine to protect as well.
    if let Some(dir) = &install_dir {
        safety.protect_tree(dir);
    }
    safety.protect_tree(&data_dir);
    Ok(AppState::new(store, rules, safety, data_dir))
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .setup(|app| {
            let state = init_state(app)?;
            let settings = state.settings();
            app.manage(state);
            tray::setup(app.handle())?;
            background::start(app.handle().clone());
            let minimized = std::env::args().any(|a| a == "--minimized");
            if minimized && settings.tray_enabled {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let tray_mode = window.app_handle().state::<AppState>().settings().tray_enabled;
                if tray_mode {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::drives::list_drives,
            commands::scan::start_scan,
            commands::scan::cancel_scan,
            commands::scan::get_scan_summary,
            commands::scan::get_node,
            commands::scan::get_children,
            commands::scan::get_treemap,
            commands::scan::get_largest_files,
            commands::scan::get_by_type,
            commands::scan::get_access_denied,
            commands::scan::get_story,
            commands::scan::compare_scanners,
            commands::heuristics::run_heuristics,
            commands::cleanup::build_cleanup_plan,
            commands::cleanup::run_cleanup,
            commands::cleanup::get_cleanup_history,
            commands::system::reveal_in_explorer,
            commands::system::open_recycle_bin,
            commands::system::open_app_setting,
            commands::system::get_app_info,
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
            commands::settings::get_settings,
            commands::settings::set_settings,
            commands::settings::reset_everything,
            commands::update::check_update,
            commands::update::install_update,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("fazasanj failed to start: {e}");
            std::process::exit(1);
        });
}
