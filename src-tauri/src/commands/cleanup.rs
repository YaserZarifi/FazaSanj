use std::collections::HashSet;
use std::sync::atomic::Ordering;

use fazasanj_cleanup::{recycle_bin_lookup, ActionExtras, CommandSpec};
use fazasanj_model::{
    ActionStatus, ApiError, CleanupMethod, CleanupOptions, CleanupPlan, CleanupProgress, CleanupTarget,
    ExplanationSource, HistoryEntry, JobId,
};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::AppState;

/// Rule data that the explanation does not carry (command program, settings page).
fn extras_for(state: &AppState, target: &CleanupTarget) -> ActionExtras {
    if target.explanation.source != ExplanationSource::KnowledgeBase {
        return ActionExtras::default();
    }
    let Some(rule) = state.rules.by_id(&target.explanation.rule_id).and_then(|i| state.rules.get(i)) else {
        return ActionExtras::default();
    };
    ActionExtras {
        command: rule.command.as_ref().map(|c| CommandSpec {
            program: c.program.clone(),
            args: rule.command_args(&target.path),
        }),
        open_target: rule.resolve_open_target(&target.path),
    }
}

#[tauri::command]
pub fn build_cleanup_plan(state: State<'_, AppState>, targets: Vec<CleanupTarget>) -> Result<CleanupPlan, ApiError> {
    if targets.is_empty() {
        return Err(ApiError::new("nothing_selected"));
    }
    let engine = state.cleanup_engine();
    let plan_id = state.next_id();
    let plan = engine.build_plan_with(plan_id, &targets, &|t| extras_for(&state, t));
    state.put_plan(plan.clone());
    Ok(plan)
}

#[tauri::command]
pub fn run_cleanup(
    app: AppHandle,
    state: State<'_, AppState>,
    plan_id: u32,
    options: CleanupOptions,
) -> Result<JobId, ApiError> {
    let plan = state.plan(plan_id).ok_or_else(|| ApiError::new("not_found"))?;
    let (job_id, cancel) = state.start_job();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let engine = state.cleanup_engine();
        let emit_app = app.clone();
        let progress = move |mut p: CleanupProgress| {
            p.job_id = job_id;
            let _ = emit_app.emit("cleanup://progress", p);
        };
        let mut report = engine.execute(&plan, options, &cancel, &progress);
        report.job_id = job_id;
        match state.store.log_cleanup(&report) {
            Ok(id) => report.run_id = id,
            Err(e) => log::warn!("could not log cleanup run: {e}"),
        }
        state.end_job(job_id);
        if !cancel.load(Ordering::Relaxed) || !report.results.is_empty() {
            let _ = app.emit("cleanup://done", report);
        }
    });
    Ok(job_id)
}

#[tauri::command]
pub fn get_cleanup_history(state: State<'_, AppState>, limit: usize, offset: usize) -> Result<Vec<HistoryEntry>, ApiError> {
    let mut entries = state
        .store
        .history(limit.clamp(1, 500), offset)
        .map_err(|e| ApiError::with_detail(e.code(), e.to_string()))?;
    let candidates: Vec<String> = entries
        .iter()
        .flat_map(|e| e.actions.iter())
        .filter(|a| {
            matches!(a.method, CleanupMethod::Recycle | CleanupMethod::DeleteContents)
                && matches!(a.status, ActionStatus::Done | ActionStatus::Partial)
        })
        .map(|a| a.path.clone())
        .collect();
    if candidates.is_empty() {
        return Ok(entries);
    }
    let in_bin: HashSet<String> = recycle_bin_lookup(&candidates)
        .into_iter()
        .map(|p| p.to_lowercase())
        .collect();
    for e in &mut entries {
        for a in &mut e.actions {
            a.restorable = in_bin.contains(&a.path.to_lowercase());
        }
    }
    Ok(entries)
}
