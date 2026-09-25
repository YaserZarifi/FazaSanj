//! Files with exactly the same content: same size, then a hash of the first and last 64 KB,
//! then a full hash. Hardlinks are one file and are not reported.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rayon::prelude::*;

use fazasanj_model::{Bilingual, DuplicateFile, FindingDetails, HeuristicFinding, HeuristicKind, SafetyLevel};

use crate::util::{fa_num, has_part, is_system_location, parent, parts};
use crate::{win, FileRecord};

const EDGE: u64 = 64 * 1024;
const MAX_GROUPS: usize = 1000;
const PROGRESS_EVERY: u64 = 32;

/// Copies here may be needed where they are (an app or a project keeps its own copy).
const OWNED_PLACES: &[&str] = &[
    "appdata", "node_modules", ".git", "site-packages", "target", ".venv", "venv", "packages", "vendor", "lib",
    "bin", "obj", ".nuget", ".gradle", ".m2", ".cargo", "steamapps",
];
const NICE_PLACES: &[&str] = &["documents", "my documents", "pictures", "desktop", "music", "videos", "onedrive"];
const MESSY_PLACES: &[&str] = &["downloads", "temp", "tmp", "appdata", "cache", "caches", "$recycle.bin", "backup", "backups"];

type Hash = [u8; 32];

pub fn find_duplicates(
    files: Vec<FileRecord>,
    min_size: u64,
    cancel: &AtomicBool,
    progress: &(dyn Fn(u64, u64) + Sync),
) -> Vec<HeuristicFinding> {
    let min_size = min_size.max(1);
    let mut by_size: HashMap<u64, Vec<FileRecord>> = HashMap::new();
    for f in files {
        if f.size >= min_size && !is_system_location(&f.path) {
            by_size.entry(f.size).or_default().push(f);
        }
    }
    let candidates: Vec<FileRecord> = by_size.into_values().filter(|g| g.len() >= 2).flatten().collect();

    let done = AtomicU64::new(0);
    let total = AtomicU64::new(candidates.len() as u64);
    let tick = || {
        let d = done.fetch_add(1, Ordering::Relaxed) + 1;
        if d % PROGRESS_EVERY == 0 {
            progress(d, total.load(Ordering::Relaxed));
        }
    };

    // Step 1: partial hash and file id, in parallel.
    let partial: Vec<(FileRecord, (u32, u64), Hash)> = candidates
        .into_par_iter()
        .filter_map(|f| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let r = open_checked(&f).and_then(|(file, id)| partial_hash(file, f.size).map(|h| (f, id, h)));
            tick();
            r
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        return Vec::new();
    }

    let mut groups: HashMap<(u64, Hash), Vec<(FileRecord, (u32, u64))>> = HashMap::new();
    for (f, id, h) in partial {
        let g = groups.entry((f.size, h)).or_default();
        // A second path to the same file id is a hardlink, not a copy.
        if !g.iter().any(|(_, other)| *other == id) {
            g.push((f, id));
        }
    }
    let groups: Vec<Vec<FileRecord>> =
        groups.into_values().filter(|g| g.len() >= 2).map(|g| g.into_iter().map(|(f, _)| f).collect()).collect();

    // Step 2: a full hash, only where the edges did not already cover the whole file.
    let need_full: u64 = groups.iter().filter(|g| g[0].size > 2 * EDGE).map(|g| g.len() as u64).sum();
    total.fetch_add(need_full, Ordering::Relaxed);
    let mut confirmed: Vec<Vec<FileRecord>> = Vec::new();
    for g in groups {
        if g[0].size <= 2 * EDGE {
            confirmed.push(g);
            continue;
        }
        let hashed: Vec<(FileRecord, Hash)> = g
            .into_par_iter()
            .filter_map(|f| {
                if cancel.load(Ordering::Relaxed) {
                    return None;
                }
                let h = full_hash(&f);
                tick();
                h.map(|h| (f, h))
            })
            .collect();
        let mut by_hash: HashMap<Hash, Vec<FileRecord>> = HashMap::new();
        for (f, h) in hashed {
            by_hash.entry(h).or_default().push(f);
        }
        confirmed.extend(by_hash.into_values().filter(|g| g.len() >= 2));
    }
    if cancel.load(Ordering::Relaxed) {
        return Vec::new();
    }
    progress(done.load(Ordering::Relaxed), total.load(Ordering::Relaxed));

    let mut out: Vec<HeuristicFinding> = confirmed.into_iter().map(finding).collect();
    out.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    out.truncate(MAX_GROUPS);
    out
}

/// Opens the file and makes sure it is still the size the scan saw.
fn open_checked(f: &FileRecord) -> Option<(File, (u32, u64))> {
    let file = File::open(&f.path).ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.len() != f.size {
        return None;
    }
    let id = win::file_id(&file)?;
    Some((file, id))
}

fn partial_hash(mut file: File, size: u64) -> Option<Hash> {
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; EDGE as usize];
    let n = read_up_to(&mut file, &mut buf)?;
    h.update(&buf[..n]);
    if size > 2 * EDGE {
        file.seek(SeekFrom::End(-(EDGE as i64))).ok()?;
        let n = read_up_to(&mut file, &mut buf)?;
        h.update(&buf[..n]);
    } else if size > EDGE {
        let n = read_up_to(&mut file, &mut buf)?;
        h.update(&buf[..n]);
    }
    Some(*h.finalize().as_bytes())
}

fn read_up_to(r: &mut impl Read, buf: &mut [u8]) -> Option<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match r.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    Some(filled)
}

fn full_hash(f: &FileRecord) -> Option<Hash> {
    let file = File::open(&f.path).ok()?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1024 * 1024];
    let mut total = 0u64;
    loop {
        let n = read_up_to(&mut reader, &mut buf)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        h.update(&buf[..n]);
    }
    (total == f.size).then(|| *h.finalize().as_bytes())
}

fn place_score(path: &str) -> u8 {
    // The closest known folder wins: Downloads\Photos is messy, Temp\x\Documents is nice.
    for c in parts(path).iter().rev().skip(1) {
        if NICE_PLACES.contains(&c.as_str()) {
            return 2;
        }
        if MESSY_PLACES.contains(&c.as_str()) {
            return 0;
        }
    }
    1
}

/// Nicest place first, then the newest, then the shortest path.
fn pick_keeper(files: &[FileRecord]) -> usize {
    let mut best = 0;
    for (i, f) in files.iter().enumerate().skip(1) {
        let b = &files[best];
        let key = |x: &FileRecord| (place_score(&x.path), x.modified.unwrap_or(i64::MIN), std::cmp::Reverse(x.path.len()));
        if key(f) > key(b) {
            best = i;
        }
    }
    best
}

fn finding(mut group: Vec<FileRecord>) -> HeuristicFinding {
    group.sort_by(|a, b| a.path.cmp(&b.path));
    let keep = pick_keeper(&group);
    let size = group[0].size;
    let n = group.len();
    let owned = group.iter().any(|f| has_part(&f.path, OWNED_PLACES));
    let keeper = &group[keep];
    let keep_folder = parent(&keeper.path).to_string();

    let mut fa = format!(
        "{} فایل دقیقاً محتوای یکسانی دارند (کل محتوایشان مقایسه شد). اگر نسخهٔ داخل «{keep_folder}» را نگه دارید و بقیه را پاک کنید، این فضا آزاد می‌شود.",
        fa_num(n)
    );
    let mut en = format!(
        "{n} files have exactly the same content (their whole content was compared). Keeping the copy in \"{keep_folder}\" and removing the others frees this space."
    );
    if owned {
        fa.push_str(" بعضی از نسخه‌ها داخل پوشهٔ یک برنامه یا پروژه هستند و شاید همان‌جا لازم باشند.");
        en.push_str(" Some copies are inside a program or project folder and may be needed there.");
    }

    HeuristicFinding {
        kind: HeuristicKind::Duplicates,
        path: keeper.path.clone(),
        node_id: keeper.node_id,
        bytes: size * (n as u64 - 1),
        confidence: if owned { 0.55 } else { 0.9 },
        safety: if owned { SafetyLevel::Careful } else { SafetyLevel::ProbablySafe },
        reason: Bilingual::new(fa, en),
        details: FindingDetails::Duplicates {
            file_size: size,
            files: group.iter().map(|f| DuplicateFile { path: f.path.clone(), modified: f.modified }).collect(),
            keep_index: keep,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn rec(p: &Path, modified: i64) -> FileRecord {
        FileRecord {
            node_id: None,
            path: p.to_string_lossy().into_owned(),
            size: std::fs::metadata(p).unwrap().len(),
            modified: Some(modified),
            accessed: None,
        }
    }

    fn content(len: usize, seed: u8) -> Vec<u8> {
        (0..len).map(|i| (i as u8).wrapping_mul(31).wrapping_add(seed)).collect()
    }

    fn run(files: Vec<FileRecord>) -> Vec<HeuristicFinding> {
        find_duplicates(files, 1, &AtomicBool::new(false), &|_, _| {})
    }

    #[test]
    fn finds_identical_and_ignores_one_byte_difference() {
        let d = tempfile::tempdir().unwrap();
        let big = content(500_000, 1);
        let mut almost = big.clone();
        almost[250_000] ^= 1; // middle byte, so the partial hash can not see it
        let docs = d.path().join("Documents");
        let dl = d.path().join("Downloads");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::create_dir_all(&dl).unwrap();
        std::fs::write(docs.join("report.pdf"), &big).unwrap();
        std::fs::write(dl.join("report (1).pdf"), &big).unwrap();
        std::fs::write(dl.join("report (2).pdf"), &almost).unwrap();
        let small = content(1000, 9);
        std::fs::write(dl.join("a.txt"), &small).unwrap();
        std::fs::write(dl.join("b.txt"), &small).unwrap();
        let mut small2 = small.clone();
        small2[999] ^= 1;
        std::fs::write(dl.join("c.txt"), &small2).unwrap();

        let files = vec![
            rec(&docs.join("report.pdf"), 1),
            rec(&dl.join("report (1).pdf"), 5),
            rec(&dl.join("report (2).pdf"), 5),
            rec(&dl.join("a.txt"), 1),
            rec(&dl.join("b.txt"), 2),
            rec(&dl.join("c.txt"), 2),
        ];
        let found = run(files);
        assert_eq!(found.len(), 2, "{found:#?}");
        let pdf = &found[0];
        assert_eq!(pdf.bytes, 500_000);
        let FindingDetails::Duplicates { files, keep_index, file_size } = &pdf.details else { panic!() };
        assert_eq!(*file_size, 500_000);
        assert_eq!(files.len(), 2);
        // Documents beats Downloads even though the Downloads copy is newer.
        assert!(files[*keep_index].path.contains("Documents"));
        assert!(!files.iter().any(|f| f.path.contains("(2)")));
        let txt = &found[1];
        let FindingDetails::Duplicates { files, keep_index, .. } = &txt.details else { panic!() };
        assert_eq!(files.len(), 2);
        // Same folder: the newest one is kept.
        assert!(files[*keep_index].path.ends_with("b.txt"));
        for f in &found {
            assert_ne!(f.safety, SafetyLevel::Safe);
            assert!(f.confidence > 0.0 && f.confidence < 1.0);
        }
    }

    #[test]
    fn hardlinks_are_not_duplicates() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.bin");
        let b = d.path().join("b.bin");
        std::fs::write(&a, content(10_000, 3)).unwrap();
        std::fs::hard_link(&a, &b).unwrap();
        assert!(run(vec![rec(&a, 1), rec(&b, 1)]).is_empty());
    }

    #[test]
    fn skips_unreadable_changed_and_system_files() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.bin");
        let b = d.path().join("b.bin");
        std::fs::write(&a, content(10_000, 3)).unwrap();
        std::fs::write(&b, content(10_000, 3)).unwrap();
        let mut gone = rec(&a, 1);
        gone.path = d.path().join("missing.bin").to_string_lossy().into_owned();
        let mut wrong_size = rec(&b, 1);
        wrong_size.size = 20_000;
        assert!(run(vec![rec(&a, 1), gone]).is_empty());
        assert!(run(vec![rec(&a, 1), wrong_size]).is_empty());
        let sys = |p: &str| FileRecord { node_id: None, path: p.into(), size: 10_000, modified: None, accessed: None };
        assert!(run(vec![sys(r"C:\Windows\a.dll"), sys(r"C:\Program Files\b.dll")]).is_empty());
    }

    #[test]
    fn copies_in_project_folders_are_careful() {
        let d = tempfile::tempdir().unwrap();
        let nm = d.path().join("proj").join("node_modules");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(nm.join("x.js"), content(5000, 1)).unwrap();
        std::fs::write(d.path().join("x.js"), content(5000, 1)).unwrap();
        let found = run(vec![rec(&nm.join("x.js"), 1), rec(&d.path().join("x.js"), 1)]);
        assert_eq!(found[0].safety, SafetyLevel::Careful);
    }

    #[test]
    fn cancel_and_progress() {
        let d = tempfile::tempdir().unwrap();
        let mut files = Vec::new();
        for i in 0..100 {
            let p = d.path().join(format!("f{i}.bin"));
            std::fs::write(&p, content(300_000, (i % 2) as u8)).unwrap();
            files.push(rec(&p, i));
        }
        assert!(find_duplicates(files.clone(), 1, &AtomicBool::new(true), &|_, _| {}).is_empty());
        let last = std::sync::Mutex::new((0u64, 0u64));
        let found = find_duplicates(files, 1, &AtomicBool::new(false), &|d, t| *last.lock().unwrap() = (d, t));
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].bytes, 49 * 300_000);
        let (d, t) = *last.lock().unwrap();
        assert_eq!(d, t);
        assert_eq!(t, 200);
    }
}
