//! Optional tray icon. With it on, closing the window keeps the app running for the weekly
//! check and low space warnings.

use std::sync::Arc;

use fazasanj_app::App;
use fazasanj_model::Language;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

const TRAY_ID: &str = "main";

/// Adds or removes the tray icon to match the setting.
pub fn sync(app: &AppHandle, enabled: bool) {
    let exists = app.tray_by_id(TRAY_ID).is_some();
    if enabled && !exists {
        if let Err(e) = build(app) {
            log::warn!("could not create tray icon: {e}");
        }
    } else if !enabled && exists {
        let _ = app.remove_tray_by_id(TRAY_ID);
    }
}

fn build(app: &AppHandle) -> tauri::Result<()> {
    let fa = app.state::<Arc<App>>().language() == Language::Fa;
    let (open, quit) = if fa { ("باز کردن فضاسنج", "خروج") } else { ("Open Fazasanj", "Quit") };
    let open = MenuItem::with_id(app, "open", open, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Fazasanj")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => crate::show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
