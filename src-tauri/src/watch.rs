//! Background check for low disk space and the weekly summary. Only active while the tray
//! icon or the weekly check is turned on.

use std::sync::Arc;
use std::time::Duration;

use fazasanj_app::{notices, App};
use fazasanj_model::DriveKind;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const FIRST_CHECK: Duration = Duration::from_secs(90);
const INTERVAL: Duration = Duration::from_secs(60 * 60);
const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const WEEK_MS: i64 = 7 * DAY_MS;
const GB: u64 = 1024 * 1024 * 1024;

pub fn start(app: AppHandle) {
    let spawned = std::thread::Builder::new().name("watch".into()).spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            check(&app);
            std::thread::sleep(INTERVAL);
        }
    });
    if let Err(e) = spawned {
        log::warn!("could not start the space watcher: {e}");
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn check(app: &AppHandle) {
    let core = app.state::<Arc<App>>().inner().clone();
    let settings = core.settings();
    if !settings.tray_enabled && !settings.weekly_check {
        return;
    }
    let Ok(drives) = fazasanj_platform::list_drives() else {
        return;
    };
    let lang = settings.language;
    let store = core.store();
    let now = now_ms();
    let threshold = u64::from(settings.low_space_threshold_gb) * GB;

    for d in drives.iter().filter(|d| d.kind == DriveKind::Fixed) {
        if d.free < threshold {
            let key = format!("notified_low.{}", d.letter);
            let last = store.get_value(&key).ok().flatten().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
            if now - last >= DAY_MS {
                let (title, body) = notices::low_space(&d.letter, d.free, lang);
                notify(app, &title, &body);
                let _ = store.set_value(&key, &now.to_string());
            }
        }
    }

    if settings.weekly_check {
        let last = store.get_value("weekly_at").ok().flatten().and_then(|v| v.parse::<i64>().ok()).unwrap_or(0);
        if now - last >= WEEK_MS {
            for d in drives.iter().filter(|d| d.kind == DriveKind::Fixed) {
                let key = format!("weekly_free.{}", d.letter);
                let before = store.get_value(&key).ok().flatten().and_then(|v| v.parse::<u64>().ok());
                if let Some(before) = before {
                    let change = d.free as i64 - before as i64;
                    if change.unsigned_abs() >= GB / 2 {
                        let (title, body) = notices::weekly(&d.letter, d.free, change, lang);
                        notify(app, &title, &body);
                    }
                }
                let _ = store.set_value(&key, &d.free.to_string());
            }
            let _ = store.set_value("weekly_at", &now.to_string());
        }
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("could not show notification: {e}");
    }
}
