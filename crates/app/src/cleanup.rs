//! Cleanup plans and runs. The engine enforces every safety rule; this layer only makes sure the
//! UI can not smuggle in a friendlier explanation than the knowledge base gives.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use fazasanj_cleanup::{ActionExtras, CleanupEngine, CommandSpec, OpenTarget};
use fazasanj_model::{
    ApiError, CleanupOptions, CleanupPlan, CleanupTarget, ExplanationSource, HistoryEntry, JobId,
};
use fazasanj_rules::RuleSet;
use fazasanj_safety::SafetyContext;

use crate::{lock, store_err, ApiResult, App, Event};

/// Plans kept for `run_cleanup`. Older ones are dropped.
const MAX_PLANS: usize = 20;

impl App {
    fn engine(&self) -> CleanupEngine {
        let safety = SafetyContext::from_env(self.install_dir.as_deref());
        let sandbox = self
            .settings()
            .dev_sandbox
            .or_else(|| std::env::var("DEV_SANDBOX").ok().filter(|v| !v.trim().is_empty()))
            .map(PathBuf::from);
        CleanupEngine::new(safety, sandbox, cfg!(debug_assertions))
    }

    pub fn build_cleanup_plan(&self, targets: Vec<CleanupTarget>) -> ApiResult<CleanupPlan> {
        let targets = trusted_targets(targets, &self.rules);
        if targets.is_empty() {
            return Err(ApiError::new("plan_empty"));
        }
        let plan_id = self.next_id();
        let rules = &self.rules;
        let plan = self.engine().build_plan_with(plan_id, &targets, &|t| extras_for(rules, t));
        let mut plans = lock(&self.plans);
        if plans.len() >= MAX_PLANS {
            if let Some(&oldest) = plans.keys().min() {
                plans.remove(&oldest);
            }
        }
        plans.insert(plan_id, plan.clone());
        Ok(plan)
    }

    /// Runs (or dry runs) a plan built earlier. The report arrives in `cleanup://done`.
    pub fn run_cleanup(self: &Arc<Self>, plan_id: u32, options: CleanupOptions) -> ApiResult<JobId> {
        let plan = lock(&self.plans).get(&plan_id).cloned().ok_or_else(|| ApiError::new("plan_not_found"))?;
        let job_id = self.next_id();
        let app = self.clone();
        std::thread::Builder::new()
            .name(format!("cleanup-{job_id}"))
            .spawn(move || {
                let cancel = AtomicBool::new(false);
                let engine = app.engine();
                let progress = |mut p: fazasanj_model::CleanupProgress| {
                    p.job_id = job_id;
                    app.emit(Event::CleanupProgress(p));
                };
                let mut report = engine.execute(&plan, options, &cancel, &progress);
                report.job_id = job_id;
                match app.store.log_cleanup(&report) {
                    Ok(run_id) => report.run_id = run_id,
                    Err(e) => log::warn!("could not log cleanup: {e}"),
                }
                app.emit(Event::CleanupDone(report));
            })
            .map_err(|e| ApiError::with_detail("cleanup_failed", e.to_string()))?;
        Ok(job_id)
    }

    pub fn cleanup_history(&self, limit: u32, offset: u32) -> ApiResult<Vec<HistoryEntry>> {
        self.store.history(limit.clamp(1, 200) as usize, offset as usize).map_err(store_err)
    }
}

/// Knowledge base explanations are replaced with the rule's own text, so only the rule file can
/// call something safe. Unknown rule ids are dropped. Heuristic and AI explanations are kept
/// (the engine caps them at probably safe).
pub(crate) fn trusted_targets(targets: Vec<CleanupTarget>, rules: &RuleSet) -> Vec<CleanupTarget> {
    targets
        .into_iter()
        .filter(|t| !t.path.trim().is_empty())
        .filter_map(|mut t| {
            if t.explanation.source == ExplanationSource::KnowledgeBase {
                let idx = rules.by_id(&t.explanation.rule_id)?;
                t.explanation = rules.explanation(idx)?;
            }
            Some(t)
        })
        .collect()
}

fn extras_for(rules: &RuleSet, t: &CleanupTarget) -> ActionExtras {
    if t.explanation.source != ExplanationSource::KnowledgeBase {
        return ActionExtras::default();
    }
    let Some(rule) = rules.by_id(&t.explanation.rule_id).and_then(|i| rules.get(i)) else {
        return ActionExtras::default();
    };
    ActionExtras {
        command: rule
            .command
            .as_ref()
            .map(|c| CommandSpec { program: c.program.clone(), args: rule.command_args(&t.path) }),
        open_target: rule.resolve_open_target(&t.path),
    }
}

/// Only settings pages and tools on the allow list can be opened from the UI.
pub fn checked_open_target(target: &str) -> ApiResult<String> {
    OpenTarget::parse(target).map(|t| t.display()).ok_or_else(|| ApiError::new("open_target_not_allowed"))
}
