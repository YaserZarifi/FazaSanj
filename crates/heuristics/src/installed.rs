//! Installed programs from the registry Uninstall keys.

use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
use winreg::RegKey;

use crate::InstalledApp;

const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const PACKAGES: &str =
    r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages";

/// Every program with a display name in HKLM (64 and 32 bit views) and HKCU, plus Store (MSIX)
/// apps of the current user, without duplicates. Windows updates are left out.
pub fn installed_programs() -> Vec<InstalledApp> {
    let mut out = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for (root, flags) in [
        (&hklm, KEY_READ | KEY_WOW64_64KEY),
        (&hklm, KEY_READ | KEY_WOW64_32KEY),
        (&hkcu, KEY_READ),
    ] {
        if let Ok(key) = root.open_subkey_with_flags(UNINSTALL, flags) {
            read_uninstall_key(&key, flags, &mut out);
        }
    }
    if let Ok(key) = hkcu.open_subkey_with_flags(PACKAGES, KEY_READ) {
        out.extend(key.enum_keys().flatten().filter_map(|k| package_app(&k)));
    }
    out.sort_by_key(|a| a.display_name.to_lowercase());
    out.dedup_by(|a, b| {
        a.display_name.eq_ignore_ascii_case(&b.display_name) && a.install_location == b.install_location
    });
    out
}

fn read_uninstall_key(key: &RegKey, flags: u32, out: &mut Vec<InstalledApp>) {
    for name in key.enum_keys().flatten() {
        let Ok(sub) = key.open_subkey_with_flags(&name, flags) else {
            continue;
        };
        let Ok(display_name) = sub.get_value::<String, _>("DisplayName") else {
            continue;
        };
        let display_name = display_name.trim().to_string();
        if display_name.is_empty() {
            continue;
        }
        let system_component = sub.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1;
        let is_update = sub.get_value::<String, _>("ParentKeyName").is_ok()
            || sub.get_value::<String, _>("ReleaseType").is_ok_and(|t| t.contains("Update") || t == "Hotfix");
        if is_update {
            continue;
        }
        // System components still count as installed for orphan matching (a hidden runtime
        // may own an AppData folder), so they are kept but marked.
        let text = |v: &str| sub.get_value::<String, _>(v).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        out.push(InstalledApp {
            display_name,
            publisher: text("Publisher"),
            install_location: text("InstallLocation")
                .or_else(|| text("DisplayIcon").map(|i| icon_folder(&i)))
                .filter(|s| !s.is_empty()),
            system_component,
        });
    }
}

/// `TelegramMessengerLLP.TelegramDesktop_4.8.1.0_x64__t4vj0pshhgkwm` gives
/// "TelegramMessengerLLP TelegramDesktop". Store apps are not in the Uninstall keys.
fn package_app(full_name: &str) -> Option<InstalledApp> {
    let name = full_name.split('_').next()?.trim();
    if name.is_empty() {
        return None;
    }
    let mut app = InstalledApp::new(&name.replace('.', " "), None, None);
    app.system_component = true;
    Some(app)
}

/// `"C:\Apps\Foo\foo.exe",0` gives `C:\Apps\Foo`.
fn icon_folder(icon: &str) -> String {
    let s = icon.trim().trim_matches('"');
    let s = match s.rfind(',') {
        Some(i) if s[i + 1..].trim().parse::<i32>().is_ok() => &s[..i],
        _ => s,
    };
    let s = s.trim().trim_matches('"');
    crate::util::parent(s).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_package_names() {
        let a = package_app("TelegramMessengerLLP.TelegramDesktop_4.8.1.0_x64__t4vj0pshhgkwm").unwrap();
        assert_eq!(a.display_name, "TelegramMessengerLLP TelegramDesktop");
    }

    #[test]
    fn icon_paths() {
        assert_eq!(icon_folder(r#""C:\Apps\Foo\foo.exe",0"#), r"C:\Apps\Foo");
        assert_eq!(icon_folder(r"C:\Apps\Foo\foo.exe"), r"C:\Apps\Foo");
    }

    #[test]
    fn reads_this_machine() {
        // Any Windows install has at least a few entries; mostly checks nothing panics.
        let apps = installed_programs();
        assert!(apps.iter().all(|a| !a.display_name.is_empty()));
    }
}
