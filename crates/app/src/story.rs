//! The Simple mode story: where the space went, in a few buckets and reasons.

use fazasanj_model::{
    CleanupMethod, NodeId, Reason, SafetyLevel, ScanId, Story, StoryBucket,
};
use fazasanj_rules::{MatchKind, RuleSet};

use crate::scans::ScanEntry;
use crate::{ApiResult, App};

const MAX_REASONS: usize = 200;
const MAX_SAFE_ITEMS: usize = 5000;
const MAX_DECISIONS: usize = 200;
/// Smaller items are not worth a decision card.
const MIN_DECISION_BYTES: u64 = 10 * 1024 * 1024;

impl App {
    pub fn story(&self, scan_id: ScanId) -> ApiResult<Story> {
        let scan = self.scan(scan_id)?;
        Ok(build_story(&scan, &self.rules))
    }
}

/// Builds the story from the rule tags of a finished scan.
pub fn build_story(scan: &ScanEntry, rules: &RuleSet) -> Story {
    let tree = &scan.tree;
    let mut reasons: Vec<Reason> = Vec::new();
    // Rules that own a whole folder (or a single file) never overlap, so their bytes can be summed.
    let mut owning: Vec<bool> = Vec::new();
    for id in 0..tree.len() as NodeId {
        let Some((category, Some(rule_idx))) = tree.tag_of(id) else {
            continue;
        };
        let (Some(rule), Some(explanation)) = (rules.get(rule_idx), rules.explanation(rule_idx)) else {
            continue;
        };
        let bytes = tree.node(id).map_or(0, |n| n.total_size);
        if bytes == 0 {
            continue;
        }
        owning.push(rule.stops_descent() || rule.match_kind == MatchKind::File);
        reasons.push(Reason { node_id: id, path: tree.path_of(id), bytes, category, explanation });
    }

    let mut order: Vec<usize> = (0..reasons.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(reasons[i].bytes));

    let mut safe_items = Vec::new();
    let mut needs_decision = Vec::new();
    let mut safe_bytes = 0u64;
    for &i in &order {
        let r = &reasons[i];
        let e = &r.explanation;
        let auto = matches!(e.method, CleanupMethod::Recycle | CleanupMethod::DeleteContents) && !e.needs_admin;
        if e.safety == SafetyLevel::Safe && auto && owning[i] {
            if safe_items.len() < MAX_SAFE_ITEMS {
                safe_bytes += r.bytes;
                safe_items.push(r.clone());
            }
        } else if matches!(e.safety, SafetyLevel::ProbablySafe | SafetyLevel::Careful)
            && r.bytes >= MIN_DECISION_BYTES
            && needs_decision.len() < MAX_DECISIONS
        {
            needs_decision.push(r.clone());
        }
    }

    let reasons: Vec<Reason> = order.iter().take(MAX_REASONS).map(|&i| reasons[i].clone()).collect();
    let buckets = tree.category_totals().into_iter().map(|(category, bytes)| StoryBucket { category, bytes }).collect();

    Story {
        scan_id: scan.summary.scan_id,
        root_path: scan.summary.root_path.clone(),
        drive_total: scan.summary.drive_total,
        drive_free: scan.summary.drive_free,
        counted_bytes: scan.summary.total_bytes,
        buckets,
        reasons,
        safe_items,
        safe_bytes,
        needs_decision,
    }
}
