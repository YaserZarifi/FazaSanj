//! Tray icon, hide to tray on close, and start with Windows when background mode is on.

use fazasanj_model::{AppSettings, Language};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::state::AppState;

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let lang = app.state::<AppState>().settings().language;
    let (open, quit) = match lang {
        Language::Fa => ("باز کردن فضاسنج", "خروج"),
        Language::En => ("Open Fazasanj", "Quit"),
    };
    let open_i = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_i, &quit_i])?;
    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Fazasanj")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Background mode starts the app hidden with Windows so the weekly check can run.
pub fn apply(app: &AppHandle, s: &AppSettings) {
    let autolaunch = app.autolaunch();
    let want = s.tray_enabled && s.weekly_check;
    let is = autolaunch.is_enabled().unwrap_or(false);
    let result = if want && !is {
        autolaunch.enable()
    } else if !want && is {
        autolaunch.disable()
    } else {
        Ok(())
    };
    if let Err(e) = result {
        log::warn!("autostart change failed: {e}");
    }
}
