//! Shared data types sent between the Rust core and the UI.
//!
//! Everything here is serialized with camelCase field names and must stay in sync with
//! `src/lib/api/types.ts`. Sizes are bytes, times are unix milliseconds.

use serde::{Deserialize, Serialize};

pub type NodeId = u32;
pub type ScanId = u32;
pub type JobId = u32;

// ---------- common ----------

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bilingual {
    pub fa: String,
    pub en: String,
}

impl Bilingual {
    pub fn new(fa: impl Into<String>, en: impl Into<String>) -> Self {
        Self { fa: fa.into(), en: en.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyLevel {
    Safe,
    ProbablySafe,
    Careful,
    DoNotTouch,
}

impl SafetyLevel {
    /// Returns the less permissive of the two levels.
    pub fn at_most(self, cap: SafetyLevel) -> SafetyLevel {
        if self < cap {
            cap
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    System,
    Apps,
    Games,
    Media,
    Dev,
    Cache,
    UserFiles,
    Virtualization,
    Messaging,
    Browsers,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupMethod {
    Recycle,
    DeleteContents,
    Command,
    OpenAppSetting,
    CompactVhdx,
    ManualOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplanationSource {
    KnowledgeBase,
    Heuristic,
    Ai,
}

/// Error sent to the UI. `code` is stable and translated by the UI (`errors.<code>`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub detail: Option<String>,
}

impl ApiError {
    pub fn new(code: impl Into<String>) -> Self {
        Self { code: code.into(), detail: None }
    }
    pub fn with_detail(code: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { code: code.into(), detail: Some(detail.into()) }
    }
}

// ---------- drives ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// "C:"
    pub letter: String,
    /// "C:\"
    pub root: String,
    pub label: String,
    /// "NTFS", "FAT32", "exFAT", "ReFS"...
    pub filesystem: String,
    pub total: u64,
    pub free: u64,
    pub kind: DriveKind,
    pub is_system: bool,
    pub fast_scan_available: bool,
}

// ---------- scanning ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanMode {
    Normal,
    Fast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackReason {
    UacRefused,
    NotNtfs,
    HelperFailed,
    Timeout,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeFlags {
    pub cloud_only: bool,
    pub access_denied: bool,
    pub reparse: bool,
    pub compressed: bool,
    pub sparse: bool,
    pub hardlink_dup: bool,
    pub system: bool,
}

/// What the knowledge base (or a heuristic, or AI) says about a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub rule_id: String,
    pub source: ExplanationSource,
    pub title: Bilingual,
    pub why_big: Bilingual,
    pub if_deleted: Bilingual,
    pub safety: SafetyLevel,
    pub method: CleanupMethod,
    pub needs_admin: bool,
    pub instructions: Option<Bilingual>,
    /// 0..1, only for heuristics and AI.
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    /// Real size on disk of the node and everything under it.
    pub size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub child_count: u32,
    /// Newest modification time in the subtree (unix ms).
    pub modified: Option<i64>,
    pub flags: NodeFlags,
    pub category: Category,
    pub explanation: Option<Explanation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChildSort {
    Size,
    Name,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildrenPage {
    pub total: u32,
    pub offset: u32,
    pub items: Vec<NodeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scan_id: ScanId,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub current_path: String,
    pub elapsed_ms: u64,
    pub scanner: ScanMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scan_id: ScanId,
    pub root_path: String,
    pub root_node: NodeId,
    pub total_bytes: u64,
    pub files: u64,
    pub dirs: u64,
    pub access_denied: u32,
    pub cloud_only: u64,
    pub duration_ms: u64,
    pub scanner: ScanMode,
    pub fallback_reason: Option<FallbackReason>,
    /// Drive totals at scan time, for "used vs counted" comparison.
    pub drive_total: u64,
    pub drive_free: u64,
    pub finished_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanFailed {
    pub scan_id: ScanId,
    pub error: ApiError,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TreemapNode {
    pub id: NodeId,
    pub name: String,
    pub size: u64,
    pub file_count: u64,
    pub modified: Option<i64>,
    pub category: Category,
    pub is_dir: bool,
    /// True for the grouped "other small items" entry. Its id is the parent id.
    pub is_other: bool,
    pub children: Vec<TreemapNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: NodeId,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: Option<i64>,
    pub category: Category,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeGroupKind {
    Video,
    Images,
    Audio,
    Archives,
    Installers,
    DiskImages,
    Documents,
    Code,
    Executables,
    System,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionStat {
    pub ext: String,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeGroup {
    pub group: TypeGroupKind,
    pub bytes: u64,
    pub files: u64,
    pub top_extensions: Vec<ExtensionStat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccessDeniedEntry {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannerComparison {
    pub normal_bytes: u64,
    pub fast_bytes: u64,
    pub normal_files: u64,
    pub fast_files: u64,
    pub normal_ms: u64,
    pub fast_ms: u64,
    /// Folders whose size differs by more than 1%, biggest difference first.
    pub differences: Vec<PathDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathDiff {
    pub path: String,
    pub normal_bytes: u64,
    pub fast_bytes: u64,
}

// ---------- simple mode story ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryBucket {
    pub category: Category,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reason {
    pub node_id: NodeId,
    pub path: String,
    pub bytes: u64,
    pub category: Category,
    pub explanation: Explanation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Story {
    pub scan_id: ScanId,
    pub root_path: String,
    pub drive_total: u64,
    pub drive_free: u64,
    pub counted_bytes: u64,
    /// Biggest categories, largest first.
    pub buckets: Vec<StoryBucket>,
    /// Top explained items (largest first), used for the story sentence and the cards.
    pub reasons: Vec<Reason>,
    /// Items with safety `safe`, what the big button frees.
    pub safe_items: Vec<Reason>,
    pub safe_bytes: u64,
    /// `probably_safe` and `careful` items that need a human decision.
    pub needs_decision: Vec<Reason>,
}

// ---------- heuristics ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeuristicKind {
    Orphan,
    Stale,
    Duplicates,
    OldProject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub path: String,
    pub modified: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
pub enum FindingDetails {
    Orphan {
        app_name: String,
    },
    Stale {
        last_modified: Option<i64>,
        last_accessed: Option<i64>,
        access_time_reliable: bool,
    },
    Duplicates {
        file_size: u64,
        files: Vec<DuplicateFile>,
        keep_index: usize,
    },
    OldProject {
        project_kind: String,
        last_touched: Option<i64>,
        rebuildable_bytes: u64,
        rebuildable_dirs: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicFinding {
    pub kind: HeuristicKind,
    pub path: String,
    pub node_id: Option<NodeId>,
    /// Space you would get back (for duplicates: all copies except the kept one).
    pub bytes: u64,
    pub confidence: f32,
    /// Never `safe`.
    pub safety: SafetyLevel,
    pub reason: Bilingual,
    pub details: FindingDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicsProgress {
    pub job_id: JobId,
    pub kind: HeuristicKind,
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeuristicsDone {
    pub job_id: JobId,
    pub scan_id: ScanId,
    pub findings: Vec<HeuristicFinding>,
}

// ---------- cleanup ----------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupTarget {
    pub path: String,
    pub bytes: u64,
    pub explanation: Explanation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanWarningCode {
    /// Recycle Bin on the same drive does not free space.
    RecycleSameDrive,
    NeedsAdmin,
    SystemAction,
    Blocked,
    OutsideSandbox,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWarning {
    pub code: PlanWarningCode,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupAction {
    pub index: u32,
    pub path: String,
    pub title: Bilingual,
    pub method: CleanupMethod,
    pub bytes: u64,
    pub safety: SafetyLevel,
    pub consequence: Bilingual,
    pub needs_admin: bool,
    /// Skip the Recycle Bin. Only possible for `safe` items.
    pub permanent: bool,
    pub instructions: Option<Bilingual>,
    /// Human readable command line for `command` actions.
    pub command: Option<String>,
    pub blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub plan_id: u32,
    pub actions: Vec<CleanupAction>,
    pub total_bytes: u64,
    pub needs_admin: bool,
    pub has_system_actions: bool,
    pub warnings: Vec<PlanWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOptions {
    pub dry_run: bool,
    /// Permanently delete `safe` caches instead of using the Recycle Bin.
    pub permanent_for_safe: bool,
    pub create_restore_point: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Done,
    DryRun,
    Partial,
    SkippedInUse,
    Blocked,
    Failed,
    NeedsManual,
    OpenedSetting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub index: u32,
    pub path: String,
    pub method: CleanupMethod,
    pub status: ActionStatus,
    pub bytes_freed: u64,
    pub files_removed: u64,
    pub files_skipped: u64,
    pub skipped_paths: Vec<String>,
    pub error: Option<ApiError>,
    /// What would happen, for dry runs (list of paths, capped).
    pub would_remove: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupProgress {
    pub job_id: JobId,
    pub index: u32,
    pub total: u32,
    pub current_path: String,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupReport {
    pub job_id: JobId,
    pub run_id: i64,
    pub dry_run: bool,
    pub free_before: u64,
    pub free_after: u64,
    pub bytes_freed: u64,
    pub restore_point_created: Option<bool>,
    pub results: Vec<ActionResult>,
    pub started_at: i64,
    pub finished_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedAction {
    pub path: String,
    pub method: CleanupMethod,
    pub bytes: u64,
    pub status: ActionStatus,
    /// Still in the Recycle Bin.
    pub restorable: bool,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub run_id: i64,
    pub started_at: i64,
    pub finished_at: i64,
    pub dry_run: bool,
    pub bytes_freed: u64,
    pub actions: Vec<LoggedAction>,
}

// ---------- AI ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    OpenAi,
    Gemini,
    Anthropic,
    Groq,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub provider: AiProvider,
    pub has_key: bool,
    pub model: String,
    pub available_models: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiAnswer {
    pub path: String,
    /// What the folder probably is.
    pub what: String,
    pub why_big: String,
    /// Capped at `probably_safe`.
    pub safety: SafetyLevel,
    pub consequence: String,
    /// "fa" or "en"
    pub language: String,
    pub provider: AiProvider,
    pub model: String,
    pub cached: bool,
    pub created_at: i64,
}

// ---------- snapshots and growth ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: i64,
    pub root_path: String,
    pub taken_at: i64,
    pub total_bytes: u64,
    pub files: u64,
    pub drive_total: u64,
    pub drive_free: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthItem {
    pub path: String,
    pub before: u64,
    pub after: u64,
    pub delta: i64,
    pub explanation: Option<Explanation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotComparison {
    pub from: SnapshotInfo,
    pub to: SnapshotInfo,
    pub total_delta: i64,
    /// Biggest absolute change first.
    pub items: Vec<GrowthItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrowthPoint {
    pub taken_at: i64,
    pub bytes: u64,
}

// ---------- settings ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Fa,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePref {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiMode {
    Simple,
    Expert,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub language: Language,
    pub theme: ThemePref,
    pub default_mode: UiMode,
    pub excluded_paths: Vec<String>,
    pub stale_months: u32,
    pub old_project_months: u32,
    pub ai_enabled: bool,
    pub ai_mask_names: bool,
    pub ai_default_provider: Option<AiProvider>,
    pub ai_preview_acknowledged: bool,
    pub low_space_threshold_gb: u32,
    pub tray_enabled: bool,
    pub weekly_check: bool,
    pub onboarding_done: bool,
    /// Debug builds only: destructive actions are limited to this path.
    pub dev_sandbox: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::Fa,
            theme: ThemePref::System,
            default_mode: UiMode::Simple,
            excluded_paths: Vec::new(),
            stale_months: 12,
            old_project_months: 6,
            ai_enabled: false,
            ai_mask_names: false,
            ai_default_provider: None,
            ai_preview_acknowledged: false,
            low_space_threshold_gb: 10,
            tray_enabled: false,
            weekly_check: false,
            onboarding_done: false,
            dev_sandbox: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub is_elevated: bool,
    pub debug_build: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_cap() {
        assert_eq!(SafetyLevel::Safe.at_most(SafetyLevel::ProbablySafe), SafetyLevel::ProbablySafe);
        assert_eq!(SafetyLevel::Careful.at_most(SafetyLevel::ProbablySafe), SafetyLevel::Careful);
    }

    #[test]
    fn enums_serialize_snake_case() {
        let s = serde_json::to_string(&SafetyLevel::ProbablySafe).unwrap();
        assert_eq!(s, "\"probably_safe\"");
        let s = serde_json::to_string(&CleanupMethod::CompactVhdx).unwrap();
        assert_eq!(s, "\"compact_vhdx\"");
    }

    #[test]
    fn settings_fill_defaults() {
        let s: AppSettings = serde_json::from_str("{\"language\":\"en\"}").unwrap();
        assert_eq!(s.language, Language::En);
        assert_eq!(s.stale_months, 12);
    }
}
