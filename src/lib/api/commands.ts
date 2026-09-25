// Typed wrappers around every Tauri command. This is the only file that calls invoke().
// Command names and argument names must match src-tauri/src/commands/*.rs.

import { invoke } from "@tauri-apps/api/core";
import type {
  AccessDeniedEntry,
  AiAnswer,
  AiProvider,
  AiProviderStatus,
  AppInfo,
  AppSettings,
  ChildSort,
  ChildrenPage,
  CleanupOptions,
  CleanupPlan,
  CleanupTarget,
  DriveInfo,
  FileEntry,
  GrowthPoint,
  HeuristicKind,
  HistoryEntry,
  JobId,
  NodeId,
  NodeInfo,
  ScanId,
  ScanMode,
  ScanSummary,
  ScannerComparison,
  SnapshotComparison,
  SnapshotInfo,
  Story,
  TreemapNode,
  TypeGroup,
} from "./types";

// drives and scanning
export const listDrives = () => invoke<DriveInfo[]>("list_drives");
export const startScan = (path: string, mode: ScanMode) => invoke<ScanId>("start_scan", { path, mode });
export const cancelScan = (scanId: ScanId) => invoke<void>("cancel_scan", { scanId });
export const getScanSummary = (scanId: ScanId) => invoke<ScanSummary>("get_scan_summary", { scanId });
export const getNode = (scanId: ScanId, nodeId: NodeId) => invoke<NodeInfo>("get_node", { scanId, nodeId });
export const getChildren = (scanId: ScanId, nodeId: NodeId, sort: ChildSort, offset: number, limit: number) =>
  invoke<ChildrenPage>("get_children", { scanId, nodeId, sort, offset, limit });
export const getTreemap = (scanId: ScanId, nodeId: NodeId, depth: number, maxItems: number) =>
  invoke<TreemapNode>("get_treemap", { scanId, nodeId, depth, maxItems });
export const getLargestFiles = (scanId: ScanId, limit: number) =>
  invoke<FileEntry[]>("get_largest_files", { scanId, limit });
export const getByType = (scanId: ScanId) => invoke<TypeGroup[]>("get_by_type", { scanId });
export const getAccessDenied = (scanId: ScanId) => invoke<AccessDeniedEntry[]>("get_access_denied", { scanId });
export const getStory = (scanId: ScanId) => invoke<Story>("get_story", { scanId });
export const compareScanners = (path: string) => invoke<ScannerComparison>("compare_scanners", { path });

// heuristics (results arrive through the heuristics://done event)
export const runHeuristics = (scanId: ScanId, kinds: HeuristicKind[]) =>
  invoke<JobId>("run_heuristics", { scanId, kinds });

// cleanup (report arrives through the cleanup://done event)
export const buildCleanupPlan = (targets: CleanupTarget[]) => invoke<CleanupPlan>("build_cleanup_plan", { targets });
export const runCleanup = (planId: number, options: CleanupOptions) =>
  invoke<JobId>("run_cleanup", { planId, options });
export const getCleanupHistory = (limit: number, offset: number) =>
  invoke<HistoryEntry[]>("get_cleanup_history", { limit, offset });
export const revealInExplorer = (path: string) => invoke<void>("reveal_in_explorer", { path });
export const openRecycleBin = () => invoke<void>("open_recycle_bin");
export const openAppSetting = (target: string) => invoke<void>("open_app_setting", { target });

// AI
export const aiGetProviders = () => invoke<AiProviderStatus[]>("ai_get_providers");
export const aiSetKey = (provider: AiProvider, key: string) => invoke<void>("ai_set_key", { provider, key });
export const aiDeleteKey = (provider: AiProvider) => invoke<void>("ai_delete_key", { provider });
export const aiTestKey = (provider: AiProvider) => invoke<void>("ai_test_key", { provider });
export const aiSetModel = (provider: AiProvider, model: string) => invoke<void>("ai_set_model", { provider, model });
export const aiPreviewPayload = (scanId: ScanId, nodeId: NodeId) =>
  invoke<unknown>("ai_preview_payload", { scanId, nodeId });
export const aiExplain = (scanId: ScanId, nodeId: NodeId, language: "fa" | "en") =>
  invoke<AiAnswer>("ai_explain", { scanId, nodeId, language });

// snapshots and growth
export const listSnapshots = (rootPath: string | null) => invoke<SnapshotInfo[]>("list_snapshots", { rootPath });
export const compareSnapshots = (fromId: number, toId: number) =>
  invoke<SnapshotComparison>("compare_snapshots", { fromId, toId });
export const compareWithLast = (scanId: ScanId) =>
  invoke<SnapshotComparison | null>("compare_with_last", { scanId });
export const getGrowth = (path: string) => invoke<GrowthPoint[]>("get_growth", { path });

// settings and app
export const getSettings = () => invoke<AppSettings>("get_settings");
export const setSettings = (settings: AppSettings) => invoke<AppSettings>("set_settings", { settings });
export const resetEverything = () => invoke<void>("reset_everything");
export const getAppInfo = () => invoke<AppInfo>("get_app_info");

/** Event names emitted by the backend. */
export const EVENTS = {
  scanProgress: "scan://progress", // ScanProgress
  scanDone: "scan://done", // ScanSummary
  scanCancelled: "scan://cancelled", // { scanId }
  scanError: "scan://error", // ScanFailed
  heuristicsProgress: "heuristics://progress", // HeuristicsProgress
  heuristicsDone: "heuristics://done", // HeuristicsDone
  cleanupProgress: "cleanup://progress", // CleanupProgress
  cleanupDone: "cleanup://done", // CleanupReport
} as const;
