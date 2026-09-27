//! Background checks while the app sits in the tray: low free space and a weekly scan of the
//! system drive that warns when something grew fast.

use std::path::PathBuf;
use std::time::Duration;

use fazasanj_model::{DriveKind, Language, ScanMode};
use fazasanj_scan::ScanOptions;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::{now_ms, AppState};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
const GB: u64 = 1024 * 1024 * 1024;

pub fn start(app: AppHandle) {
    let _ = std::thread::Builder::new().name("background".into()).spawn(move || loop {
        std::thread::sleep(Duration::from_secs(20 * 60));
        tick(&app);
    });
}

fn get_ts(state: &AppState, key: &str) -> i64 {
    state.store.get_value(key).ok().flatten().and_then(|v| v.parse().ok()).unwrap_or(0)
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    if let Err(e) = app.notification().builder().title(title).body(body).show() {
        log::warn!("notification failed: {e}");
    }
}

fn gb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / GB as f64)
}

/// Persian digits and decimal separator for Persian notifications.
fn fa_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32(0x06F0 + (c as u32 - '0' as u32)).unwrap_or(c),
            '.' => '٫',
            _ => c,
        })
        .collect()
}

fn tick(app: &AppHandle) {
    let state = app.state::<AppState>();
    let s = state.settings();
    if !s.tray_enabled {
        return;
    }
    let now = now_ms();
    let fa = s.language == Language::Fa;

    if now - get_ts(&state, "bg.low_space_notice") > DAY_MS {
        let threshold = u64::from(s.low_space_threshold_gb) * GB;
        let low: Vec<String> = fazasanj_platform::list_drives()
            .unwrap_or_default()
            .into_iter()
            .filter(|d| d.kind == DriveKind::Fixed && d.free < threshold)
            .map(|d| format!("{} ({} GB)", d.letter, gb(d.free)))
            .collect();
        if !low.is_empty() {
            let list = low.join(", ");
            if fa {
                let list = fa_digits(&list.replace(" GB", " گیگابایت"));
                notify(app, "فضای درایو کم شده", &format!("فضای خالی این درایوها کم است: {list}. فضاسنج را باز کنید تا ببینید چه چیزی جا گرفته."));
            } else {
                notify(app, "Running low on space", &format!("These drives are almost full: {list}. Open Fazasanj to see what's taking the space."));
            }
            let _ = state.store.set_value("bg.low_space_notice", &now.to_string());
        }
    }

    if s.weekly_check && now - get_ts(&state, "bg.weekly_scan") > 7 * DAY_MS {
        let _ = state.store.set_value("bg.weekly_scan", &now.to_string());
        weekly_scan(app, fa);
    }
}

fn weekly_scan(app: &AppHandle, fa: bool) {
    let state = app.state::<AppState>();
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    let root = PathBuf::from(format!("{drive}\\"));
    let mut opts = ScanOptions::new(root.clone(), ScanMode::Normal);
    opts.excluded = state.settings().excluded_paths.iter().map(PathBuf::from).collect();
    let Ok(outcome) = fazasanj_scan::run_scan(opts) else {
        return;
    };
    let tree = outcome.tree;
    let sum = tree.summary();
    let space = fazasanj_platform::disk_space(&root).ok();
    let root_path = fazasanj_scan::display_path(&root);
    let Ok(id) = state.store.save_snapshot(
        &root_path,
        now_ms(),
        sum.total_bytes,
        sum.files,
        space.as_ref().map(|s| s.total).unwrap_or(0),
        space.as_ref().map(|s| s.free).unwrap_or(0),
        &tree.folder_sizes(4),
    ) else {
        return;
    };
    let Ok(Some(prev)) = state.store.latest_before(&root_path, id) else {
        return;
    };
    let Ok(cmp) = state.store.compare(prev.id, id, 20) else {
        return;
    };
    // Deepest single folder that grew by 3 GB or more.
    let grew = cmp
        .items
        .iter()
        .filter(|i| i.delta >= 3 * GB as i64)
        .max_by_key(|i| (i.path.matches('\\').count(), i.delta));
    if let Some(item) = grew {
        let size = gb(item.delta as u64);
        if fa {
            notify(app, "یک پوشه سریع بزرگ شده", &format!("{} در یک هفته {} گیگابایت بزرگ‌تر شده است.", item.path, fa_digits(&size)));
        } else {
            notify(app, "A folder grew fast", &format!("{} grew by {} GB in a week.", item.path, size));
        }
    }
}
