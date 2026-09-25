//! Arena tests on a small hand built tree. No disk access.

use fazasanj_model::{Category, ChildSort, TypeGroupKind};

use crate::builder::TreeBuilder;
use crate::finalize::finalize;
use crate::node::{flags, NONE, NO_TIME};
use crate::tree::ScanTree;
use crate::visit::Visit;

const F: u16 = 0;
const D: u16 = flags::DIR;

/// C:\
///   Windows\System32\kernel.dll    1000 (file id 77)
///   Windows\WinSxS\amd64_x\kernel.dll 1000 (same file id 77)
///   Windows\big.log                300
///   Users\me\Videos\a.mp4          5000
///   Users\me\Videos\b.mp4          4000
///   Users\me\tiny1..3.txt          1 each
///   pagefile.sys                   8000
///   cloud.docx                     0, cloud only
fn sample() -> ScanTree {
    let mut b = TreeBuilder::new();
    let win = b.push(0, "Windows", 0, 10, D);
    let s32 = b.push(win, "System32", 0, 10, D);
    // WinSxS copy is pushed first on purpose, so the rank has to pick System32.
    let sxs = b.push(win, "WinSxS", 0, 10, D);
    let amd = b.push(sxs, "amd64_x", 0, 10, D);
    let k2 = b.push(amd, "kernel.dll", 1000, 50, F);
    let k1 = b.push(s32, "kernel.dll", 1000, 50, F);
    b.add_link_candidate(77, k2);
    b.add_link_candidate(77, k1);
    b.push(win, "big.log", 300, 70, F);
    let users = b.push(0, "Users", 0, 10, D);
    let me = b.push(users, "me", 0, 10, D);
    let vids = b.push(me, "Videos", 0, 10, D);
    b.push(vids, "a.mp4", 5000, 100, F);
    b.push(vids, "b.mp4", 4000, 90, F);
    for i in 1..=3 {
        b.push(me, &format!("tiny{i}.txt"), 1, 20, F);
    }
    b.push(0, "pagefile.sys", 8000, NO_TIME, F | flags::SYSTEM);
    b.push(0, "cloud.docx", 0, 5, F | flags::CLOUD_ONLY);
    // Orphan with a broken parent, must be dropped.
    b.push(NONE - 1, "orphan.bin", 123_456, 1, F);
    finalize(b, 0, r"C:\".to_string())
}

fn id(t: &ScanTree, p: &str) -> u32 {
    t.find_by_path(p).unwrap_or_else(|| panic!("missing {p}"))
}

#[test]
fn totals_and_hardlink_dedupe() {
    let t = sample();
    let s = t.summary();
    assert_eq!(s.total_bytes, 1000 + 300 + 9000 + 3 + 8000);
    assert_eq!(s.files, 10);
    assert_eq!(s.dirs, 7);
    assert_eq!(s.cloud_only, 1);
    assert_eq!(s.hardlink_dups, 1);
    let sxs = id(&t, r"C:\Windows\WinSxS");
    assert_eq!(t.node(sxs).map(|n| n.total_size), Some(0));
    let dup = id(&t, r"C:\Windows\WinSxS\amd64_x\kernel.dll");
    assert!(t.node_info(dup).is_some_and(|i| i.flags.hardlink_dup && i.size == 0));
    assert_eq!(t.node(id(&t, r"C:\Windows\System32")).map(|n| n.total_size), Some(1000));
    assert!(t.find_by_path(r"C:\orphan.bin").is_none());
    assert_eq!(t.len(), 18);
}

#[test]
fn children_sorted_and_paged() {
    let t = sample();
    let names: Vec<&str> = t.children(0).map(|c| t.name(c)).collect();
    assert_eq!(names, vec!["Users", "pagefile.sys", "Windows", "cloud.docx"]);

    let page = t.children_page(0, ChildSort::Size, 1, 2).expect("page");
    assert_eq!(page.total, 4);
    assert_eq!(page.items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(), vec!["pagefile.sys", "Windows"]);

    let page = t.children_page(0, ChildSort::Name, 0, 10).expect("page");
    let by_name: Vec<&str> = page.items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(by_name, vec!["cloud.docx", "pagefile.sys", "Users", "Windows"]);

    let page = t.children_page(0, ChildSort::Modified, 0, 1).expect("page");
    assert_eq!(page.items[0].name, "Users");
    assert!(t.children_page(9999, ChildSort::Size, 0, 1).is_none());
}

#[test]
fn paths_and_lookup() {
    let t = sample();
    let a = id(&t, r"c:\users\ME\videos\A.MP4");
    assert_eq!(t.path_of(a), r"C:\Users\me\Videos\a.mp4");
    assert_eq!(t.find_by_path(r"C:\"), Some(0));
    assert_eq!(t.find_by_path("C:/Users/me/"), Some(id(&t, r"C:\Users\me")));
    assert_eq!(t.find_by_path(r"D:\Users"), None);
    let info = t.node_info(a).expect("info");
    assert_eq!(info.parent, Some(id(&t, r"C:\Users\me\Videos")));
    assert_eq!(info.modified, Some(100));
    let vids = t.node_info(id(&t, r"C:\Users\me\Videos")).expect("info");
    assert_eq!((vids.file_count, vids.child_count, vids.modified), (2, 2, Some(100)));
}

#[test]
fn treemap_groups_small_items() {
    let t = sample();
    let me = id(&t, r"C:\Users\me");
    let tm = t.treemap(me, 2, 2).expect("treemap");
    assert_eq!(tm.children.len(), 2);
    assert_eq!(tm.children[0].name, "Videos");
    assert_eq!(tm.children[0].children.len(), 2);
    let other = &tm.children[1];
    assert!(other.is_other);
    assert_eq!((other.id, other.size, other.file_count), (me, 3, 3));

    // Depth 0 has no children at all.
    assert!(t.treemap(me, 0, 10).expect("tm").children.is_empty());
    // Plenty of room: only the tiny ones are grouped, zero sized items are skipped.
    let root = t.treemap(0, 1, 50).expect("tm");
    assert!(root.children.iter().all(|c| c.size > 0));
}

#[test]
fn largest_files_uses_top_n() {
    let t = sample();
    let top = t.largest_files(3);
    let names: Vec<&str> = top.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["pagefile.sys", "a.mp4", "b.mp4"]);
    assert_eq!(top[1].path, r"C:\Users\me\Videos\a.mp4");
    assert!(t.largest_files(0).is_empty());
    // The deduped kernel.dll shows up once.
    let all = t.largest_files(100);
    assert_eq!(all.iter().filter(|f| f.name == "kernel.dll").count(), 1);
}

#[test]
fn by_type_groups() {
    let t = sample();
    let groups = t.by_type();
    let get = |k: TypeGroupKind| groups.iter().find(|g| g.group == k);
    let video = get(TypeGroupKind::Video).expect("video");
    assert_eq!((video.bytes, video.files), (9000, 2));
    assert_eq!(video.top_extensions[0].ext, "mp4");
    assert_eq!(get(TypeGroupKind::System).map(|g| g.bytes), Some(8000 + 300));
    assert_eq!(groups[0].group, TypeGroupKind::Video);
}

#[test]
fn subtree_stats() {
    let t = sample();
    let me = id(&t, r"C:\Users\me");
    let stats = t.subtree_extension_stats(me, 1);
    assert_eq!(stats.len(), 1);
    assert_eq!((stats[0].ext.as_str(), stats[0].bytes), ("mp4", 9000));
    assert_eq!(t.subtree_date_range(me), (Some(20), Some(100)));
    assert_eq!(t.subtree_date_range(id(&t, r"C:\pagefile.sys")), (None, None));
}

#[test]
fn tags_and_category_totals() {
    let mut t = sample();
    let win = id(&t, r"C:\Windows");
    let s32 = id(&t, r"C:\Windows\System32");
    let vids = id(&t, r"C:\Users\me\Videos");
    assert!(t.set_tag(win, Category::System, Some(1)));
    t.set_tags([(s32, Category::System, Some(2)), (vids, Category::Media, None)]);
    assert!(!t.set_tag(9999, Category::Dev, None));
    assert_eq!(t.tag_of(s32), Some((Category::System, Some(2))));
    assert_eq!(t.tagged_roots(), vec![vids, win]);

    let totals = t.category_totals();
    let get = |c: Category| totals.iter().find(|x| x.0 == c).map(|x| x.1);
    assert_eq!(get(Category::Media), Some(9000));
    // Windows by tag plus pagefile.sys by guess.
    assert_eq!(get(Category::System), Some(1300 + 8000));
    // Users is guessed as user files.
    assert_eq!(get(Category::UserFiles), Some(3));
    assert_eq!(totals.iter().map(|x| x.1).sum::<u64>(), t.summary().total_bytes);

    assert_eq!(t.effective_category(id(&t, r"C:\Windows\big.log")), Category::System);
    assert_eq!(t.effective_category(id(&t, r"C:\Users\me\tiny1.txt")), Category::UserFiles);
    t.clear_tags();
    assert!(t.tagged_roots().is_empty());
    assert_eq!(t.effective_category(id(&t, r"C:\Windows\big.log")), Category::System);
}

#[test]
fn visit_builds_paths_and_skips() {
    let t = sample();
    let mut seen = Vec::new();
    t.visit_dirs_and_files(&mut |e| {
        seen.push((e.path.to_string(), e.path_lower.to_string(), e.depth));
        if e.name == "Windows" {
            Visit::SkipChildren
        } else {
            Visit::Continue
        }
    });
    assert_eq!(seen[0], (r"C:\".to_string(), r"c:\".to_string(), 0));
    assert!(seen.iter().any(|s| s.0 == r"C:\Users\me\Videos\b.mp4" && s.1 == r"c:\users\me\videos\b.mp4" && s.2 == 4));
    assert!(!seen.iter().any(|s| s.0.starts_with(r"C:\Windows\")));
    assert!(seen.iter().any(|s| s.0 == r"C:\pagefile.sys" && s.2 == 1));

    let mut count = 0;
    t.visit_dirs_and_files(&mut |_| {
        count += 1;
        if count == 3 {
            Visit::Stop
        } else {
            Visit::Continue
        }
    });
    assert_eq!(count, 3);
}

#[test]
fn files_and_folder_sizes() {
    let t = sample();
    let files: Vec<_> = t.files().collect();
    assert_eq!(files.len(), 10);
    assert!(files.iter().any(|f| f.path == r"C:\Windows\System32\kernel.dll" && f.size == 1000 && !f.hardlink_dup));
    assert!(files.iter().any(|f| f.path == r"C:\Windows\WinSxS\amd64_x\kernel.dll" && f.hardlink_dup));
    for f in &files {
        assert_eq!(t.find_by_path(&f.path), Some(f.id));
    }

    let sizes = t.folder_sizes(1);
    let paths: Vec<&str> = sizes.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(paths, vec![r"C:\", r"C:\Users", r"C:\Windows"]);
    assert_eq!(sizes[1], (r"C:\Users".to_string(), 9003, 5));
}

#[test]
fn folder_root_paths() {
    let mut b = TreeBuilder::new();
    let sub = b.push(0, "src", 0, 1, D);
    b.push(sub, "main.rs", 4096, 1, F);
    let t = finalize(b, 0, r"D:\Projects\app".to_string());
    let f = id(&t, r"D:\projects\APP\src\main.rs");
    assert_eq!(t.path_of(f), r"D:\Projects\app\src\main.rs");
    assert_eq!(t.name(0), r"D:\Projects\app");
    assert!(t.find_by_path(r"D:\Projects\application").is_none());
    let files: Vec<_> = t.files().map(|f| f.path).collect();
    assert_eq!(files, vec![r"D:\Projects\app\src\main.rs".to_string()]);
}

#[test]
fn empty_root() {
    let t = finalize(TreeBuilder::new(), 0, r"E:\".to_string());
    assert_eq!(t.len(), 1);
    assert_eq!(t.summary().total_bytes, 0);
    assert!(t.children(0).next().is_none());
    assert!(t.treemap(0, 3, 10).is_some_and(|m| m.children.is_empty()));
}

#[test]
fn node_layout_is_compact() {
    assert!(std::mem::size_of::<crate::node::Node>() <= 56);
}
