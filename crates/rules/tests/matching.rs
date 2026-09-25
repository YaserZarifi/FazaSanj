mod common;

use common::*;

/// (path relative to a profile-style drive root, is_dir, expected rule id). `{u}` is replaced by
/// a user name so the same list runs for a real looking profile and the junk generator layout.
const CASES: &[(&str, bool, &str)] = &[
    (r"Users\{u}\AppData\Local\Temp", true, "user-temp"),
    (r"Windows\Temp", true, "windows-temp"),
    (r"hiberfil.sys", false, "hiberfil"),
    (r"pagefile.sys", false, "pagefile"),
    (r"Windows\WinSxS", true, "winsxs"),
    (r"Windows.old", true, "windows-old"),
    (r"$WINDOWS.~BT", true, "windows-upgrade-leftovers"),
    (r"Windows\SoftwareDistribution\Download", true, "windows-update-downloads"),
    (r"ProgramData\Microsoft\Windows\WER", true, "wer-system"),
    (r"Windows\MEMORY.DMP", false, "memory-dump"),
    (r"System Volume Information", true, "system-restore"),
    (r"$Recycle.Bin", true, "recycle-bin"),
    (r"Users\{u}\AppData\Local\Microsoft\Windows\Explorer\thumbcache_1280.db", false, "thumbnail-cache"),
    (r"Windows\Installer\$PatchCache$", true, "installer-patch-cache"),
    (r"Users\{u}\AppData\Local\Google\Chrome\User Data\Default\Cache", true, "chrome-cache"),
    (r"Users\{u}\AppData\Local\Google\Chrome\User Data\Profile 2\Code Cache", true, "chrome-cache"),
    (r"Users\{u}\AppData\Local\Google\Chrome\User Data", true, "browser-profile-data"),
    (r"Users\{u}\AppData\Local\Microsoft\Edge\User Data\Default\Service Worker\CacheStorage", true, "edge-cache"),
    (r"Users\{u}\AppData\Local\Mozilla\Firefox\Profiles\abcd1234.default-release\cache2", true, "firefox-cache"),
    (r"Users\{u}\AppData\Roaming\Opera Software\Opera GX Stable\Cache", true, "opera-gx-cache"),
    (r"Users\{u}\AppData\Roaming\Telegram Desktop\tdata\user_data", true, "telegram-cache"),
    (r"Users\{u}\AppData\Roaming\discord\Cache", true, "discord-cache"),
    (r"Program Files (x86)\Steam\steamapps\shadercache", true, "steam-cache-in-program-files"),
    (r"SteamLibrary\steamapps\shadercache", true, "steam-shader-cache"),
    (r"Users\{u}\AppData\Local\NVIDIA\DXCache", true, "nvidia-shader-cache"),
    (r"Users\{u}\AppData\Local\D3DSCache", true, "directx-shader-cache"),
    (r"AMD", true, "amd-driver-files"),
    (r"Users\{u}\AppData\Local\npm-cache", true, "npm-cache"),
    (r"Users\{u}\.nuget\packages", true, "nuget-packages"),
    (r"Users\{u}\.gradle\caches", true, "gradle-caches"),
    (r"Users\{u}\.cargo\registry", true, "cargo-registry"),
    (r"Users\{u}\source\repos\web\node_modules", true, "node-modules"),
    (r"Users\{u}\source\repos\tool\src\__pycache__", true, "python-pycache"),
    (r"Users\{u}\AppData\Local\Docker\wsl\disk\docker_data.vhdx", false, "docker-desktop-disk"),
    (
        r"Users\{u}\AppData\Local\Packages\CanonicalGroupLimited.Ubuntu22.04LTS_79rhkp1fndgsc\LocalState\ext4.vhdx",
        false,
        "wsl-disk",
    ),
    (r"VMs\lab.vhdx", false, "virtual-disk-file"),
    (r"Users\{u}\VirtualBox VMs", true, "virtualbox-vms"),
    (r"Users\{u}\AppData\Roaming\Apple Computer\MobileSync\Backup", true, "iphone-backups"),
    (r"Users\{u}\Apple\MobileSync\Backup", true, "iphone-backups"),
    (r"Users\{u}\Videos\Captures", true, "game-captures"),
    (r"Users\{u}\NTUSER.DAT", false, "registry-hives"),
    (r"Users\{u}", true, "user-profile-root"),
    (r"Windows\System32", true, "windows-system-files"),
    (r"Program Files", true, "program-files"),
];

fn check(drive: &str, user: &str) {
    let r = rules();
    for (rel, is_dir, want) in CASES {
        let p = format!(r"{drive}\{}", rel.replace("{u}", user));
        let got = r.match_path(&p, *is_dir, Some(0), NOW).map(|i| id(&r, i));
        assert_eq!(got.as_deref(), Some(*want), "{p}");
    }
}

#[test]
fn real_profile_paths() {
    check("C:", "Yaser");
}

#[test]
fn junk_generator_layout() {
    check("T:", "Test");
}

#[test]
fn case_and_prefix_variants() {
    let r = rules();
    let a = r.match_path(r"\\?\c:\users\x\appdata\local\temp", true, None, NOW);
    let b = r.match_path(r"C:/Users/x/AppData/Local/Temp/", true, None, NOW);
    assert_eq!(a, r.by_id("user-temp"));
    assert_eq!(b, r.by_id("user-temp"));
}

#[test]
fn installers_need_to_be_old() {
    let r = rules();
    let p = r"C:\Users\a\Downloads\setup.exe";
    let want = r.by_id("old-installers");
    assert_eq!(r.match_path(p, false, Some(NOW - 120 * DAY_MS), NOW), want);
    assert_eq!(r.match_path(p, false, Some(NOW - 10 * DAY_MS), NOW), None);
    assert_eq!(r.match_path(p, false, None, NOW), None);
    assert_eq!(r.match_path(r"C:\Users\a\Downloads\photo.jpg", false, Some(0), NOW), None);
}

#[test]
fn profile_data_is_not_cache() {
    let r = rules();
    for name in ["History", "Cookies", "Login Data", "Bookmarks", "Extensions"] {
        let p = format!(r"C:\Users\a\AppData\Local\Google\Chrome\User Data\Default\{name}");
        assert_eq!(r.match_path(&p, name == "Extensions", None, NOW), None, "{p}");
    }
}

#[test]
fn unrelated_paths_do_not_match() {
    let r = rules();
    for p in [r"C:\Users\a\Documents\report.docx", r"D:\Photos\2020", r"C:\Users\a\AppData\Local\SomeApp\Cache"] {
        assert_eq!(r.match_path(p, !p.ends_with("docx"), None, NOW), None, "{p}");
    }
}

#[test]
fn commands_fill_in_the_drive() {
    let r = rules();
    let i = r.by_id("windows-old").unwrap();
    let rule = r.get(i).unwrap();
    assert_eq!(rule.command_line(r"D:\Windows.old").as_deref(), Some("cleanmgr /d D:"));
    let t = r.get(r.by_id("ea-app-cache").unwrap()).unwrap();
    assert!(t.resolve_open_target(r"C:\Users\a\AppData\Local\x").is_some());
}
