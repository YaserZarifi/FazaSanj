//! Attaches knowledge base rules to a finished scan tree.

use fazasanj_model::Explanation;
use fazasanj_rules::RuleSet;
use fazasanj_scan::{NodeId, ScanTree, Visit};

/// Matches every node against the rules and tags the hits. A folder matched by a rule that owns
/// its contents is not walked into, so everything below it belongs to that rule.
/// Returns how many nodes were tagged.
pub fn tag_tree(tree: &mut ScanTree, rules: &RuleSet, now_ms: i64) -> usize {
    let mut tags = Vec::new();
    tree.visit_dirs_and_files(&mut |e| {
        let Some(idx) = rules.match_path(e.path, e.is_dir, e.modified, now_ms) else {
            return Visit::Continue;
        };
        let Some(rule) = rules.get(idx) else {
            return Visit::Continue;
        };
        tags.push((e.id, rule.category, Some(idx)));
        if e.is_dir && rule.stops_descent() {
            Visit::SkipChildren
        } else {
            Visit::Continue
        }
    });
    let n = tags.len();
    tree.set_tags(tags);
    n
}

/// The rule explanation attached to this exact node, if any.
pub fn explanation_for(tree: &ScanTree, rules: &RuleSet, id: NodeId) -> Option<Explanation> {
    let (_, rule) = tree.tag_of(id)?;
    rules.explanation(rule?)
}
