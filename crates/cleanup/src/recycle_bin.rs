//! Reads the Recycle Bin index files (`$I...`) so history can say what can still be restored.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// One `$I` record: where the item came from, its size and when it was deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecycledItem {
    pub original_path: String,
    pub size: u64,
    /// FILETIME (100 ns ticks since 1601).
    pub deleted_at: i64,
}

/// Parses a `$I` file. Version 1 (Vista to 8) has a fixed 260 char path, version 2 (Windows 10+)
/// stores the length first.
pub fn parse_index(bytes: &[u8]) -> Option<RecycledItem> {
    let rd_i64 = |at: usize| -> Option<i64> {
        bytes.get(at..at + 8).and_then(|b| b.try_into().ok()).map(i64::from_le_bytes)
    };
    let version = rd_i64(0)?;
    let size = rd_i64(8)?;
    let deleted_at = rd_i64(16)?;
    let units: Vec<u16> = match version {
        1 => {
            let raw = bytes.get(24..24 + 520)?;
            utf16_units(raw)
        }
        2 => {
            let len = u32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?) as usize;
            if len > 32_768 {
                return None;
            }
            let raw = bytes.get(28..28 + len * 2)?;
            utf16_units(raw)
        }
        _ => return None,
    };
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    let original_path = String::from_utf16(&units[..end]).ok()?;
    if original_path.is_empty() {
        return None;
    }
    Some(RecycledItem { original_path, size: size.max(0) as u64, deleted_at })
}

fn utf16_units(raw: &[u8]) -> Vec<u16> {
    (0..raw.len() / 2).map(|i| u16::from_le_bytes([raw[2 * i], raw[2 * i + 1]])).collect()
}

/// All items of the current user in the Recycle Bin of `drive` (like `C`).
pub fn list_drive(drive: char, sid: &str) -> Vec<RecycledItem> {
    let dir = PathBuf::from(format!("{drive}:\\$Recycle.Bin\\{sid}"));
    list_dir(&dir)
}

fn list_dir(dir: &Path) -> Vec<RecycledItem> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    rd.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("$I"))
        .filter_map(|e| std::fs::read(e.path()).ok())
        .filter_map(|b| parse_index(&b))
        .collect()
}

/// Which of `paths` are still in the Recycle Bin. A folder counts too when something that was
/// inside it is there (a `delete_contents` run recycles the children, not the folder).
pub fn recycle_bin_lookup(paths: &[String]) -> HashSet<String> {
    let Some(sid) = crate::win::current_user_sid() else {
        return HashSet::new();
    };
    let mut drives: Vec<char> = paths.iter().filter_map(|p| drive_letter(p)).collect();
    drives.sort();
    drives.dedup();
    let items: Vec<String> = drives
        .into_iter()
        .flat_map(|d| list_drive(d, &sid))
        .map(|i| fazasanj_safety::normalize(Path::new(&i.original_path)))
        .collect();
    match_paths(paths, &items)
}

fn match_paths(paths: &[String], recycled_normalized: &[String]) -> HashSet<String> {
    let set: HashSet<&str> = recycled_normalized.iter().map(String::as_str).collect();
    paths
        .iter()
        .filter(|p| {
            let n = fazasanj_safety::normalize(Path::new(p.as_str()));
            if n.is_empty() {
                return false;
            }
            set.contains(n.as_str())
                || recycled_normalized.iter().any(|r| r.len() > n.len() && r.starts_with(&n) && r.as_bytes()[n.len()] == b'\\')
        })
        .cloned()
        .collect()
}

fn drive_letter(p: &str) -> Option<char> {
    let p = p.strip_prefix("\\\\?\\").unwrap_or(p);
    let b = p.as_bytes();
    (b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic()).then(|| (b[0] as char).to_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(version: i64, size: i64, time: i64) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend(version.to_le_bytes());
        v.extend(size.to_le_bytes());
        v.extend(time.to_le_bytes());
        v
    }

    fn utf16(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    #[test]
    fn parses_v2() {
        let path = r"C:\Users\یاسر\Downloads\big file.iso";
        let mut b = header(2, 123_456, 133_000_000_000_000_000);
        let n = path.encode_utf16().count() as u32 + 1;
        b.extend(n.to_le_bytes());
        b.extend(utf16(path));
        b.extend([0, 0]);
        let item = parse_index(&b).unwrap();
        assert_eq!(item.original_path, path);
        assert_eq!(item.size, 123_456);
        assert_eq!(item.deleted_at, 133_000_000_000_000_000);
    }

    #[test]
    fn parses_v1() {
        let path = r"D:\old\stuff";
        let mut b = header(1, 42, 7);
        let mut p = utf16(path);
        p.resize(520, 0);
        b.extend(p);
        let item = parse_index(&b).unwrap();
        assert_eq!(item.original_path, path);
        assert_eq!(item.size, 42);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_index(&[]).is_none());
        assert!(parse_index(&header(3, 1, 1)).is_none());
        let mut b = header(2, 1, 1);
        b.extend(1000u32.to_le_bytes());
        b.extend(utf16("C:\\short"));
        assert!(parse_index(&b).is_none());
        let mut b = header(1, 1, 1);
        b.extend([0u8; 100]);
        assert!(parse_index(&b).is_none());
    }

    #[test]
    fn reads_index_files_from_a_folder() {
        let d = tempfile::tempdir().unwrap();
        let path = r"C:\x\y.txt";
        let mut b = header(2, 5, 1);
        b.extend((path.encode_utf16().count() as u32 + 1).to_le_bytes());
        b.extend(utf16(path));
        b.extend([0, 0]);
        std::fs::write(d.path().join("$IABC123.txt"), &b).unwrap();
        std::fs::write(d.path().join("$RABC123.txt"), b"data").unwrap();
        let items = list_dir(d.path());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].original_path, path);
    }

    #[test]
    fn matches_items_and_parents() {
        let recycled = vec![r"c:\cache\a.bin".to_string(), r"d:\old".to_string()];
        let paths = vec![
            r"C:\Cache".to_string(),
            r"C:\Cache\a.bin".to_string(),
            r"C:\Cach".to_string(),
            r"D:\old".to_string(),
            r"D:\other".to_string(),
        ];
        let got = match_paths(&paths, &recycled);
        assert!(got.contains(r"C:\Cache"));
        assert!(got.contains(r"C:\Cache\a.bin"));
        assert!(got.contains(r"D:\old"));
        assert_eq!(got.len(), 3);
    }
}
