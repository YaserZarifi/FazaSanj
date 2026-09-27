use fazasanj_model::{ApiError, GrowthItem, GrowthPoint, ScanId, SnapshotComparison, SnapshotInfo};
use tauri::State;

use crate::state::{lock, now_ms, AppState};

fn store_err(e: fazasanj_store::StoreError) -> ApiError {
    ApiError::with_detail(e.code(), e.to_string())
}

fn is_inside(child: &str, parent: &str) -> bool {
    child.len() > parent.len()
        && child[..parent.len()].eq_ignore_ascii_case(parent)
        && child.as_bytes().get(parent.len()) == Some(&b'\\')
}

/// Snapshots store every depth, so a parent and its child often show the same growth.
/// Keep the most specific entry when it explains most of its parent's change.
fn tidy(state: &AppState, mut cmp: SnapshotComparison) -> SnapshotComparison {
    cmp.items.sort_by_key(|i| std::cmp::Reverse(i.delta.unsigned_abs()));
    let mut kept: Vec<GrowthItem> = Vec::new();
    for item in cmp.items {
        if item.delta == 0 {
            continue;
        }
        if let Some(pos) = kept.iter().position(|k| is_inside(&item.path, &k.path)) {
            let parent = &kept[pos];
            if item.delta.signum() == parent.delta.signum() && item.delta.abs() * 10 >= parent.delta.abs() * 8 {
                kept[pos] = item;
            }
            continue;
        }
        if kept.iter().any(|k| is_inside(&k.path, &item.path)) {
            continue;
        }
        kept.push(item);
        if kept.len() >= 50 {
            break;
        }
    }
    let now = now_ms();
    for k in &mut kept {
        k.explanation = state
            .rules
            .match_path(&k.path.to_lowercase(), true, None, now)
            .and_then(|i| state.rules.explanation(i));
    }
    cmp.items = kept;
    cmp
}

#[tauri::command]
pub fn list_snapshots(state: State<'_, AppState>, root_path: Option<String>) -> Result<Vec<SnapshotInfo>, ApiError> {
    state.store.list_snapshots(root_path.as_deref()).map_err(store_err)
}

#[tauri::command]
pub fn compare_snapshots(state: State<'_, AppState>, from_id: i64, to_id: i64) -> Result<SnapshotComparison, ApiError> {
    let cmp = state.store.compare(from_id, to_id, 2000).map_err(store_err)?;
    Ok(tidy(&state, cmp))
}

#[tauri::command]
pub fn compare_with_last(state: State<'_, AppState>, scan_id: ScanId) -> Result<Option<SnapshotComparison>, ApiError> {
    let s = state.scans.get(scan_id)?;
    let Some(to_id) = *lock(&s.snapshot_id) else {
        return Ok(None);
    };
    let Some(prev) = state.store.latest_before(&s.root, to_id).map_err(store_err)? else {
        return Ok(None);
    };
    let cmp = state.store.compare(prev.id, to_id, 2000).map_err(store_err)?;
    Ok(Some(tidy(&state, cmp)))
}

#[tauri::command]
pub fn get_growth(state: State<'_, AppState>, path: String) -> Result<Vec<GrowthPoint>, ApiError> {
    state.store.growth(&path, 60).map_err(store_err)
}
