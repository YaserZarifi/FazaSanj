//! Structural checks for the rule files. Tests run these, and loading fails on any problem.

use std::collections::HashSet;

use fazasanj_model::{Bilingual, Category, CleanupMethod, SafetyLevel};

use crate::rule::{MatchKind, Rule, COMMAND_ALLOW_LIST};

pub(crate) fn validate_rules(rules: &[Rule]) -> Vec<String> {
    let mut issues = Vec::new();
    let mut ids = HashSet::new();
    for r in rules {
        let mut bad = |msg: String| issues.push(format!("{}: {msg}", r.id));
        if r.id.is_empty() || !r.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
            bad("id must be kebab case".into());
        }
        if !ids.insert(r.id.clone()) {
            bad("duplicate id".into());
        }
        if r.category == Category::Unknown {
            bad("category unknown is not allowed".into());
        }
        if r.paths.is_empty() {
            bad("no paths".into());
        }
        for (field, text) in [("title", &r.title), ("why_big", &r.why_big), ("if_deleted", &r.if_deleted)] {
            check_text(field, text, &mut bad);
        }
        if let Some(i) = &r.instructions {
            check_text("instructions", i, &mut bad);
        }
        if !r.file_patterns.is_empty() && r.match_kind != MatchKind::File {
            bad("file_patterns only work with match=file".into());
        }
        if r.min_age_days.is_some() && r.match_kind != MatchKind::File {
            bad("min_age_days only works with match=file".into());
        }
        if !r.inherit && r.match_kind == MatchKind::File {
            bad("inherit=false makes no sense for files".into());
        }
        if r.permanent_ok && r.safety != SafetyLevel::Safe {
            bad("permanent_ok is only allowed for safe rules".into());
        }
        if r.safety == SafetyLevel::DoNotTouch && r.method != CleanupMethod::ManualOnly {
            bad("do_not_touch rules must use manual_only".into());
        }
        match (r.method, &r.command) {
            (CleanupMethod::Command, None) => bad("method command needs a command".into()),
            (CleanupMethod::Command, Some(c)) => {
                if !COMMAND_ALLOW_LIST.contains(&c.program.as_str()) {
                    bad(format!("program {} is not in the allow list", c.program));
                }
            }
            (_, Some(_)) => bad("command is only allowed with method command".into()),
            _ => {}
        }
        match r.method {
            CleanupMethod::OpenAppSetting => {
                if r.open_target.is_none() {
                    bad("open_app_setting needs open_target".into());
                }
                if r.instructions.is_none() {
                    bad("open_app_setting needs instructions".into());
                }
            }
            CleanupMethod::ManualOnly if r.instructions.is_none() => bad("manual_only needs instructions".into()),
            CleanupMethod::ManualOnly => {}
            _ if r.open_target.is_some() => bad("open_target is only for open_app_setting and manual_only".into()),
            _ => {}
        }
        if let Some(t) = &r.open_target {
            let lower = t.to_ascii_lowercase();
            if !(lower.starts_with("ms-settings:") || lower.starts_with("tg://") || lower.ends_with(".exe")) {
                bad(format!("open_target {t:?} must be an ms-settings: URI, tg:// link or an .exe"));
            }
        }
        if r.method == CleanupMethod::CompactVhdx && r.match_kind != MatchKind::File {
            bad("compact_vhdx works on files".into());
        }
        let vhd_pattern = r.file_patterns.iter().chain(r.paths.iter()).any(|p| {
            let l = p.to_ascii_lowercase();
            l.ends_with(".vhdx") || l.ends_with(".vhd")
        });
        if vhd_pattern && !matches!(r.method, CleanupMethod::CompactVhdx | CleanupMethod::ManualOnly) {
            bad("virtual disks are compacted, never deleted".into());
        }
        if r.method == CleanupMethod::DeleteContents && r.match_kind != MatchKind::Contents {
            bad("delete_contents goes with match=contents".into());
        }
        if r.match_kind == MatchKind::Contents && r.method == CleanupMethod::Recycle {
            bad("match=contents should use delete_contents".into());
        }
        for e in &r.examples {
            if e.contains('*') {
                bad(format!("example {e:?} has wildcards"));
            }
        }
    }
    issues
}

fn check_text(field: &str, t: &Bilingual, bad: &mut impl FnMut(String)) {
    if t.fa.trim().is_empty() || t.en.trim().is_empty() {
        bad(format!("{field} needs both fa and en"));
    }
    if !t.fa.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)) {
        bad(format!("{field}.fa has no Persian text"));
    }
    for s in [&t.fa, &t.en] {
        if s.contains(['\u{2014}', '\u{2013}']) {
            bad(format!("{field} uses a dash as punctuation"));
        }
    }
}
