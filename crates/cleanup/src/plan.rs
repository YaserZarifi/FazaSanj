//! Turns the user's selection into a plan the UI can show before anything happens.

use std::path::Path;

use fazasanj_model::{
    CleanupAction, CleanupMethod, CleanupPlan, CleanupTarget, ExplanationSource, PlanWarning, PlanWarningCode,
    SafetyLevel,
};

use crate::commands::{CommandSpec, KnownCommand, OpenTarget};
use crate::error::CleanupError;
use crate::guard::Guard;

/// Extra rule data the model's `Explanation` does not carry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionExtras {
    /// For `command` rules: program and args from the rule file.
    pub command: Option<CommandSpec>,
    /// For `open_app_setting` rules: `ms-settings:...` or a known tool name.
    pub open_target: Option<String>,
}

pub(crate) fn is_system(method: CleanupMethod) -> bool {
    matches!(method, CleanupMethod::Command | CleanupMethod::CompactVhdx)
}

pub(crate) fn is_destructive(method: CleanupMethod) -> bool {
    matches!(
        method,
        CleanupMethod::Recycle | CleanupMethod::DeleteContents | CleanupMethod::Command | CleanupMethod::CompactVhdx
    )
}

/// Only the knowledge base may call something `safe`. Heuristics and AI are capped, which also
/// means they can never be deleted permanently.
pub(crate) fn effective_safety(level: SafetyLevel, source: ExplanationSource) -> SafetyLevel {
    if source == ExplanationSource::KnowledgeBase {
        level
    } else {
        level.at_most(SafetyLevel::ProbablySafe)
    }
}

pub(crate) fn build(
    guard: &Guard,
    plan_id: u32,
    targets: &[CleanupTarget],
    extras: &dyn Fn(&CleanupTarget) -> ActionExtras,
) -> CleanupPlan {
    let mut actions = Vec::with_capacity(targets.len());
    let mut warnings = Vec::new();
    for (i, t) in targets.iter().enumerate() {
        let (action, mut w) = build_action(guard, i as u32, t, &extras(t));
        actions.push(action);
        warnings.append(&mut w);
    }
    let live = || actions.iter().filter(|a: &&CleanupAction| !a.blocked);
    CleanupPlan {
        plan_id,
        total_bytes: live().map(|a| a.bytes).sum(),
        needs_admin: live().any(|a| a.needs_admin),
        has_system_actions: live().any(|a| is_system(a.method)),
        actions,
        warnings,
    }
}

fn build_action(guard: &Guard, index: u32, t: &CleanupTarget, extras: &ActionExtras) -> (CleanupAction, Vec<PlanWarning>) {
    let exp = &t.explanation;
    let method = exp.method;
    let safety = effective_safety(exp.safety, exp.source);
    let path = Path::new(&t.path);
    let mut needs_admin = exp.needs_admin;
    let mut blocked = false;
    let mut outside_sandbox = false;
    let mut command = None;

    if is_destructive(method) && exp.safety == SafetyLevel::DoNotTouch {
        blocked = true;
    }
    match method {
        CleanupMethod::Recycle | CleanupMethod::DeleteContents | CleanupMethod::CompactVhdx => {
            if guard.check_block_list(path).is_err() {
                blocked = true;
            }
        }
        CleanupMethod::Command => {
            let known = extras
                .command
                .as_ref()
                .and_then(|s| KnownCommand::from_spec(s, &t.path))
                .or_else(|| KnownCommand::infer_from_path(&t.path));
            match known {
                Some(k) => {
                    needs_admin |= k.needs_admin();
                    command = Some(k.display());
                }
                None => blocked = true,
            }
        }
        CleanupMethod::OpenAppSetting => {
            command = extras.open_target.as_deref().and_then(OpenTarget::parse).map(|o| o.display());
        }
        CleanupMethod::ManualOnly => {}
    }
    // Heuristic and AI guesses never get to run system tools.
    if is_system(method) && exp.source != ExplanationSource::KnowledgeBase {
        blocked = true;
    }
    if is_destructive(method) {
        if let Err(e) = guard.check_sandbox(path) {
            if matches!(e, CleanupError::Sandbox(_)) {
                outside_sandbox = true;
                blocked = true;
            }
        }
    }

    let mut w = Vec::new();
    let warn = |code| PlanWarning { code, path: Some(t.path.clone()) };
    if blocked {
        w.push(warn(PlanWarningCode::Blocked));
    }
    if outside_sandbox {
        w.push(warn(PlanWarningCode::OutsideSandbox));
    }
    if !blocked {
        // Shown even for `safe` items: they only skip the bin when the user turns that on.
        if matches!(method, CleanupMethod::Recycle | CleanupMethod::DeleteContents) {
            w.push(warn(PlanWarningCode::RecycleSameDrive));
        }
        if needs_admin {
            w.push(warn(PlanWarningCode::NeedsAdmin));
        }
        if is_system(method) {
            w.push(warn(PlanWarningCode::SystemAction));
        }
    }

    let action = CleanupAction {
        index,
        path: t.path.clone(),
        title: exp.title.clone(),
        method,
        bytes: t.bytes,
        safety,
        consequence: exp.if_deleted.clone(),
        needs_admin,
        permanent: false,
        instructions: exp.instructions.clone(),
        command,
        blocked,
    };
    (action, w)
}
