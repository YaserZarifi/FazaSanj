//! Dev only: fills a sandbox folder or drive with a fake Windows-like tree full of junk,
//! so the scanner, rules, heuristics and cleanup can be tried without real user data.

pub mod guard;
pub mod plan;
pub mod rng;
mod write;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fazasanj_rules::RuleSet;
use thiserror::Error;

pub use guard::{Guard, MARKER};
pub use plan::{Entry, Group, Kind, Plan, Scale};

#[derive(Debug, Error)]
pub enum JunkError {
    #[error("refusing to write to the system drive {0}")]
    SystemDrive(String),
    #[error("{0} looks like a real Windows install, refusing")]
    RealWindows(String),
    #[error("{0} is not empty. Use an empty folder, or --force on a folder junk-gen made before")]
    NotEmpty(String),
    #[error("{0} was made by junk-gen before, pass --force to write it again")]
    NeedsForce(String),
    #[error("target must be an absolute path on a drive, like T:\\ or D:\\sandbox (got {0})")]
    BadTarget(String),
    #[error("rules failed to load: {0}")]
    Rules(String),
    #[error("{0}: {1}")]
    Io(String, #[source] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct Options {
    pub target: PathBuf,
    pub scale: Scale,
    pub seed: u64,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Default)]
pub struct Summary {
    pub dry_run: bool,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    /// Bytes and file count per group.
    pub groups: BTreeMap<Group, (u64, u64)>,
    pub rule_examples: usize,
    pub rules_covered: usize,
    pub rules_total: usize,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mb = |b: u64| b as f64 / (1024.0 * 1024.0);
        let verb = if self.dry_run { "would create" } else { "created" };
        writeln!(f, "{verb} {} files, {} folders, {:.1} MB", self.files, self.dirs, mb(self.bytes))?;
        for (g, (bytes, files)) in &self.groups {
            writeln!(f, "  {:<24} {:>6} files {:>10.1} MB", g.label(), files, mb(*bytes))?;
        }
        write!(
            f,
            "rule examples: {} items covering {} of {} rules",
            self.rule_examples, self.rules_covered, self.rules_total
        )
    }
}

pub fn summarize(plan: &Plan, rules: &RuleSet, dry_run: bool) -> Summary {
    let mut s = Summary { dry_run, rules_total: rules.len(), ..Summary::default() };
    let mut covered = std::collections::HashSet::new();
    for e in &plan.entries {
        match e.kind {
            Kind::Dir => s.dirs += 1,
            Kind::File { size, .. } => {
                s.files += 1;
                s.bytes += size;
                let g = s.groups.entry(e.group).or_default();
                g.0 += size;
                g.1 += 1;
            }
        }
        if let Some(id) = &e.rule_id {
            s.rule_examples += 1;
            covered.insert(id.clone());
        }
    }
    s.rules_covered = covered.len();
    s
}

/// Checks the target, builds the plan and writes it (unless dry run).
pub fn run(opts: &Options, guard: &Guard) -> Result<(Plan, Summary), JunkError> {
    guard.check(&opts.target, opts.force)?;
    let rules = RuleSet::load_embedded().map_err(|e| JunkError::Rules(e.to_string()))?;
    let plan = plan::build(&rules, opts.scale, opts.seed);
    let summary = summarize(&plan, &rules, opts.dry_run);
    if !opts.dry_run {
        let io = |p: &Path, e: std::io::Error| JunkError::Io(p.display().to_string(), e);
        std::fs::create_dir_all(&opts.target).map_err(|e| io(&opts.target, e))?;
        let marker = opts.target.join(MARKER);
        std::fs::write(&marker, "made by junk-gen, safe to delete\n").map_err(|e| io(&marker, e))?;
        write::write_plan(&opts.target, &plan)?;
    }
    Ok((plan, summary))
}
