//! Small helpers shared by the heuristics: path parts, time math and Persian digits.

use std::time::{SystemTime, UNIX_EPOCH};

/// Average month (365.2425 / 12 days) in milliseconds.
pub(crate) const MONTH_MS: i64 = 2_629_746_000;

pub(crate) fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub(crate) fn months_between(from_ms: i64, to_ms: i64) -> u32 {
    ((to_ms - from_ms).max(0) / MONTH_MS).min(u32::MAX as i64) as u32
}

/// Lowercase path parts without the `\\?\` prefix. The drive (`c:`) is the first part.
pub(crate) fn parts(path: &str) -> Vec<String> {
    let p = path.strip_prefix(r"\\?\").unwrap_or(path);
    p.split(['\\', '/']).filter(|c| !c.is_empty()).map(|c| c.to_lowercase()).collect()
}

pub(crate) fn parent(path: &str) -> &str {
    let t = path.trim_end_matches(['\\', '/']);
    match t.rfind(['\\', '/']) {
        Some(i) => &t[..i],
        None => "",
    }
}

#[cfg(test)]
pub(crate) fn file_name(path: &str) -> &str {
    let t = path.trim_end_matches(['\\', '/']);
    t.rsplit(['\\', '/']).next().unwrap_or(t)
}

/// Windows, Program Files and other places that are never "user files".
pub(crate) fn is_system_location(path: &str) -> bool {
    let p = parts(path);
    let top = p.get(1).map(String::as_str).unwrap_or("");
    if matches!(
        top,
        "windows"
            | "program files"
            | "program files (x86)"
            | "programdata"
            | "$recycle.bin"
            | "system volume information"
            | "windows.old"
            | "$windows.~bt"
            | "$windows.~ws"
            | "recovery"
            | "perflogs"
            | "msocache"
    ) {
        return true;
    }
    // Drive root files like pagefile.sys.
    p.len() == 2 && matches!(top, "pagefile.sys" | "hiberfil.sys" | "swapfile.sys" | "dumpstack.log")
}

pub(crate) fn has_part(path: &str, names: &[&str]) -> bool {
    parts(path).iter().any(|c| names.contains(&c.as_str()))
}

/// Persian digits for numbers inside Persian sentences.
pub(crate) fn fa_num(n: impl std::fmt::Display) -> String {
    n.to_string()
        .chars()
        .map(|c| match c.to_digit(10) {
            Some(d) => char::from_u32(0x06F0 + d).unwrap_or(c),
            None => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits() {
        assert_eq!(fa_num(1403), "۱۴۰۳");
    }

    #[test]
    fn system_locations() {
        assert!(is_system_location(r"C:\Windows\System32\a.dll"));
        assert!(is_system_location(r"\\?\C:\Program Files\x\y"));
        assert!(is_system_location(r"C:\pagefile.sys"));
        assert!(!is_system_location(r"C:\Users\me\Documents\windows\a.txt"));
        assert!(!is_system_location(r"D:\Movies\a.mkv"));
    }

    #[test]
    fn path_bits() {
        assert_eq!(parent(r"C:\a\b\c.txt"), r"C:\a\b");
        assert_eq!(file_name(r"C:\a\b\c.txt"), "c.txt");
        assert_eq!(months_between(0, MONTH_MS * 13 + 5), 13);
    }
}
