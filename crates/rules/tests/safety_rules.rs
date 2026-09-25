//! Rules must never offer an easy delete for anything the hard block list protects.

mod common;

use std::path::Path;

use common::*;
use fazasanj_model::{CleanupMethod, SafetyLevel};
use fazasanj_safety::SafetyContext;

fn ctx_for(drive: &str) -> SafetyContext {
    SafetyContext::new(
        vec![
            format!("{drive}\\Windows\\System32"),
            format!("{drive}\\Windows\\SysWOW64"),
            format!("{drive}\\Windows\\WinSxS"),
            format!("{drive}\\Windows\\servicing"),
            format!("{drive}\\Windows\\Boot"),
            format!("{drive}\\Windows\\Installer"),
            format!("{drive}\\Program Files"),
            format!("{drive}\\Program Files (x86)"),
            format!("{drive}\\Recovery"),
            "D:\\Apps\\Fazasanj".into(),
        ],
        vec![format!("{drive}\\Windows"), format!("{drive}\\ProgramData"), format!("{drive}\\Users")],
        Some(format!("{drive}\\Users")),
    )
}

fn destructive(m: CleanupMethod) -> bool {
    matches!(m, CleanupMethod::Recycle | CleanupMethod::DeleteContents)
}

/// Paths from the safety crate tests plus the obvious system places, as (path, is_dir).
fn protected_samples(drive: &str) -> Vec<(String, bool)> {
    let dirs = [
        "",
        "Windows",
        "Windows\\System32",
        "Windows\\System32\\drivers",
        "Windows\\System32\\config",
        "Windows\\SysWOW64",
        "Windows\\WinSxS",
        "Windows\\WinSxS\\Temp",
        "Windows\\servicing",
        "Windows\\Boot",
        "Windows\\Installer",
        "Windows\\Installer\\$PatchCache$",
        "Windows\\Installer\\$PatchCache$\\Managed",
        "Program Files",
        "Program Files\\App",
        "Program Files (x86)",
        "Program Files (x86)\\App",
        "Program Files (x86)\\Steam\\steamapps\\shadercache",
        "Program Files (x86)\\Steam\\steamapps\\downloading",
        "Program Files (x86)\\Ubisoft\\Ubisoft Game Launcher\\cache",
        "Program Files\\WindowsApps",
        "ProgramData",
        "Users",
        "Users\\yaser",
        "Users\\Public",
        "Users\\Test",
        "Users\\Default",
        "Recovery",
        "Windows.old",
        "Windows.old\\Users",
        "Windows.old\\Windows\\System32",
        "$Windows.~BT",
        "$Windows.~WS",
        "System Volume Information",
        "$Recycle.Bin",
        "$Recycle.Bin\\S-1-5-21",
    ];
    let files = [
        "Windows\\System32\\drivers\\etc\\hosts",
        "Windows\\System32\\ntoskrnl.exe",
        "Windows\\System32\\config\\SYSTEM",
        "Windows\\SysWOW64\\x.dll",
        "Windows\\Installer\\1a2b3c.msi",
        "Program Files (x86)\\App\\a.exe",
        "Program Files\\Docker\\disk.vhdx",
        "Users\\yaser\\NTUSER.DAT",
        "Users\\yaser\\ntuser.dat.LOG1",
        "Users\\Test\\NTUSER.DAT",
        "Users\\yaser\\AppData\\Local\\Microsoft\\Windows\\UsrClass.dat",
        "hiberfil.sys",
        "pagefile.sys",
        "swapfile.sys",
    ];
    let mut out: Vec<(String, bool)> = dirs.iter().map(|d| (format!("{drive}\\{d}"), true)).collect();
    out.extend(files.iter().map(|f| (format!("{drive}\\{f}"), false)));
    out
}

#[test]
fn protected_paths_never_get_an_easy_delete() {
    let r = rules();
    for drive in ["C:", "T:", "D:"] {
        let ctx = ctx_for(drive);
        for (p, is_dir) in protected_samples(drive) {
            assert!(ctx.is_blocked(Path::new(&p)), "sample {p} should be blocked by the safety crate");
            let Some((i, at)) = governing(&r, &p, is_dir, Some(0)) else { continue };
            let rule = r.get(i).unwrap();
            assert!(
                rule.safety == SafetyLevel::DoNotTouch || !destructive(rule.method),
                "{p} (matched at {at}) gets {} with {:?}/{:?}",
                rule.id,
                rule.safety,
                rule.method
            );
        }
    }
}

#[test]
fn protected_system_items_are_recognized() {
    let r = rules();
    let must = [
        ("T:\\Windows\\System32", true, "windows-system-files"),
        ("T:\\Windows\\SysWOW64", true, "windows-system-files"),
        ("T:\\Windows\\Installer", true, "windows-installer"),
        ("T:\\Program Files", true, "program-files"),
        ("T:\\Program Files (x86)", true, "program-files"),
        ("T:\\Users\\Test", true, "user-profile-root"),
        ("T:\\Users\\Test\\NTUSER.DAT", false, "registry-hives"),
        ("T:\\pagefile.sys", false, "pagefile"),
        ("T:\\swapfile.sys", false, "swapfile"),
        ("T:\\hiberfil.sys", false, "hiberfil"),
    ];
    for (p, is_dir, want) in must {
        let got = r.match_path(p, is_dir, None, NOW).map(|i| id(&r, i));
        assert_eq!(got.as_deref(), Some(want), "{p}");
    }
}

#[test]
fn safe_rules_never_touch_blocked_paths() {
    let r = rules();
    for drive in ["C:", "T:"] {
        let ctx = ctx_for(drive);
        for (i, path, _, _) in example_paths(&r, drive) {
            let rule = r.get(i).unwrap();
            if !ctx.is_blocked(Path::new(&path)) {
                continue;
            }
            assert_ne!(rule.safety, SafetyLevel::Safe, "{} is safe but {path} is blocked", rule.id);
            assert!(
                !destructive(rule.method),
                "{} deletes but the safety crate would refuse {path}; use command/manual_only",
                rule.id
            );
        }
    }
}

#[test]
fn every_safe_rule_is_allowed_by_real_env_context() {
    let r = rules();
    let ctx = SafetyContext::from_env(None);
    let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    for (i, path, _, _) in example_paths(&r, &drive) {
        let rule = r.get(i).unwrap();
        if rule.safety == SafetyLevel::Safe {
            assert!(!ctx.is_blocked(Path::new(&path)), "{} is safe but blocked: {path}", rule.id);
        }
    }
}

#[test]
fn anywhere_rules_skip_installed_apps() {
    let r = rules();
    let places = [
        r"C:\Program Files\SomeApp\resources\app",
        r"C:\Program Files (x86)\Tool",
        r"C:\Users\sara\AppData\Local\Programs\Microsoft VS Code\resources\app",
        r"C:\Users\sara\AppData\Roaming\npm",
        r"C:\Users\sara\.vscode\extensions\ms-python",
        r"C:\Users\sara\scoop\apps\nodejs",
        r"C:\Windows\SystemApps\x",
        r"C:\ProgramData\Vendor",
    ];
    for base in places {
        for name in ["node_modules", "__pycache__", ".vs", "DerivedDataCache"] {
            let p = format!(r"{base}\{name}");
            if let Some(i) = r.match_path(&p, true, None, NOW) {
                let rule = r.get(i).unwrap();
                assert!(!destructive(rule.method), "{p} matched {}", rule.id);
            }
        }
    }
    let ok = r.match_path(r"D:\code\site\node_modules", true, None, NOW);
    assert_eq!(ok.map(|i| id(&r, i)).as_deref(), Some("node-modules"));
}
