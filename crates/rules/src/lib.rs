//! Knowledge base rules: loading, validation and fast path matching.
//!
//! Rules live in `crates/rules/rules/*.json` and are embedded at compile time.
//! See `docs/ARCHITECTURE.md` section 6 for the schema.

mod category;
mod error;
mod matcher;
mod pattern;
mod rule;
mod validate;

use std::path::Path;

use fazasanj_model::{Explanation, ExplanationSource};

pub use category::guess_category;
pub use error::RulesError;
pub use pattern::SAMPLE_USER;
pub use rule::{MatchKind, Rule, RuleCommand, COMMAND_ALLOW_LIST};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
}

/// A concrete sample item for a rule, relative to a drive root (no drive letter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    /// Like `Users\Test\AppData\Local\Temp`.
    pub rel_path: String,
    pub is_dir: bool,
    /// Files must be at least this old to match.
    pub min_age_days: Option<u32>,
}

pub struct RuleSet {
    rules: Vec<Rule>,
    explanations: Vec<Explanation>,
    matcher: matcher::Matcher,
}

impl std::fmt::Debug for RuleSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleSet").field("rules", &self.rules.len()).finish()
    }
}

impl RuleSet {
    /// Loads the rule files built into the binary.
    pub fn load_embedded() -> Result<RuleSet, RulesError> {
        Self::from_json_strs(embedded::EMBEDDED)
    }

    /// Loads rules from files on disk (tests, tooling).
    pub fn from_json_files<P: AsRef<Path>>(files: &[P]) -> Result<RuleSet, RulesError> {
        let mut loaded = Vec::new();
        for f in files {
            let name = f.as_ref().display().to_string();
            let text = std::fs::read_to_string(f).map_err(|source| RulesError::Io { file: name.clone(), source })?;
            loaded.push((name, text));
        }
        let refs: Vec<(&str, &str)> = loaded.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
        Self::from_json_strs(&refs)
    }

    /// Loads rules from `(file name, JSON text)` pairs. Each JSON text is an array of rules.
    pub fn from_json_strs(files: &[(&str, &str)]) -> Result<RuleSet, RulesError> {
        let mut rules = Vec::new();
        for (name, text) in files {
            let mut part: Vec<Rule> =
                serde_json::from_str(text).map_err(|source| RulesError::Json { file: (*name).to_string(), source })?;
            rules.append(&mut part);
        }
        Self::from_rules(rules)
    }

    pub fn from_rules(rules: Vec<Rule>) -> Result<RuleSet, RulesError> {
        let issues = validate::validate_rules(&rules);
        if !issues.is_empty() {
            return Err(RulesError::Invalid(issues));
        }
        let matcher = matcher::Matcher::build(&rules).map_err(|e| RulesError::Pattern {
            id: rules.get(e.rule).map(|r| r.id.clone()).unwrap_or_default(),
            pattern: e.pattern,
            reason: e.reason,
        })?;
        let explanations = rules.iter().map(explain).collect();
        let set = RuleSet { rules, explanations, matcher };
        let bad_examples = set.example_issues();
        if !bad_examples.is_empty() {
            return Err(RulesError::Invalid(bad_examples));
        }
        Ok(set)
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    pub fn get(&self, idx: u32) -> Option<&Rule> {
        self.rules.get(idx as usize)
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn by_id(&self, id: &str) -> Option<u32> {
        self.rules.iter().position(|r| r.id == id).map(|i| i as u32)
    }

    /// The explanation shown to the user for a rule. `None` only for a bad index.
    pub fn explanation(&self, idx: u32) -> Option<Explanation> {
        self.explanations.get(idx as usize).cloned()
    }

    /// Finds the most specific rule for a node. `modified_ms` and `now_ms` are unix ms and only
    /// matter for rules with `min_age_days`. Returns the rule index.
    ///
    /// Call it top-down. When a folder matches and [`Rule::stops_descent`] is true, the
    /// children belong to that rule and don't need to be matched.
    pub fn match_path(&self, path: &str, is_dir: bool, modified_ms: Option<i64>, now_ms: i64) -> Option<u32> {
        self.matcher.match_path(path, is_dir, modified_ms, now_ms)
    }

    /// Runs all structural checks again. Loading already does this, tests call it directly.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut issues = validate::validate_rules(&self.rules);
        issues.extend(self.example_issues());
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }

    /// Sample items for a rule: its hand written `examples`, or one synthesized per pattern.
    pub fn examples(&self, idx: u32) -> Vec<Example> {
        let Some(r) = self.get(idx) else {
            return Vec::new();
        };
        let is_dir = r.match_kind != MatchKind::File;
        let make = |rel_path: String| Example { rel_path, is_dir, min_age_days: r.min_age_days };
        if !r.examples.is_empty() {
            return r.examples.iter().filter_map(|e| pattern::synthesize(e).ok()).map(make).collect();
        }
        let mut out = Vec::new();
        for p in &r.paths {
            let Ok(base) = pattern::synthesize(p) else { continue };
            if r.file_patterns.is_empty() {
                out.push(make(base));
            } else {
                for fp in &r.file_patterns {
                    let name = pattern::synthesize_name(fp);
                    out.push(make(if base.is_empty() { name } else { format!("{base}\\{name}") }));
                }
            }
        }
        out
    }

    fn example_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for (i, r) in self.rules.iter().enumerate() {
            for e in &r.examples {
                if let Err(reason) = pattern::synthesize(e) {
                    issues.push(format!("{}: example {e:?}: {reason}", r.id));
                }
            }
            if self.examples(i as u32).is_empty() {
                issues.push(format!("{}: no usable example", r.id));
            }
        }
        issues
    }
}

fn explain(r: &Rule) -> Explanation {
    Explanation {
        rule_id: r.id.clone(),
        source: ExplanationSource::KnowledgeBase,
        title: r.title.clone(),
        why_big: r.why_big.clone(),
        if_deleted: r.if_deleted.clone(),
        safety: r.safety,
        method: r.method,
        needs_admin: r.needs_admin,
        instructions: r.instructions.clone(),
        confidence: None,
    }
}
