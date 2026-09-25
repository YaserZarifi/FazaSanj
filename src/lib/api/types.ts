// Mirror of crates/model/src/lib.rs. Keep both in sync.
// Sizes are bytes, times are unix milliseconds.

export type NodeId = number;
export type ScanId = number;
export type JobId = number;

export interface Bilingual {
  fa: string;
  en: string;
}

export type SafetyLevel = "safe" | "probably_safe" | "careful" | "do_not_touch";

export type Category =
  | "system"
  | "apps"
  | "games"
  | "media"
  | "dev"
  | "cache"
  | "user_files"
  | "virtualization"
  | "messaging"
  | "browsers"
  | "unknown";

export type CleanupMethod =
  | "recycle"
  | "delete_contents"
  | "command"
  | "open_app_setting"
  | "compact_vhdx"
  | "manual_only";

export type ExplanationSource = "knowledge_base" | "heuristic" | "ai";

export interface ApiError {
  code: string;
  detail: string | null;
}

// ---------- drives ----------

export type DriveKind = "fixed" | "removable" | "network" | "other";

export interface DriveInfo {
  letter: string;
  root: string;
  label: string;
  filesystem: string;
  total: number;
  free: number;
  kind: DriveKind;
  isSystem: boolean;
  fastScanAvailable: boolean;
}

// ---------- scanning ----------

export type ScanMode = "normal" | "fast";
export type FallbackReason = "uac_refused" | "not_ntfs" | "helper_failed" | "timeout";

export interface NodeFlags {
  cloudOnly: boolean;
  accessDenied: boolean;
  reparse: boolean;
  compressed: boolean;
  sparse: boolean;
  hardlinkDup: boolean;
  system: boolean;
}

export interface Explanation {
  ruleId: string;
  source: ExplanationSource;
  title: Bilingual;
  whyBig: Bilingual;
  ifDeleted: Bilingual;
  safety: SafetyLevel;
  method: CleanupMethod;
  needsAdmin: boolean;
  instructions: Bilingual | null;
  confidence: number | null;
}

export interface NodeInfo {
  id: NodeId;
  parent: NodeId | null;
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  fileCount: number;
  dirCount: number;
  childCount: number;
  modified: number | null;
  flags: NodeFlags;
  category: Category;
  explanation: Explanation | null;
}

export type ChildSort = "size" | "name" | "modified";

export interface ChildrenPage {
  total: number;
  offset: number;
  items: NodeInfo[];
}

export interface ScanProgress {
  scanId: ScanId;
  files: number;
  dirs: number;
  bytes: number;
  currentPath: string;
  elapsedMs: number;
  scanner: ScanMode;
}

export interface ScanSummary {
  scanId: ScanId;
  rootPath: string;
  rootNode: NodeId;
  totalBytes: number;
  files: number;
  dirs: number;
  accessDenied: number;
  cloudOnly: number;
  durationMs: number;
  scanner: ScanMode;
  fallbackReason: FallbackReason | null;
  driveTotal: number;
  driveFree: number;
  finishedAt: number;
}

export interface ScanFailed {
  scanId: ScanId;
  error: ApiError;
}

export interface TreemapNode {
  id: NodeId;
  name: string;
  size: number;
  fileCount: number;
  modified: number | null;
  category: Category;
  isDir: boolean;
  isOther: boolean;
  children: TreemapNode[];
}

export interface FileEntry {
  id: NodeId;
  name: string;
  path: string;
  size: number;
  modified: number | null;
  category: Category;
}

export type TypeGroupKind =
  | "video"
  | "images"
  | "audio"
  | "archives"
  | "installers"
  | "disk_images"
  | "documents"
  | "code"
  | "executables"
  | "system"
  | "other";

export interface ExtensionStat {
  ext: string;
  bytes: number;
  files: number;
}

export interface TypeGroup {
  group: TypeGroupKind;
  bytes: number;
  files: number;
  topExtensions: ExtensionStat[];
}

export interface AccessDeniedEntry {
  path: string;
}

export interface PathDiff {
  path: string;
  normalBytes: number;
  fastBytes: number;
}

export interface ScannerComparison {
  normalBytes: number;
  fastBytes: number;
  normalFiles: number;
  fastFiles: number;
  normalMs: number;
  fastMs: number;
  differences: PathDiff[];
}

// ---------- simple mode story ----------

export interface StoryBucket {
  category: Category;
  bytes: number;
}

export interface Reason {
  nodeId: NodeId;
  path: string;
  bytes: number;
  category: Category;
  explanation: Explanation;
}

export interface Story {
  scanId: ScanId;
  rootPath: string;
  driveTotal: number;
  driveFree: number;
  countedBytes: number;
  buckets: StoryBucket[];
  reasons: Reason[];
  safeItems: Reason[];
  safeBytes: number;
  needsDecision: Reason[];
}

// ---------- heuristics ----------

export type HeuristicKind = "orphan" | "stale" | "duplicates" | "old_project";

export interface DuplicateFile {
  path: string;
  modified: number | null;
}

export type FindingDetails =
  | { kind: "orphan"; appName: string }
  | {
      kind: "stale";
      lastModified: number | null;
      lastAccessed: number | null;
      accessTimeReliable: boolean;
    }
  | { kind: "duplicates"; fileSize: number; files: DuplicateFile[]; keepIndex: number }
  | {
      kind: "old_project";
      projectKind: string;
      lastTouched: number | null;
      rebuildableBytes: number;
      rebuildableDirs: string[];
    };

export interface HeuristicFinding {
  kind: HeuristicKind;
  path: string;
  nodeId: NodeId | null;
  bytes: number;
  confidence: number;
  safety: SafetyLevel;
  reason: Bilingual;
  details: FindingDetails;
}

export interface HeuristicsProgress {
  jobId: JobId;
  kind: HeuristicKind;
  done: number;
  total: number;
}

export interface HeuristicsDone {
  jobId: JobId;
  scanId: ScanId;
  findings: HeuristicFinding[];
}

// ---------- cleanup ----------

export interface CleanupTarget {
  path: string;
  bytes: number;
  explanation: Explanation;
}

export type PlanWarningCode =
  | "recycle_same_drive"
  | "needs_admin"
  | "system_action"
  | "blocked"
  | "outside_sandbox";

export interface PlanWarning {
  code: PlanWarningCode;
  path: string | null;
}

export interface CleanupAction {
  index: number;
  path: string;
  title: Bilingual;
  method: CleanupMethod;
  bytes: number;
  safety: SafetyLevel;
  consequence: Bilingual;
  needsAdmin: boolean;
  permanent: boolean;
  instructions: Bilingual | null;
  command: string | null;
  blocked: boolean;
}

export interface CleanupPlan {
  planId: number;
  actions: CleanupAction[];
  totalBytes: number;
  needsAdmin: boolean;
  hasSystemActions: boolean;
  warnings: PlanWarning[];
}

export interface CleanupOptions {
  dryRun: boolean;
  permanentForSafe: boolean;
  createRestorePoint: boolean;
}

export type ActionStatus =
  | "done"
  | "dry_run"
  | "partial"
  | "skipped_in_use"
  | "blocked"
  | "failed"
  | "needs_manual"
  | "opened_setting";

export interface ActionResult {
  index: number;
  path: string;
  method: CleanupMethod;
  status: ActionStatus;
  bytesFreed: number;
  filesRemoved: number;
  filesSkipped: number;
  skippedPaths: string[];
  error: ApiError | null;
  wouldRemove: string[];
}

export interface CleanupProgress {
  jobId: JobId;
  index: number;
  total: number;
  currentPath: string;
  bytesFreed: number;
}

export interface CleanupReport {
  jobId: JobId;
  runId: number;
  dryRun: boolean;
  freeBefore: number;
  freeAfter: number;
  bytesFreed: number;
  restorePointCreated: boolean | null;
  results: ActionResult[];
  startedAt: number;
  finishedAt: number;
}

export interface LoggedAction {
  path: string;
  method: CleanupMethod;
  bytes: number;
  status: ActionStatus;
  restorable: boolean;
  errorCode: string | null;
}

export interface HistoryEntry {
  runId: number;
  startedAt: number;
  finishedAt: number;
  dryRun: boolean;
  bytesFreed: number;
  actions: LoggedAction[];
}

// ---------- AI ----------

export type AiProvider = "open_ai" | "gemini" | "anthropic" | "groq";

export interface AiProviderStatus {
  provider: AiProvider;
  hasKey: boolean;
  model: string;
  availableModels: string[];
}

export interface AiAnswer {
  path: string;
  what: string;
  whyBig: string;
  safety: SafetyLevel;
  consequence: string;
  language: string;
  provider: AiProvider;
  model: string;
  cached: boolean;
  createdAt: number;
}

// ---------- snapshots ----------

export interface SnapshotInfo {
  id: number;
  rootPath: string;
  takenAt: number;
  totalBytes: number;
  files: number;
  driveTotal: number;
  driveFree: number;
}

export interface GrowthItem {
  path: string;
  before: number;
  after: number;
  delta: number;
  explanation: Explanation | null;
}

export interface SnapshotComparison {
  from: SnapshotInfo;
  to: SnapshotInfo;
  totalDelta: number;
  items: GrowthItem[];
}

export interface GrowthPoint {
  takenAt: number;
  bytes: number;
}

// ---------- settings ----------

export type Language = "fa" | "en";
export type ThemePref = "system" | "light" | "dark";
export type UiMode = "simple" | "expert";

export interface AppSettings {
  language: Language;
  theme: ThemePref;
  defaultMode: UiMode;
  excludedPaths: string[];
  staleMonths: number;
  oldProjectMonths: number;
  aiEnabled: boolean;
  aiMaskNames: boolean;
  aiDefaultProvider: AiProvider | null;
  aiPreviewAcknowledged: boolean;
  lowSpaceThresholdGb: number;
  trayEnabled: boolean;
  weeklyCheck: boolean;
  onboardingDone: boolean;
  devSandbox: string | null;
}

export interface AppInfo {
  version: string;
  dataDir: string;
  isElevated: boolean;
  debugBuild: boolean;
}
