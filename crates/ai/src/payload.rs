//! Builds the metadata-only JSON sent to a provider. The UI shows this exact value as the
//! preview, so whatever is added here is visible to the user before anything is sent.

use std::collections::HashMap;

use fazasanj_model::ExtensionStat;
use serde_json::{json, Value};

const MAX_CHILDREN: usize = 15;
const MAX_EXTENSIONS: usize = 10;
const USER_PLACEHOLDER: &str = "<user>";

/// What the app knows about a folder. There is deliberately no field for file contents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderMeta {
    pub path: String,
    pub total_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub top_extensions: Vec<ExtensionStat>,
    /// Unix ms.
    pub oldest_modified: Option<i64>,
    /// Unix ms.
    pub newest_modified: Option<i64>,
    /// Biggest children first.
    pub child_names: Vec<String>,
    /// Installed app that seems to own the folder (from the registry), if any.
    pub parent_app_hint: Option<String>,
}

/// Folder names that say nothing about the user and help the model a lot. Compared
/// case-insensitively. Vendor and app names are left out on purpose.
const WELL_KNOWN: &[&str] = &[
    "users",
    "public",
    "default",
    "windows",
    "system32",
    "syswow64",
    "winsxs",
    "temp",
    "tmp",
    "program files",
    "program files (x86)",
    "programdata",
    "appdata",
    "local",
    "locallow",
    "roaming",
    "desktop",
    "documents",
    "downloads",
    "pictures",
    "videos",
    "music",
    "cache",
    "caches",
    ".cache",
    "code cache",
    "gpucache",
    "shadercache",
    "cachestorage",
    "indexeddb",
    "service worker",
    "user data",
    "profiles",
    "logs",
    "log",
    "crashdumps",
    "crashpad",
    "backup",
    "backups",
    "updates",
    "update",
    "packages",
    "installer",
    "downloaded program files",
    "node_modules",
    ".git",
    ".npm",
    ".nuget",
    ".gradle",
    ".m2",
    ".cargo",
    ".rustup",
    "target",
    "build",
    "dist",
    "bin",
    "obj",
    "out",
    "__pycache__",
    "venv",
    ".venv",
    "vendor",
    "data",
    "config",
    "settings",
    "media",
    "images",
    "thumbnails",
    "saved games",
    "onedrive",
    "$recycle.bin",
    "system volume information",
    "recovery",
    "perflogs",
    "prefetch",
    "softwaredistribution",
    "fonts",
    "drivers",
];

fn is_well_known(name: &str) -> bool {
    let lower = name.to_lowercase();
    WELL_KNOWN.contains(&lower.as_str())
}

/// Builds the payload. `username` is the current Windows user name; it is always masked.
/// With `mask_names`, every other non-generic name becomes `folder_N` / `file_N.ext`.
pub fn build_payload(meta: &FolderMeta, mask_names: bool, username: &str) -> Value {
    let mut masker = Masker::new(mask_names, username);
    let path = masker.path(&meta.path);
    let children: Vec<String> = meta
        .child_names
        .iter()
        .take(MAX_CHILDREN)
        .map(|c| masker.name(c))
        .collect();
    let extensions: Vec<Value> = meta
        .top_extensions
        .iter()
        .take(MAX_EXTENSIONS)
        .map(|e| json!({ "ext": e.ext.to_lowercase(), "bytes": e.bytes, "files": e.files }))
        .collect();
    let app_hint = meta
        .parent_app_hint
        .as_deref()
        .map(|h| mask_username(h, username));
    json!({
        "path": path,
        "totalBytes": meta.total_bytes,
        "totalSize": human_size(meta.total_bytes),
        "fileCount": meta.file_count,
        "folderCount": meta.dir_count,
        "topExtensions": extensions,
        "oldestModified": meta.oldest_modified.map(iso_date),
        "newestModified": meta.newest_modified.map(iso_date),
        "biggestChildren": children,
        "installedAppHint": app_hint,
    })
}

/// Same as [`build_payload`]; named separately so the UI command reads clearly.
pub fn preview_payload(meta: &FolderMeta, mask_names: bool, username: &str) -> Value {
    build_payload(meta, mask_names, username)
}

struct Masker<'a> {
    mask_names: bool,
    username: &'a str,
    seen: HashMap<String, String>,
    folders: usize,
    files: usize,
}

impl<'a> Masker<'a> {
    fn new(mask_names: bool, username: &'a str) -> Self {
        Masker {
            mask_names,
            username,
            seen: HashMap::new(),
            folders: 0,
            files: 0,
        }
    }

    fn path(&mut self, raw: &str) -> String {
        let raw = raw.strip_prefix(r"\\?\").unwrap_or(raw).replace('/', "\\");
        let parts: Vec<&str> = raw.split('\\').collect();
        let mut out = Vec::with_capacity(parts.len());
        for (i, part) in parts.iter().enumerate() {
            let after_users = i > 0 && parts[i - 1].eq_ignore_ascii_case("users");
            if i == 0 && part.ends_with(':') || part.is_empty() {
                out.push((*part).to_owned());
            } else if after_users && !is_well_known(part) && !part.eq_ignore_ascii_case("all users")
            {
                // Whatever sits under \Users\ is a profile name, even if it is not ours.
                out.push(USER_PLACEHOLDER.to_owned());
            } else {
                out.push(self.component(part, true));
            }
        }
        out.join("\\")
    }

    fn name(&mut self, raw: &str) -> String {
        self.component(raw, false)
    }

    fn component(&mut self, raw: &str, in_path: bool) -> String {
        let unmasked = mask_username(raw, self.username);
        if !self.mask_names || is_well_known(raw) || unmasked == USER_PLACEHOLDER {
            return unmasked;
        }
        let key = raw.to_lowercase();
        if let Some(p) = self.seen.get(&key) {
            return p.clone();
        }
        // Path components are folders. A child with a normal looking extension is a file.
        let placeholder = match (in_path, extension(raw)) {
            (false, Some(ext)) => {
                self.files += 1;
                format!("file_{}.{}", self.files, ext.to_lowercase())
            }
            _ => {
                self.folders += 1;
                format!("folder_{}", self.folders)
            }
        };
        self.seen.insert(key, placeholder.clone());
        placeholder
    }
}

fn extension(name: &str) -> Option<&str> {
    let dot = name.rfind('.')?;
    if dot == 0 {
        return None;
    }
    let ext = &name[dot + 1..];
    let ok = (1..=8).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric());
    ok.then_some(ext)
}

/// Replaces every case-insensitive occurrence of `username` with `<user>`. Very short names
/// are skipped, masking "al" inside every word would wreck the payload.
fn mask_username(text: &str, username: &str) -> String {
    let username = username.trim();
    if username.chars().count() < 3 {
        return text.to_owned();
    }
    let needle: Vec<char> = username.chars().flat_map(char::to_lowercase).collect();
    let hay: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < hay.len() {
        if let Some(len) = match_at(&hay[i..], &needle) {
            out.push_str(USER_PLACEHOLDER);
            i += len;
        } else {
            out.push(hay[i].1);
            i += 1;
        }
    }
    out
}

/// If `needle` (already lowercased) matches at the start of `hay`, returns how many chars of
/// `hay` it used.
fn match_at(hay: &[(usize, char)], needle: &[char]) -> Option<usize> {
    let mut n = 0;
    let mut used = 0;
    for &(_, c) in hay {
        if n == needle.len() {
            break;
        }
        for lc in c.to_lowercase() {
            if needle.get(n) != Some(&lc) {
                return None;
            }
            n += 1;
        }
        used += 1;
    }
    (n == needle.len()).then_some(used)
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

/// Unix ms to "YYYY-MM-DD" (UTC). Days are enough for the model, and it avoids a date crate.
fn iso_date(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> FolderMeta {
        FolderMeta {
            path: r"C:\Users\Yaser\AppData\Local\SecretProject\cache".into(),
            total_bytes: 5 * 1024 * 1024 * 1024,
            file_count: 1200,
            dir_count: 40,
            top_extensions: vec![
                ExtensionStat {
                    ext: "MP4".into(),
                    bytes: 4_000_000_000,
                    files: 10,
                },
                ExtensionStat {
                    ext: "tmp".into(),
                    bytes: 1_000,
                    files: 1000,
                },
            ],
            oldest_modified: Some(0),
            newest_modified: Some(1_700_000_000_000),
            child_names: vec![
                "yaser_backup_2023".into(),
                "holiday.mp4".into(),
                "Temp".into(),
                "Private Stuff".into(),
            ],
            parent_app_hint: Some("Secret App".into()),
        }
    }

    #[test]
    fn username_always_masked() {
        let p = build_payload(&meta(), false, "yaser");
        let s = p.to_string();
        assert!(!s.to_lowercase().contains("yaser"), "{s}");
        assert_eq!(
            p["path"],
            r"C:\Users\<user>\AppData\Local\SecretProject\cache"
        );
        assert_eq!(p["biggestChildren"][0], "<user>_backup_2023");
        assert_eq!(p["biggestChildren"][1], "holiday.mp4");
    }

    #[test]
    fn other_profiles_are_masked_too() {
        let mut m = meta();
        m.path = r"\\?\D:\users\Someone Else\Videos".into();
        let p = build_payload(&m, false, "yaser");
        assert_eq!(p["path"], r"D:\users\<user>\Videos");
        m.path = r"C:\Users\Public\Documents".into();
        assert_eq!(
            build_payload(&m, false, "yaser")["path"],
            r"C:\Users\Public\Documents"
        );
    }

    #[test]
    fn mask_names_keeps_generic_parts_and_extensions() {
        let p = build_payload(&meta(), true, "Yaser");
        assert_eq!(p["path"], r"C:\Users\<user>\AppData\Local\folder_1\cache");
        let kids: Vec<&str> = p["biggestChildren"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(kids, vec!["folder_2", "file_1.mp4", "Temp", "folder_3"]);
        let s = p.to_string();
        assert!(!s.contains("SecretProject") && !s.contains("holiday") && !s.contains("Private"));
    }

    #[test]
    fn same_name_gets_same_placeholder() {
        let mut m = meta();
        m.path = r"D:\Work\Work".into();
        m.child_names = vec!["work".into()];
        let p = build_payload(&m, true, "nobody");
        assert_eq!(p["path"], r"D:\folder_1\folder_1");
        assert_eq!(p["biggestChildren"][0], "folder_1");
    }

    #[test]
    fn only_metadata_fields() {
        let p = build_payload(&meta(), false, "yaser");
        let mut keys: Vec<&str> = p.as_object().unwrap().keys().map(|k| k.as_str()).collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "biggestChildren",
                "fileCount",
                "folderCount",
                "installedAppHint",
                "newestModified",
                "oldestModified",
                "path",
                "topExtensions",
                "totalBytes",
                "totalSize",
            ]
        );
        assert_eq!(p["topExtensions"][0]["ext"], "mp4");
        assert_eq!(p["totalSize"], "5.0 GB");
        assert_eq!(p["oldestModified"], "1970-01-01");
        assert_eq!(p["newestModified"], "2023-11-14");
        assert_eq!(preview_payload(&meta(), false, "yaser"), p);
    }

    #[test]
    fn children_are_capped() {
        let mut m = meta();
        m.child_names = (0..40).map(|i| format!("c{i}")).collect();
        let p = build_payload(&m, false, "yaser");
        assert_eq!(p["biggestChildren"].as_array().unwrap().len(), MAX_CHILDREN);
    }

    #[test]
    fn username_mask_handles_unicode_and_short_names() {
        assert_eq!(mask_username("پوشه یاسر", "یاسر"), "پوشه <user>");
        assert_eq!(mask_username("ALIREZA-pc", "alireza"), "<user>-pc");
        assert_eq!(mask_username("alias", "al"), "alias");
    }

    #[test]
    fn dates() {
        assert_eq!(iso_date(951_782_400_000), "2000-02-29");
        assert_eq!(iso_date(-86_400_000), "1969-12-31");
    }
}
