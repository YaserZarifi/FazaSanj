use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use fazasanj_model::{CleanupOptions, CleanupPlan, CleanupProgress, CleanupReport, CleanupTarget};
use fazasanj_safety::SafetyContext;

use crate::error::CleanupError;
use crate::exec::{self, Run};
use crate::guard::Guard;
use crate::plan::{self, ActionExtras};
use crate::win;

pub struct CleanupEngine {
    guard: Guard,
}

impl CleanupEngine {
    /// `debug` should be `cfg!(debug_assertions)` of the app. In debug mode every destructive
    /// step must be inside `sandbox`, and nothing destructive runs without one.
    pub fn new(safety: SafetyContext, sandbox: Option<PathBuf>, debug: bool) -> Self {
        Self { guard: Guard::new(safety, sandbox, debug) }
    }

    pub fn build_plan(&self, plan_id: u32, targets: &[CleanupTarget]) -> CleanupPlan {
        self.build_plan_with(plan_id, targets, &|_| ActionExtras::default())
    }

    /// Same as [`CleanupEngine::build_plan`], with rule data the explanation does not carry
    /// (command program and args, settings page to open).
    pub fn build_plan_with(
        &self,
        plan_id: u32,
        targets: &[CleanupTarget],
        extras: &dyn Fn(&CleanupTarget) -> ActionExtras,
    ) -> CleanupPlan {
        plan::build(&self.guard, plan_id, targets, extras)
    }

    /// Runs (or dry runs) a plan. `run_id` and `job_id` are left at 0 for the app to fill.
    pub fn execute(
        &self,
        plan: &CleanupPlan,
        opts: CleanupOptions,
        cancel: &AtomicBool,
        progress: &dyn Fn(CleanupProgress),
    ) -> CleanupReport {
        let started_at = now_ms();
        let mut drives: Vec<PathBuf> = plan.actions.iter().filter(|a| !a.blocked).filter_map(exec::action_drive).collect();
        drives.sort();
        drives.dedup();
        let free_before = total_free(&drives);

        let wants_system = plan.actions.iter().any(|a| {
            !a.blocked && plan::is_system(a.method) && self.guard.check_sandbox(Path::new(&a.path)).is_ok()
        });
        let restore_point_created = if opts.create_restore_point && !opts.dry_run && wants_system {
            Some(exec::create_restore_point("Fazasanj").is_ok())
        } else {
            None
        };

        let total = plan.actions.len() as u32;
        let mut results = Vec::with_capacity(plan.actions.len());
        let mut freed = 0u64;
        for a in &plan.actions {
            if cancel.load(Ordering::Relaxed) {
                results.push(exec::failed(a, CleanupError::Cancelled));
                continue;
            }
            // The user asked for a safety net before system changes. No net, no system change.
            if plan::is_system(a.method) && restore_point_created == Some(false) {
                results.push(exec::failed(a, CleanupError::RestorePointFailed));
                continue;
            }
            let base = freed;
            let report = |p: &Path, bytes: u64| {
                progress(CleanupProgress {
                    job_id: 0,
                    index: a.index,
                    total,
                    current_path: p.to_string_lossy().into_owned(),
                    bytes_freed: base + bytes,
                })
            };
            report(Path::new(&a.path), 0);
            let run = Run { guard: &self.guard, opts, cancel, progress: &report };
            let r = exec::run_action(a, &run);
            freed += r.bytes_freed;
            results.push(r);
        }

        CleanupReport {
            job_id: 0,
            run_id: 0,
            dry_run: opts.dry_run,
            free_before,
            free_after: total_free(&drives),
            bytes_freed: freed,
            restore_point_created,
            results,
            started_at,
            finished_at: now_ms(),
        }
    }
}

fn total_free(drives: &[PathBuf]) -> u64 {
    drives.iter().filter_map(|d| win::free_space(d)).sum()
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
