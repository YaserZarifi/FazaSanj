#![allow(dead_code)]

use fazasanj_rules::RuleSet;

pub const DAY_MS: i64 = 86_400_000;
pub const NOW: i64 = 1_790_000_000_000;

pub fn rules() -> RuleSet {
    match RuleSet::load_embedded() {
        Ok(r) => r,
        Err(e) => panic!("rules failed to load: {e}"),
    }
}

/// The rule the app would attach while walking top-down: the first ancestor folder whose rule
/// stops descent wins, otherwise the node's own match.
pub fn governing(rules: &RuleSet, path: &str, is_dir: bool, modified: Option<i64>) -> Option<(u32, String)> {
    let comps: Vec<&str> = path.split('\\').filter(|c| !c.is_empty()).collect();
    for n in 1..comps.len() {
        let anc = comps[..n].join("\\");
        let anc = if n == 1 { format!("{anc}\\") } else { anc };
        if let Some(i) = rules.match_path(&anc, true, None, NOW) {
            if rules.get(i).is_some_and(|r| r.stops_descent()) {
                return Some((i, anc));
            }
        }
    }
    rules.match_path(path, is_dir, modified, NOW).map(|i| (i, path.to_string()))
}

pub fn id(rules: &RuleSet, idx: u32) -> String {
    rules.get(idx).map(|r| r.id.clone()).unwrap_or_default()
}

/// Every rule example as a full path on the given drive root (like `T:`).
pub fn example_paths(rules: &RuleSet, drive: &str) -> Vec<(u32, String, bool, Option<i64>)> {
    let mut out = Vec::new();
    for i in 0..rules.len() as u32 {
        for e in rules.examples(i) {
            let modified = Some(NOW - (i64::from(e.min_age_days.unwrap_or(0)) + 30) * DAY_MS);
            out.push((i, format!("{drive}\\{}", e.rel_path), e.is_dir, modified));
        }
    }
    out
}
