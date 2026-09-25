//! Big user files nobody has used in a long time.

use std::collections::HashMap;

use fazasanj_model::{Bilingual, FindingDetails, HeuristicFinding, HeuristicKind, SafetyLevel};

use crate::util::{fa_num, has_part, is_system_location, months_between, parent, MONTH_MS};
use crate::FileRecord;

/// Keeps the UI list short even on huge drives.
const MAX_FINDINGS: usize = 500;
/// Small stale files are reported per folder once their folder adds up to this.
const MIN_FOLDER_BYTES: u64 = 50 * 1024 * 1024;
/// More big stale files than this in one folder become one folder finding.
const MAX_FILES_PER_FOLDER: usize = 25;

/// Folders that belong to programs, not to the user, even inside the profile.
const NOT_USER_FILES: &[&str] = &[
    "appdata", "node_modules", ".git", ".svn", ".hg", ".cargo", ".rustup", ".nuget", ".gradle", ".m2", ".npm",
    ".cache", ".vscode", ".idea", ".android", "$recycle.bin", "system volume information", "site-packages",
    "__pycache__", ".venv", "venv", "target",
];

/// Whether NTFS keeps last access times on this PC. Reads
/// `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\NtfsDisableLastAccessUpdate`.
pub fn access_time_reliable() -> bool {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let value = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\FileSystem")
        .and_then(|k| k.get_value::<u32, _>("NtfsDisableLastAccessUpdate"))
        .ok();
    last_access_enabled(value)
}

/// 0 and 0x80000000 (user set) or 0x80000002 (system managed) mean updates are on; 1, 0x80000001
/// and 0x80000003 mean off. The lowest bit is the "disabled" flag in all of them. A missing value
/// is treated as off because we can not tell.
pub(crate) fn last_access_enabled(value: Option<u32>) -> bool {
    match value {
        Some(v) => v & 1 == 0,
        None => false,
    }
}

pub fn find_stale(files: &[FileRecord], months: u32, now_ms: i64, atime_reliable: bool, min_size: u64) -> Vec<HeuristicFinding> {
    let cutoff = now_ms - i64::from(months.max(1)) * MONTH_MS;
    let min_size = min_size.max(1);

    let mut big: HashMap<&str, Vec<(&FileRecord, i64)>> = HashMap::new();
    let mut small: HashMap<&str, Vec<(&FileRecord, i64)>> = HashMap::new();
    for f in files {
        if f.size == 0 || is_system_location(&f.path) || has_part(&f.path, NOT_USER_FILES) {
            continue;
        }
        let Some(last) = last_used(f, atime_reliable) else {
            continue;
        };
        if last >= cutoff {
            continue;
        }
        let bucket = if f.size >= min_size { &mut big } else { &mut small };
        bucket.entry(parent(&f.path)).or_default().push((f, last));
    }

    let mut out = Vec::new();
    let folder_min = MIN_FOLDER_BYTES.max(min_size);
    for (folder, list) in big.iter() {
        if list.len() > MAX_FILES_PER_FOLDER {
            out.push(folder_finding(folder, list, months, now_ms, atime_reliable));
        } else {
            for (f, last) in list {
                out.push(file_finding(f, *last, months, now_ms, atime_reliable));
            }
        }
    }
    for (folder, list) in small.iter() {
        let bytes: u64 = list.iter().map(|(f, _)| f.size).sum();
        if bytes >= folder_min && list.len() >= 2 {
            out.push(folder_finding(folder, list, months, now_ms, atime_reliable));
        }
    }
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    out.truncate(MAX_FINDINGS);
    out
}

fn last_used(f: &FileRecord, atime_reliable: bool) -> Option<i64> {
    match (f.modified, f.accessed) {
        (Some(m), Some(a)) if atime_reliable => Some(m.max(a)),
        (None, Some(a)) if atime_reliable => Some(a),
        (m, _) => m,
    }
}

/// Older than the limit gives more confidence; without real access times we know less.
fn confidence(last: i64, months: u32, now_ms: i64, atime_reliable: bool) -> f32 {
    let age = months_between(last, now_ms);
    let limit = months.max(1);
    let mut c: f32 = if atime_reliable { 0.55 } else { 0.35 };
    if age >= limit * 3 {
        c += 0.15;
    } else if age >= limit * 2 {
        c += 0.1;
    }
    c.clamp(0.1, 0.8)
}

fn file_finding(f: &FileRecord, last: i64, months: u32, now_ms: i64, reliable: bool) -> HeuristicFinding {
    let age = months_between(last, now_ms);
    HeuristicFinding {
        kind: HeuristicKind::Stale,
        path: f.path.clone(),
        node_id: f.node_id,
        bytes: f.size,
        confidence: confidence(last, months, now_ms, reliable),
        safety: SafetyLevel::Careful,
        reason: reason(age, None, reliable),
        details: FindingDetails::Stale {
            last_modified: f.modified,
            last_accessed: if reliable { f.accessed } else { None },
            access_time_reliable: reliable,
        },
    }
}

fn folder_finding(folder: &str, list: &[(&FileRecord, i64)], months: u32, now_ms: i64, reliable: bool) -> HeuristicFinding {
    // The newest file decides how old the group is.
    let last = list.iter().map(|(_, l)| *l).max().unwrap_or(now_ms);
    let modified = list.iter().filter_map(|(f, _)| f.modified).max();
    let accessed = if reliable { list.iter().filter_map(|(f, _)| f.accessed).max() } else { None };
    let age = months_between(last, now_ms);
    HeuristicFinding {
        kind: HeuristicKind::Stale,
        path: folder.to_string(),
        node_id: None,
        bytes: list.iter().map(|(f, _)| f.size).sum(),
        confidence: confidence(last, months, now_ms, reliable) - 0.05,
        safety: SafetyLevel::Careful,
        reason: reason(age, Some(list.len()), reliable),
        details: FindingDetails::Stale { last_modified: modified, last_accessed: accessed, access_time_reliable: reliable },
    }
}

fn reason(age_months: u32, files_in_folder: Option<usize>, reliable: bool) -> Bilingual {
    let m = age_months;
    let (mut fa, mut en) = match (files_in_folder, reliable) {
        (None, true) => (
            format!("حدود {} ماه است که این فایل باز یا تغییر داده نشده است.", fa_num(m)),
            format!("This file has not been opened or changed in about {m} months."),
        ),
        (None, false) => (
            format!("حدود {} ماه است که این فایل تغییر نکرده است.", fa_num(m)),
            format!("This file has not been changed in about {m} months."),
        ),
        (Some(n), true) => (
            format!("{} فایل در این پوشه دست‌کم {} ماه است که باز یا تغییر داده نشده‌اند.", fa_num(n), fa_num(m)),
            format!("{n} files in this folder have not been opened or changed in at least {m} months."),
        ),
        (Some(n), false) => (
            format!("{} فایل در این پوشه دست‌کم {} ماه است که تغییر نکرده‌اند.", fa_num(n), fa_num(m)),
            format!("{n} files in this folder have not been changed in at least {m} months."),
        ),
    };
    if !reliable {
        fa.push_str(" ویندوز روی این رایانه زمان باز شدن فایل‌ها را ثبت نمی‌کند، پس فقط تاریخ آخرین تغییر را در نظر گرفتیم.");
        en.push_str(" Windows does not record when files are opened on this PC, so we could only use the last change date.");
    }
    fa.push_str(" قدیمی بودن به معنای بی‌مصرف بودن نیست؛ پیش از پاک کردن بررسی کنید.");
    en.push_str(" Old does not mean useless, so check before deleting.");
    Bilingual::new(fa, en)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_760_000_000_000;
    const MB: u64 = 1024 * 1024;

    fn file(path: &str, size: u64, modified_months: i64, accessed_months: Option<i64>) -> FileRecord {
        FileRecord {
            node_id: Some(3),
            path: path.into(),
            size,
            modified: Some(NOW - modified_months * MONTH_MS),
            accessed: accessed_months.map(|a| NOW - a * MONTH_MS),
        }
    }

    #[test]
    fn registry_values() {
        assert!(last_access_enabled(Some(0)));
        assert!(last_access_enabled(Some(0x8000_0000)));
        assert!(last_access_enabled(Some(0x8000_0002)));
        assert!(!last_access_enabled(Some(1)));
        assert!(!last_access_enabled(Some(0x8000_0001)));
        assert!(!last_access_enabled(Some(0x8000_0003)));
        assert!(!last_access_enabled(None));
        let _ = access_time_reliable();
    }

    #[test]
    fn uses_access_time_only_when_reliable() {
        let files = vec![file(r"D:\Videos\old.mkv", 900 * MB, 30, Some(1))];
        assert!(find_stale(&files, 12, NOW, true, 100 * MB).is_empty(), "opened last month");
        let found = find_stale(&files, 12, NOW, false, 100 * MB);
        assert_eq!(found.len(), 1);
        let f = &found[0];
        assert!(f.reason.en.contains("does not record"));
        assert!(f.reason.fa.contains("ثبت نمی‌کند"));
        assert!(matches!(f.details, FindingDetails::Stale { access_time_reliable: false, last_accessed: None, .. }));
    }

    #[test]
    fn reliable_atime_gives_more_confidence() {
        let files = vec![file(r"D:\Videos\old.mkv", 900 * MB, 30, Some(30))];
        let a = find_stale(&files, 12, NOW, true, 100 * MB);
        let b = find_stale(&files, 12, NOW, false, 100 * MB);
        assert!(a[0].confidence > b[0].confidence);
        for f in a.iter().chain(&b) {
            assert_ne!(f.safety, SafetyLevel::Safe);
            assert!(f.confidence > 0.0 && f.confidence < 1.0);
        }
    }

    #[test]
    fn skips_system_and_program_places() {
        let files = vec![
            file(r"C:\Windows\Installer\big.msi", 900 * MB, 60, None),
            file(r"C:\Program Files\App\data.bin", 900 * MB, 60, None),
            file(r"C:\Users\me\AppData\Local\App\cache.bin", 900 * MB, 60, None),
            file(r"C:\Users\me\code\proj\node_modules\x\big.bin", 900 * MB, 60, None),
            file(r"C:\Users\me\Documents\recent.docx", 900 * MB, 1, None),
        ];
        assert!(find_stale(&files, 12, NOW, false, 100 * MB).is_empty());
    }

    #[test]
    fn groups_small_files_by_folder() {
        let mut files: Vec<FileRecord> =
            (0..2000).map(|i| file(&format!(r"D:\Photos\2015\IMG_{i}.jpg"), 3 * MB, 100, None)).collect();
        files.push(file(r"D:\Photos\2015\IMG_new.jpg", 3 * MB, 1, None));
        files.extend((0..3).map(|i| file(&format!(r"D:\Notes\n{i}.txt"), 1024, 100, None)));
        let found = find_stale(&files, 12, NOW, false, 100 * MB);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, r"D:\Photos\2015");
        assert_eq!(found[0].bytes, 2000 * 3 * MB);
        assert!(found[0].node_id.is_none());
        assert!(found[0].reason.en.starts_with("2000 files"));
    }

    #[test]
    fn many_big_files_in_one_folder_become_one_finding() {
        let files: Vec<FileRecord> =
            (0..100).map(|i| file(&format!(r"D:\Rips\m{i}.mkv"), 200 * MB, 40, None)).collect();
        let found = find_stale(&files, 12, NOW, false, 100 * MB);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].bytes, 100 * 200 * MB);
    }

    #[test]
    fn result_count_is_capped() {
        let files: Vec<FileRecord> =
            (0..5000).map(|i| file(&format!(r"D:\f{}\x.iso", i), 200 * MB, 40, None)).collect();
        assert_eq!(find_stale(&files, 12, NOW, false, 100 * MB).len(), MAX_FINDINGS);
    }
}
