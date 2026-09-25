//! AppData and ProgramData folders whose program seems to be gone.

use fazasanj_model::{Bilingual, FindingDetails, HeuristicFinding, HeuristicKind, SafetyLevel};

use crate::names::{normalize, similarity, Name};
use crate::util::{fa_num, months_between, now_ms, parts, MONTH_MS};
use crate::{DirRecord, InstalledApp};

/// At or above this score the folder is treated as belonging to an installed program.
const INSTALLED_SCORE: f32 = 0.7;
const MIN_SIZE: u64 = 1024 * 1024;
/// Folders changed this recently are probably in use (portable apps, services).
const RECENT_MS: i64 = 14 * 24 * 60 * 60 * 1000;

/// Folders that belong to Windows or are shared by many programs. Lowercase.
const BUILT_IN: &[&str] = &[
    // Windows and Microsoft
    "microsoft", "packages", "temp", "tmp", "programs", "crashdumps", "d3dscache", "connecteddevicesplatform",
    "placeholdertilelogofolder", "publishers", "virtualstore", "comms", "history", "elevateddiagnostics",
    "diagnostics", "windows", "windowsapps", "microsoftedge", "microsoft help", "identitynexusintegration",
    "peerdistrepub", "fontcache", "iconcache", "application data", "desktop", "documents", "templates",
    "start menu", "favorites", "package cache", "regid.1991-06.com.microsoft", "ssh", "usoshared", "usoprivate",
    "softwaredistribution", "windowsholographicdevices", "microsoft onedrive", "onedrive", "windowspowershell",
    "powershell", "dbg", "sysinternals", "gamedvr", "microsoft sdks", "nuget", "vstelemetry", "servicehub",
    "visualstudio", ".identityservice", "ms-playwright", "microsoft_corporation",
    // shared runtimes and dev tool caches
    "npm", "npm-cache", "pip", "pypa", "python", "pnpm", "pnpm-cache", "pnpm-state", "yarn", "node-gyp",
    "electron", "electron-builder", "squirreltemp", "cef", "java", "sun", "oracle", "jedi", "virtualenv",
    "go-build", "gradle", "deno", "bun", "cargo", "rustup", "typescript", "code cache", "gtk-3.0", "fontconfig",
    "downloaded installations", "installshield", "installshield installation information", "system.data.sqlite",
    "chocolatey", "scoop", "winget", "cache", "caches", "logs", "log", "crashreports", "crashpad", "sentry",
    "backup", "backups", "updates", "data", "config", "settings", "user data", "userdata", "profiles", "default",
    "shared", "common", "common files", "fonts", "certificates", "licenses", "launcher", "updater", "update",
    "bin", "app", "plugins", "extensions", "user", "users", "local", "roaming", "locallow", "storage",
    // big vendors that own many products, or hardware drivers
    "google", "mozilla", "apple", "apple computer", "adobe", "macromedia", "intel", "amd", "ati", "nvidia",
    "nvidia corporation", "realtek", "dolby", "waves", "synaptics", "elan", "conexant", "killer networking",
    "rivet networks", "dell", "hp", "hewlett-packard", "lenovo", "asus", "acer", "msi", "samsung", "logitech",
    "logishrd", "razer", "corsair", "qualcomm", "broadcom", "ibm", "vmware", "oem",
];

pub fn find_orphans(appdata_dirs: &[DirRecord], installed: &[InstalledApp]) -> Vec<HeuristicFinding> {
    find_orphans_at(appdata_dirs, installed, now_ms())
}

/// Same as [`find_orphans`] with a fixed clock, for tests.
pub fn find_orphans_at(appdata_dirs: &[DirRecord], installed: &[InstalledApp], now_ms: i64) -> Vec<HeuristicFinding> {
    let index = Index::new(installed);
    let mut dirs: Vec<&DirRecord> = appdata_dirs.iter().collect();
    dirs.sort_by_key(|a| a.path.to_lowercase());

    let mut flagged_parents: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for d in dirs {
        let Some(place) = place_of(&d.path) else {
            continue;
        };
        let lower = d.path.to_lowercase();
        // A flagged vendor folder already covers everything inside it.
        if flagged_parents.iter().any(|p| lower.starts_with(p.as_str()) && lower.as_bytes().get(p.len()) == Some(&b'\\')) {
            continue;
        }
        if let Some(f) = judge(d, &place, &index, now_ms) {
            flagged_parents.push(lower.trim_end_matches('\\').to_string());
            out.push(f);
        }
    }
    out.sort_by_key(|f| std::cmp::Reverse(f.bytes));
    out
}

/// Where the folder sits: its own name, and the vendor folder above it when it is one level down.
struct Place {
    name: String,
    vendor: Option<String>,
}

fn place_of(path: &str) -> Option<Place> {
    let p = parts(path);
    let original: Vec<&str> = path.trim_start_matches(r"\\?\").split(['\\', '/']).filter(|c| !c.is_empty()).collect();
    if original.len() != p.len() {
        return None;
    }
    let root = p.iter().enumerate().find_map(|(i, c)| {
        let parent_is_appdata = i > 0 && p[i - 1] == "appdata";
        let is_container = (parent_is_appdata && matches!(c.as_str(), "roaming" | "local" | "locallow"))
            || (i == 1 && c == "programdata");
        is_container.then_some(i)
    })?;
    match p.len() - root - 1 {
        1 => Some(Place { name: original[root + 1].to_string(), vendor: None }),
        2 => Some(Place { name: original[root + 2].to_string(), vendor: Some(original[root + 1].to_string()) }),
        _ => None,
    }
}

fn is_built_in(name: &str) -> bool {
    let low = name.to_lowercase();
    BUILT_IN.contains(&low.as_str())
        || low.starts_with('.')
        || low.starts_with('{')
        || low.starts_with('$')
        || low.starts_with("microsoft")
        || low.starts_with("windows")
        || looks_like_id(&low)
}

/// GUIDs, hashes and numbers are not program names we can match.
fn looks_like_id(s: &str) -> bool {
    let hexish = s.chars().filter(|c| c.is_ascii_hexdigit() || *c == '-').count();
    (s.len() >= 16 && hexish == s.len()) || s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-')
}

struct Index {
    names: Vec<Name>,
}

impl Index {
    fn new(installed: &[InstalledApp]) -> Self {
        let mut names = Vec::new();
        for app in installed {
            names.push(normalize(&app.display_name));
            if let Some(p) = &app.publisher {
                names.push(normalize(p));
            }
            if let Some(loc) = &app.install_location {
                let lp = parts(loc);
                // "C:\Program Files\JetBrains\Toolbox" gives "Toolbox" and "JetBrains".
                for c in lp.iter().rev().take(2) {
                    if !matches!(c.as_str(), "program files" | "program files (x86)" | "programs" | "appdata" | "local" | "roaming")
                        && !c.ends_with(':')
                    {
                        names.push(normalize(c));
                    }
                }
            }
        }
        names.retain(|n| !n.is_empty());
        Self { names }
    }

    fn best(&self, name: &Name) -> f32 {
        self.names.iter().map(|n| similarity(name, n)).fold(0.0, f32::max)
    }
}

fn judge(d: &DirRecord, place: &Place, index: &Index, now: i64) -> Option<HeuristicFinding> {
    if d.size < MIN_SIZE || is_built_in(&place.name) {
        return None;
    }
    if let Some(v) = &place.vendor {
        if is_built_in(v) {
            return None;
        }
    }
    let own = normalize(&place.name);
    if own.is_empty() || own.joined.chars().count() < 3 {
        return None;
    }
    // Only the folder's own name counts. Mixing in the vendor would let "JetBrains\PyCharm" match
    // "JetBrains Toolbox" through the vendor word alone.
    let best = index.best(&own);
    if best >= INSTALLED_SCORE {
        return None;
    }
    let age = d.modified.map(|m| now - m);
    if age.is_some_and(|a| a < RECENT_MS) {
        return None;
    }

    // Weak partial matches, a recent change or a small size all lower the score.
    let mut conf = 0.35 + 0.25 * (1.0 - best / INSTALLED_SCORE);
    conf += match age {
        Some(a) if a >= 24 * MONTH_MS => 0.2,
        Some(a) if a >= 12 * MONTH_MS => 0.15,
        Some(a) if a >= 6 * MONTH_MS => 0.1,
        Some(_) => 0.0,
        None => -0.05,
    };
    if d.size >= 500 * 1024 * 1024 {
        conf += 0.05;
    }
    if place.vendor.is_some() {
        // The vendor folder is still matched, so this may be an old part of a current product.
        conf -= 0.1;
    }
    let conf = conf.clamp(0.1, 0.85);

    let label = match &place.vendor {
        Some(v) => format!("{v}\\{}", place.name),
        None => place.name.clone(),
    };
    let months = d.modified.map(|m| months_between(m, now));
    Some(HeuristicFinding {
        kind: HeuristicKind::Orphan,
        path: d.path.clone(),
        node_id: d.node_id,
        bytes: d.size,
        confidence: conf,
        safety: SafetyLevel::Careful,
        reason: reason(&label, months),
        details: FindingDetails::Orphan { app_name: label },
    })
}

fn reason(label: &str, months: Option<u32>) -> Bilingual {
    let mut fa = format!(
        "هیچ برنامهٔ نصب‌شده‌ای با نام پوشهٔ «{label}» پیدا نشد. احتمالاً برنامه‌ای که این پوشه را ساخته حذف شده و اطلاعاتش جا مانده است."
    );
    let mut en = format!(
        "No installed program matches the folder \"{label}\". The program that created it was probably uninstalled and left its data behind."
    );
    if let Some(m) = months.filter(|m| *m >= 1) {
        fa.push_str(&format!(" آخرین تغییر آن حدود {} ماه پیش بوده است.", fa_num(m)));
        en.push_str(&format!(" It was last changed about {m} months ago."));
    }
    fa.push_str(" پیش از پاک کردن نگاهی به آن بیندازید؛ بعضی برنامه‌های قابل‌حمل و بازی‌ها هم اطلاعاتشان را اینجا نگه می‌دارند.");
    en.push_str(" Take a look before deleting it. Some portable programs and games keep their data here too.");
    Bilingual::new(fa, en)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_760_000_000_000;

    fn dir(path: &str, size: u64, months_old: i64) -> DirRecord {
        DirRecord {
            node_id: Some(1),
            path: path.into(),
            name: crate::util::file_name(path).into(),
            size,
            modified: Some(NOW - months_old * MONTH_MS),
        }
    }

    fn apps() -> Vec<InstalledApp> {
        vec![
            InstalledApp::new("Google Chrome", Some("Google LLC"), Some(r"C:\Program Files\Google\Chrome\Application")),
            InstalledApp::new("Mozilla Firefox (x64 en-US)", Some("Mozilla"), Some(r"C:\Program Files\Mozilla Firefox")),
            InstalledApp::new("JetBrains Toolbox", Some("JetBrains s.r.o."), None),
            InstalledApp::new("Telegram Desktop", Some("Telegram FZ-LLC"), Some(r"C:\Users\me\AppData\Roaming\Telegram Desktop\")),
            InstalledApp::new("Visual Studio Code", Some("Microsoft Corporation"), Some(r"C:\Users\me\AppData\Local\Programs\Microsoft VS Code\")),
            InstalledApp::new("Discord", Some("Discord Inc."), Some(r"C:\Users\me\AppData\Local\Discord")),
            InstalledApp::new("VLC media player", Some("VideoLAN"), Some(r"C:\Program Files\VideoLAN\VLC")),
            InstalledApp::new("Steam", Some("Valve Corporation"), Some(r"C:\Program Files (x86)\Steam")),
            InstalledApp::new("OBS Studio", Some("OBS Project"), Some(r"C:\Program Files\obs-studio")),
        ]
    }

    const MB: u64 = 1024 * 1024;

    #[test]
    fn installed_apps_are_never_flagged() {
        let r = r"C:\Users\me\AppData\Roaming";
        let l = r"C:\Users\me\AppData\Local";
        let dirs = vec![
            dir(&format!(r"{l}\Google"), 900 * MB, 30),
            dir(&format!(r"{l}\Google\Chrome"), 900 * MB, 30),
            dir(&format!(r"{r}\Mozilla"), 300 * MB, 30),
            dir(&format!(r"{l}\Mozilla\Firefox"), 300 * MB, 30),
            dir(&format!(r"{l}\JetBrains"), 3000 * MB, 30),
            dir(&format!(r"{l}\JetBrains\Toolbox"), 3000 * MB, 30),
            dir(&format!(r"{r}\JetBrains"), 30 * MB, 30),
            dir(&format!(r"{r}\Telegram Desktop"), 2000 * MB, 30),
            dir(&format!(r"{r}\Code"), 500 * MB, 30),
            dir(&format!(r"{l}\Programs\Microsoft VS Code"), 500 * MB, 30),
            dir(&format!(r"{r}\discord"), 400 * MB, 30),
            dir(&format!(r"{r}\vlc"), 5 * MB, 30),
            dir(&format!(r"{l}\Steam"), 50 * MB, 30),
            dir(&format!(r"{r}\obs-studio"), 50 * MB, 30),
            dir(r"C:\ProgramData\Microsoft", 5000 * MB, 30),
            dir(r"C:\ProgramData\Package Cache", 5000 * MB, 30),
            dir(&format!(r"{l}\Packages"), 5000 * MB, 30),
            dir(&format!(r"{l}\Temp"), 5000 * MB, 30),
            dir(&format!(r"{l}\npm-cache"), 5000 * MB, 30),
            dir(&format!(r"{l}\NVIDIA"), 5000 * MB, 30),
            dir(&format!(r"{l}\D3DSCache"), 5000 * MB, 30),
            dir(&format!(r"{l}\{{2F3A1B2C-0000-1111-2222-333344445555}}"), 5000 * MB, 30),
        ];
        let found = find_orphans_at(&dirs, &apps(), NOW);
        assert!(found.is_empty(), "{:#?}", found.iter().map(|f| &f.path).collect::<Vec<_>>());
    }

    #[test]
    fn leftovers_are_flagged_as_careful() {
        let dirs = vec![
            dir(r"C:\Users\me\AppData\Roaming\OldGameStudio", 800 * MB, 30),
            dir(r"C:\Users\me\AppData\Local\OldGameStudio\Launcher", 100 * MB, 30),
            dir(r"C:\Users\me\AppData\Local\Kitchen Planner 3D", 60 * MB, 8),
            dir(r"C:\ProgramData\AcmeSync", 120 * MB, 3),
            dir(r"C:\Users\me\AppData\Local\JetBrains\PyCharm2021.1", 900 * MB, 30),
        ];
        let found = find_orphans_at(&dirs, &apps(), NOW);
        let paths: Vec<&str> = found.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&r"C:\Users\me\AppData\Roaming\OldGameStudio"));
        assert!(paths.contains(&r"C:\Users\me\AppData\Local\Kitchen Planner 3D"));
        assert!(paths.contains(&r"C:\ProgramData\AcmeSync"));
        assert!(paths.contains(&r"C:\Users\me\AppData\Local\JetBrains\PyCharm2021.1"));
        for f in &found {
            assert_eq!(f.safety, SafetyLevel::Careful);
            assert_ne!(f.safety, SafetyLevel::Safe);
            assert!(f.confidence > 0.0 && f.confidence < 1.0);
            assert!(f.reason.fa.contains('«') && f.reason.en.contains("No installed program"));
        }
        let old = found.iter().find(|f| f.path.ends_with("OldGameStudio")).unwrap();
        let recent = found.iter().find(|f| f.path.ends_with("AcmeSync")).unwrap();
        assert!(old.confidence > recent.confidence);
        let vendor_child = found.iter().find(|f| f.path.ends_with("PyCharm2021.1")).unwrap();
        assert!(vendor_child.confidence < old.confidence);
    }

    #[test]
    fn flagged_vendor_hides_its_children() {
        let dirs = vec![
            dir(r"C:\Users\me\AppData\Local\OldGameStudio", 800 * MB, 30),
            dir(r"C:\Users\me\AppData\Local\OldGameStudio\Launcher", 100 * MB, 30),
        ];
        let found = find_orphans_at(&dirs, &[], NOW);
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn skips_small_recent_and_odd_places() {
        let dirs = vec![
            dir(r"C:\Users\me\AppData\Roaming\TinyThing", 10 * 1024, 30),
            dir(r"C:\Users\me\AppData\Roaming\FreshThing", 100 * MB, 0),
            dir(r"C:\Users\me\Documents\OldGameStudio", 100 * MB, 30),
            dir(r"C:\Users\me\AppData\Local\A\B\C", 100 * MB, 30),
        ];
        assert!(find_orphans_at(&dirs, &[], NOW).is_empty());
    }
}
