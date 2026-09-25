//! Normal scanner: parallel directory walk, no admin needed.
//!
//! Each folder is one rayon task. A task lists its folder with batched
//! `GetFileInformationByHandleEx` calls (see `fazasanj_platform::read_dir`), works out sizes
//! outside any lock, then appends all entries to the shared builder in one short locked step
//! and spawns tasks for the subfolders.
//!
//! Hard links: the directory listing gives every file's 128 bit id for free, so every file with
//! bytes becomes a dedupe candidate and `finalize` sorts them once. That costs 16 bytes per file
//! for a moment and needs no extra handle per file, which is far cheaper than asking each file
//! for its link count. WinSxS and System32 share most files through hard links, this is what
//! keeps WinSxS from looking huge.

use std::cell::RefCell;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use fazasanj_platform::{
    compressed_size, filetime_to_unix_ms, is_name_surrogate, read_dir, strip_long_prefix, Attributes, DirBuffer,
    DirError,
};

use crate::builder::TreeBuilder;
use crate::finalize::finalize;
use crate::node::{flags, NO_TIME};
use crate::options::ScanError;
use crate::progress::Counters;
use crate::rootpath::ScanRoot;
use crate::size::{attribute_flags, file_size, size_source, SizeSource};
use crate::tree::ScanTree;

pub struct WalkOutput {
    pub tree: ScanTree,
    pub access_denied: Vec<String>,
}

struct Shared<'a> {
    builder: Mutex<TreeBuilder>,
    cancel: &'a AtomicBool,
    counters: &'a Counters,
    denied: Mutex<Vec<String>>,
    /// Lowercase display paths.
    excluded: &'a [String],
    root_error: Mutex<Option<DirError>>,
}

/// One entry kept between listing and the locked append.
struct Pending {
    name: std::ops::Range<usize>,
    size: u64,
    flags: u16,
    modified: i64,
    file_id: u128,
    descend: bool,
}

thread_local! {
    static DIR_BUF: RefCell<DirBuffer> = RefCell::new(DirBuffer::new());
}

fn threads() -> usize {
    let n = std::thread::available_parallelism().map_or(4, |n| n.get());
    (n * 2).clamp(4, 32)
}

pub fn walk(
    root: &ScanRoot,
    excluded: &[String],
    cancel: &AtomicBool,
    counters: &Counters,
) -> Result<WalkOutput, ScanError> {
    let mut builder = TreeBuilder::with_capacity(1 << 16);
    if let Ok(meta) = std::fs::metadata(&root.verbatim) {
        if let Ok(t) = meta.modified() {
            if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
                builder.nodes[0].modified = d.as_millis() as i64;
            }
        }
    }
    let shared = Shared {
        builder: Mutex::new(builder),
        cancel,
        counters,
        denied: Mutex::new(Vec::new()),
        excluded,
        root_error: Mutex::new(None),
    };

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads())
        .thread_name(|i| format!("scan-{i}"))
        .build()
        .map_err(|e| ScanError::Io(e.to_string()))?;
    let root_path = root.verbatim.clone();
    pool.scope(|s| scan_dir(s, &shared, 0, root_path, true));

    if cancel.load(Ordering::Relaxed) {
        return Err(ScanError::Cancelled);
    }
    if let Some(err) = shared.root_error.lock().ok().and_then(|mut e| e.take()) {
        return Err(match err {
            DirError::AccessDenied => ScanError::RootAccessDenied(root.display.clone()),
            DirError::NotFound => ScanError::RootNotFound(root.display.clone()),
            DirError::Other(code) => ScanError::Io(format!("{}: win32 error {code}", root.display)),
        });
    }
    let builder = shared.builder.into_inner().map_err(|_| ScanError::Io("builder lock poisoned".into()))?;
    let mut access_denied = shared.denied.into_inner().unwrap_or_default();
    access_denied.sort();
    let tree = finalize(builder, 0, root.display.clone());
    Ok(WalkOutput { tree, access_denied })
}

fn scan_dir<'s>(scope: &rayon::Scope<'s>, sh: &'s Shared<'s>, id: u32, path: String, is_root: bool) {
    if sh.cancel.load(Ordering::Relaxed) {
        return;
    }
    let mut names = String::new();
    let mut entries: Vec<Pending> = Vec::new();
    let mut raw: Vec<(std::ops::Range<usize>, u32, u32, u64, u128, i64)> = Vec::new();

    let listed = DIR_BUF.with(|b| {
        let mut buf = b.borrow_mut();
        read_dir(Path::new(&path), is_root, &mut buf, |e| {
            let start = names.len();
            names.push_str(e.name);
            raw.push((start..names.len(), e.attributes, e.reparse_tag, e.allocation_size, e.file_id, e.last_write));
            !sh.cancel.load(Ordering::Relaxed)
        })
    });

    if let Err(err) = listed {
        if is_root {
            if let Ok(mut r) = sh.root_error.lock() {
                *r = Some(err);
            }
        } else if err != DirError::NotFound {
            if let Ok(mut b) = sh.builder.lock() {
                b.nodes[id as usize].flags |= flags::ACCESS_DENIED;
            }
            if let Ok(mut d) = sh.denied.lock() {
                d.push(strip_long_prefix(&path));
            }
        }
        return;
    }
    if sh.cancel.load(Ordering::Relaxed) {
        return;
    }

    let (mut files, mut dirs, mut bytes) = (0u64, 0u64, 0u64);
    for (range, attrs, tag, alloc, file_id, last_write) in raw {
        let a = Attributes(attrs);
        let modified = filetime_to_unix_ms(last_write).unwrap_or(NO_TIME);
        let (size, f, descend) = if a.is_dir() {
            if is_excluded(sh.excluded, &path, &names[range.clone()]) {
                continue;
            }
            dirs += 1;
            let surrogate = a.is_reparse() && is_name_surrogate(tag);
            (0, attribute_flags(attrs), !surrogate)
        } else {
            files += 1;
            let precise = (size_source(attrs, tag) == SizeSource::Precise)
                .then(|| compressed_size(Path::new(&join(&path, &names[range.clone()]))).ok())
                .flatten();
            let (size, f) = file_size(attrs, tag, alloc, precise);
            bytes += size;
            (size, f, false)
        };
        entries.push(Pending { name: range, size, flags: f, modified, file_id, descend });
    }

    let base = {
        let Ok(mut b) = sh.builder.lock() else { return };
        let base = b.nodes.len() as u32;
        for (i, e) in entries.iter().enumerate() {
            let node = b.push(id, &names[e.name.clone()], e.size, e.modified, e.flags);
            if e.flags & flags::DIR == 0 && e.size > 0 && e.file_id != 0 {
                b.add_link_candidate(e.file_id, node);
            }
            debug_assert_eq!(node, base + i as u32);
        }
        base
    };

    sh.counters.add(files, dirs, bytes);
    sh.counters.set_current(strip_long_prefix(&path).as_str());

    for (i, e) in entries.iter().enumerate() {
        if e.descend {
            let child = join(&path, &names[e.name.clone()]);
            scope.spawn(move |s| scan_dir(s, sh, base + i as u32, child, false));
        }
    }
}

fn join(dir: &str, name: &str) -> String {
    let mut s = String::with_capacity(dir.len() + name.len() + 1);
    s.push_str(dir);
    if !s.ends_with('\\') {
        s.push('\\');
    }
    s.push_str(name);
    s
}

fn is_excluded(excluded: &[String], parent_verbatim: &str, name: &str) -> bool {
    if excluded.is_empty() {
        return false;
    }
    let full = join(&strip_long_prefix(parent_verbatim), name).to_lowercase();
    excluded.contains(&full)
}
