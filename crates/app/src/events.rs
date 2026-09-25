//! Events the backend pushes to the UI. Names match `EVENTS` in `src/lib/api/commands.ts`.

use std::sync::Arc;

use fazasanj_model::{
    CleanupProgress, CleanupReport, HeuristicsDone, HeuristicsProgress, ScanFailed, ScanId, ScanProgress, ScanSummary,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    ScanProgress(ScanProgress),
    ScanDone(ScanSummary),
    ScanCancelled(ScanId),
    ScanError(ScanFailed),
    HeuristicsProgress(HeuristicsProgress),
    HeuristicsDone(HeuristicsDone),
    CleanupProgress(CleanupProgress),
    CleanupDone(CleanupReport),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Cancelled {
    scan_id: ScanId,
}

impl Event {
    pub fn name(&self) -> &'static str {
        match self {
            Event::ScanProgress(_) => "scan://progress",
            Event::ScanDone(_) => "scan://done",
            Event::ScanCancelled(_) => "scan://cancelled",
            Event::ScanError(_) => "scan://error",
            Event::HeuristicsProgress(_) => "heuristics://progress",
            Event::HeuristicsDone(_) => "heuristics://done",
            Event::CleanupProgress(_) => "cleanup://progress",
            Event::CleanupDone(_) => "cleanup://done",
        }
    }

    pub fn payload(&self) -> Value {
        let v = match self {
            Event::ScanProgress(p) => serde_json::to_value(p),
            Event::ScanDone(s) => serde_json::to_value(s),
            Event::ScanCancelled(id) => serde_json::to_value(Cancelled { scan_id: *id }),
            Event::ScanError(e) => serde_json::to_value(e),
            Event::HeuristicsProgress(p) => serde_json::to_value(p),
            Event::HeuristicsDone(d) => serde_json::to_value(d),
            Event::CleanupProgress(p) => serde_json::to_value(p),
            Event::CleanupDone(r) => serde_json::to_value(r),
        };
        v.unwrap_or(Value::Null)
    }
}

/// Where events go. The Tauri app forwards them to the window.
pub type EventSink = Arc<dyn Fn(Event) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelled_payload_is_camel_case() {
        let e = Event::ScanCancelled(7);
        assert_eq!(e.name(), "scan://cancelled");
        assert_eq!(e.payload(), serde_json::json!({ "scanId": 7 }));
    }
}
