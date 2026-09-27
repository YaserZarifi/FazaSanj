//! Finished scans kept in memory, and everything derived from them.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard};

use fazasanj_ai::FolderMeta;
use fazasanj_model::{
    ApiError, CleanupMethod, NodeId, NodeInfo, Reason, SafetyLevel, ScanId, ScanSummary, Story, StoryBucket,
};
use fazasanj_rules::RuleSet;
use fazasanj_scan::{ScanTree, Visit};

use crate::state::lock;

pub struct ScanSession {
    pub root: String,
    pub cancel: Arc<AtomicBool>,
    pub tree: RwLock<Option<ScanTree>>,
    pub summary: Mutex<Option<ScanSummary>>,
    pub access_denied: Mutex<Vec<String>>,
    /// Knowledge base matches: node and rule index.
    pub matches: Mutex<Vec<(NodeId, u32)>>,
    pub snapshot_id: Mutex<Option<i64>>,
}

impl ScanSession {
    pub fn new(root: String) -> Self {
        Self {
            root,
            cancel: Arc::new(AtomicBool::new(false)),
            tree: RwLock::new(None),
            summary: Mutex::new(None),
            access_denied: Mutex::new(Vec::new()),
            matches: Mutex::new(Vec::new()),
            snapshot_id: Mutex::new(None),
        }
    }

    pub fn read(&self) -> Result<TreeGuard<'_>, ApiError> {
        let g = self.tree.read().unwrap_or_else(|e| e.into_inner());
        if g.is_none() {
            return Err(ApiError::new("scan_not_ready"));
        }
        Ok(TreeGuard(g))
    }
}

pub struct TreeGuard<'a>(RwLockReadGuard<'a, Option<ScanTree>>);

impl std::ops::Deref for TreeGuard<'_> {
    type Target = ScanTree;
    fn deref(&self) -> &ScanTree {
        // `read` checked it is Some, and nobody takes it out while a guard exists.
        match self.0.as_ref() {
            Some(t) => t,
            None => unreachable!("tree checked in ScanSession::read"),
        }
    }
}

#[derive(Default)]
pub struct ScanRegistry {
    sessions: Mutex<HashMap<ScanId, Arc<ScanSession>>>,
}

impl ScanRegistry {
    pub fn insert(&self, id: ScanId, s: Arc<ScanSession>) {
        lock(&self.sessions).insert(id, s);
    }

    pub fn get(&self, id: ScanId) -> Result<Arc<ScanSession>, ApiError> {
        lock(&self.sessions).get(&id).cloned().ok_or_else(|| ApiError::new("scan_not_found"))
    }

    pub fn remove(&self, id: ScanId) {
        lock(&self.sessions).remove(&id);
    }

    /// Only the newest finished scan per root stays in memory, trees can be large.
    pub fn drop_older_than(&self, id: ScanId, root: &str) {
        lock(&self.sessions).retain(|k, s| *k >= id || !s.root.eq_ignore_ascii_case(root));
    }

    pub fn clear(&self) {
        lock(&self.sessions).clear();
    }

    pub fn folder_meta(&self, scan_id: ScanId, node_id: NodeId) -> Result<FolderMeta, ApiError> {
        let s = self.get(scan_id)?;
        let tree = s.read()?;
        folder_meta(&tree, node_id)
    }
}

/// Runs the knowledge base over the tree and stores the tags on it.
pub fn tag_tree(tree: &mut ScanTree, rules: &RuleSet, now: i64) -> Vec<(NodeId, u32)> {
    let mut found: Vec<(NodeId, u32)> = Vec::new();
    tree.visit_dirs_and_files(&mut |e| {
        if e.depth == 0 {
            return Visit::Continue;
        }
        match rules.match_path(e.path_lower, e.is_dir, e.modified, now) {
            Some(idx) => {
                found.push((e.id, idx));
                let stop = rules.get(idx).map(|r| r.stops_descent()).unwrap_or(true);
                if stop {
                    Visit::SkipChildren
                } else {
                    Visit::Continue
                }
            }
            None => Visit::Continue,
        }
    });
    tree.clear_tags();
    tree.set_tags(found.iter().filter_map(|&(id, idx)| rules.get(idx).map(|r| (id, r.category, Some(idx)))));
    found
}

/// Node info with the knowledge base explanation filled in.
pub fn explained(tree: &ScanTree, rules: &RuleSet, id: NodeId) -> Option<NodeInfo> {
    let mut info = tree.node_info(id)?;
    if let Some((_, Some(idx))) = tree.tag_of(id) {
        info.explanation = rules.explanation(idx);
    }
    Some(info)
}

pub fn explain_all(tree: &ScanTree, rules: &RuleSet, items: &mut [NodeInfo]) {
    for n in items {
        if let Some((_, Some(idx))) = tree.tag_of(n.id) {
            n.explanation = rules.explanation(idx);
        }
    }
}

fn is_destructive(m: CleanupMethod) -> bool {
    matches!(m, CleanupMethod::Recycle | CleanupMethod::DeleteContents)
}

pub fn build_story(
    tree: &ScanTree,
    rules: &RuleSet,
    summary: &ScanSummary,
    matches: &[(NodeId, u32)],
) -> Story {
    let mut reasons: Vec<Reason> = matches
        .iter()
        .filter_map(|&(id, idx)| {
            let rule = rules.get(idx)?;
            // Containers like "Program Files" describe a place, not something to act on.
            if !rule.stops_descent() {
                return None;
            }
            let node = tree.node(id)?;
            let bytes = node.total_size;
            if bytes == 0 {
                return None;
            }
            Some(Reason {
                node_id: id,
                path: tree.path_of(id),
                bytes,
                category: rule.category,
                explanation: rules.explanation(idx)?,
            })
        })
        .collect();
    reasons.sort_by_key(|r| std::cmp::Reverse(r.bytes));

    let safe_items: Vec<Reason> = reasons
        .iter()
        .filter(|r| r.explanation.safety == SafetyLevel::Safe && is_destructive(r.explanation.method))
        .cloned()
        .collect();
    let safe_bytes = safe_items.iter().map(|r| r.bytes).sum();
    let needs_decision: Vec<Reason> = reasons
        .iter()
        .filter(|r| {
            matches!(r.explanation.safety, SafetyLevel::ProbablySafe | SafetyLevel::Careful)
                && r.explanation.method != CleanupMethod::ManualOnly
                && r.bytes >= 50 * 1024 * 1024
        })
        .take(40)
        .cloned()
        .collect();

    let mut buckets: Vec<StoryBucket> = tree
        .category_totals()
        .into_iter()
        .filter(|(_, b)| *b > 0)
        .map(|(category, bytes)| StoryBucket { category, bytes })
        .collect();
    buckets.sort_by_key(|b| std::cmp::Reverse(b.bytes));

    Story {
        scan_id: summary.scan_id,
        root_path: summary.root_path.clone(),
        drive_total: summary.drive_total,
        drive_free: summary.drive_free,
        counted_bytes: summary.total_bytes,
        buckets,
        reasons: reasons.into_iter().take(25).collect(),
        safe_items,
        safe_bytes,
        needs_decision,
    }
}

pub fn folder_meta(tree: &ScanTree, id: NodeId) -> Result<FolderMeta, ApiError> {
    let node = tree.node_info(id).ok_or_else(|| ApiError::new("not_found"))?;
    let (oldest, newest) = tree.subtree_date_range(id);
    let child_names = tree
        .children_page(id, fazasanj_model::ChildSort::Size, 0, 15)
        .map(|p| p.items.into_iter().map(|c| c.name).collect())
        .unwrap_or_default();
    Ok(FolderMeta {
        path: node.path,
        total_bytes: node.size,
        file_count: node.file_count,
        dir_count: node.dir_count,
        top_extensions: tree.subtree_extension_stats(id, 10),
        oldest_modified: oldest,
        newest_modified: newest,
        child_names,
        parent_app_hint: None,
    })
}
