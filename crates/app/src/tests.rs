//! Tests on a small hand built tree. No disk access.

use fazasanj_model::{
    Bilingual, CleanupMethod, CleanupTarget, Explanation, ExplanationSource, SafetyLevel, ScanMode, ScanSummary,
};
use fazasanj_rules::RuleSet;
use fazasanj_scan::{finalize, flags, ScanTree, TreeBuilder};

use crate::cleanup::trusted_targets;
use crate::heuristics::{appdata_dirs, project_dirs};
use crate::scans::ScanEntry;
use crate::{build_story, explanation_for, tag_tree};

const D: u16 = flags::DIR;
const MB: u64 = 1024 * 1024;

/// C:\
///   Users\sam\AppData\Local\Temp\a.tmp          300 MB
///   Users\sam\AppData\Local\Temp\sub\b.tmp      100 MB
///   Users\sam\AppData\Roaming\OldApp\data.bin    50 MB
///   Users\sam\code\site\package.json              1 KB
///   Users\sam\code\site\node_modules\x\y.js      20 MB
///   Users\sam\Videos\movie.mp4                  900 MB
fn sample() -> ScanTree {
    let mut b = TreeBuilder::new();
    let users = b.push(0, "Users", 0, 10, D);
    let sam = b.push(users, "sam", 0, 10, D);
    let appdata = b.push(sam, "AppData", 0, 10, D);
    let local = b.push(appdata, "Local", 0, 10, D);
    let temp = b.push(local, "Temp", 0, 10, D);
    b.push(temp, "a.tmp", 300 * MB, 10, 0);
    let sub = b.push(temp, "sub", 0, 10, D);
    b.push(sub, "b.tmp", 100 * MB, 10, 0);
    let roaming = b.push(appdata, "Roaming", 0, 10, D);
    let old = b.push(roaming, "OldApp", 0, 10, D);
    b.push(old, "data.bin", 50 * MB, 10, 0);
    let code = b.push(sam, "code", 0, 10, D);
    let site = b.push(code, "site", 0, 10, D);
    b.push(site, "package.json", 1024, 10, 0);
    let nm = b.push(site, "node_modules", 0, 10, D);
    let x = b.push(nm, "x", 0, 10, D);
    b.push(x, "y.js", 20 * MB, 10, 0);
    let vids = b.push(sam, "Videos", 0, 10, D);
    b.push(vids, "movie.mp4", 900 * MB, 10, 0);
    finalize(b, 0, r"C:\".to_string())
}

fn entry(tree: ScanTree) -> ScanEntry {
    let s = tree.summary();
    let summary = ScanSummary {
        scan_id: 1,
        root_path: tree.root_path().to_string(),
        root_node: 0,
        total_bytes: s.total_bytes,
        files: s.files,
        dirs: s.dirs,
        access_denied: 0,
        cloud_only: 0,
        duration_ms: 1,
        scanner: ScanMode::Normal,
        fallback_reason: None,
        drive_total: 0,
        drive_free: 0,
        finished_at: 0,
    };
    ScanEntry { tree, summary, access_denied: Vec::new(), snapshot_id: None }
}

fn rules() -> RuleSet {
    RuleSet::load_embedded().unwrap()
}

#[test]
fn temp_folder_is_tagged_and_not_walked_into() {
    let rules = rules();
    let mut tree = sample();
    tag_tree(&mut tree, &rules, 0);
    let temp = tree.find_by_path(r"C:\Users\sam\AppData\Local\Temp").unwrap();
    let e = explanation_for(&tree, &rules, temp).unwrap();
    assert_eq!(e.rule_id, "user-temp");
    let inner = tree.find_by_path(r"C:\Users\sam\AppData\Local\Temp\sub").unwrap();
    assert!(explanation_for(&tree, &rules, inner).is_none());
}

#[test]
fn story_counts_safe_temp_once() {
    let rules = rules();
    let mut tree = sample();
    tag_tree(&mut tree, &rules, 0);
    let story = build_story(&entry(tree), &rules);
    let temp = story.safe_items.iter().find(|r| r.explanation.rule_id == "user-temp").unwrap();
    assert_eq!(temp.bytes, 400 * MB);
    assert!(story.safe_bytes >= 400 * MB);
    assert!(story.safe_items.iter().all(|r| r.explanation.safety == SafetyLevel::Safe));
    assert!(!story.buckets.is_empty());
    let sorted = story.reasons.windows(2).all(|w| w[0].bytes >= w[1].bytes);
    assert!(sorted);
}

#[test]
fn appdata_dirs_are_one_or_two_levels_down() {
    let tree = sample();
    let dirs = appdata_dirs(&tree);
    let paths: Vec<&str> = dirs.iter().map(|d| d.path.as_str()).collect();
    assert!(paths.contains(&r"C:\Users\sam\AppData\Roaming\OldApp"));
    assert!(paths.contains(&r"C:\Users\sam\AppData\Local\Temp"));
    assert!(paths.contains(&r"C:\Users\sam\AppData\Local\Temp\sub"));
    assert!(!paths.iter().any(|p| p.ends_with("Local")));
}

#[test]
fn project_dirs_found_by_marker() {
    let tree = sample();
    let dirs = project_dirs(&tree);
    assert_eq!(dirs.len(), 1);
    assert_eq!(dirs[0].path, r"C:\Users\sam\code\site");
    assert_eq!(dirs[0].size, 20 * MB + 1024);
}

fn explanation(rule_id: &str, source: ExplanationSource, safety: SafetyLevel) -> Explanation {
    Explanation {
        rule_id: rule_id.into(),
        source,
        title: Bilingual::new("x", "x"),
        why_big: Bilingual::default(),
        if_deleted: Bilingual::default(),
        safety,
        method: CleanupMethod::Recycle,
        needs_admin: false,
        instructions: None,
        confidence: None,
    }
}

#[test]
fn forged_explanations_are_replaced_or_dropped() {
    let rules = rules();
    let targets = vec![
        CleanupTarget {
            path: r"C:\Windows\MEMORY.DMP".into(),
            bytes: 1,
            explanation: explanation("memory-dump", ExplanationSource::KnowledgeBase, SafetyLevel::Safe),
        },
        CleanupTarget {
            path: r"C:\x".into(),
            bytes: 1,
            explanation: explanation("made-up", ExplanationSource::KnowledgeBase, SafetyLevel::Safe),
        },
        CleanupTarget {
            path: r"C:\dup.bin".into(),
            bytes: 1,
            explanation: explanation("duplicates", ExplanationSource::Heuristic, SafetyLevel::ProbablySafe),
        },
    ];
    let out = trusted_targets(targets, &rules);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].explanation.safety, SafetyLevel::ProbablySafe);
    assert_eq!(out[1].explanation.source, ExplanationSource::Heuristic);
}
