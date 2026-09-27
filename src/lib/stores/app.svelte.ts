// App wide state: settings, drives, navigation, running scans and background jobs.

import { listen } from "@tauri-apps/api/event";
import * as api from "../api/commands";
import { EVENTS } from "../api/commands";
import type {
  ApiError,
  AppInfo,
  AppSettings,
  CleanupProgress,
  CleanupReport,
  CleanupTarget,
  DriveInfo,
  HeuristicFinding,
  HeuristicKind,
  HeuristicsDone,
  HeuristicsProgress,
  JobId,
  ScanFailed,
  ScanId,
  ScanMode,
  ScanProgress,
  ScanSummary,
  UiMode,
} from "../api/types";
import { setLang } from "../i18n/index.svelte";
import { applyTheme } from "../theme/theme";

export const DEFAULT_SETTINGS: AppSettings = {
  language: "fa",
  theme: "system",
  defaultMode: "simple",
  excludedPaths: [],
  staleMonths: 12,
  oldProjectMonths: 6,
  aiEnabled: false,
  aiMaskNames: false,
  aiDefaultProvider: null,
  aiPreviewAcknowledged: false,
  lowSpaceThresholdGb: 10,
  trayEnabled: false,
  weeklyCheck: false,
  onboardingDone: false,
  devSandbox: null,
};

export type View =
  | { kind: "home" }
  | { kind: "target"; root: string }
  | { kind: "history" }
  | { kind: "growth" }
  | { kind: "settings" }
  | { kind: "about" };

export type ScanStatus = "scanning" | "done" | "cancelled" | "error";

export interface ScanSession {
  root: string;
  scanId: ScanId;
  mode: ScanMode;
  status: ScanStatus;
  progress: ScanProgress | null;
  summary: ScanSummary | null;
  error: ApiError | null;
  startedAt: number;
}

export interface HeuristicsState {
  jobId: JobId;
  running: boolean;
  kinds: HeuristicKind[];
  progress: HeuristicsProgress | null;
  findings: HeuristicFinding[];
}

export interface CleanupState {
  targets: CleanupTarget[];
  jobId: JobId | null;
  progress: CleanupProgress | null;
  report: CleanupReport | null;
}

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "ok" | "error";
}

const key = (root: string) => root.toLowerCase().replace(/[\\/]+$/, "");

class AppState {
  ready = $state(false);
  settings = $state<AppSettings>({ ...DEFAULT_SETTINGS });
  info = $state<AppInfo | null>(null);
  drives = $state<DriveInfo[]>([]);
  drivesLoading = $state(true);
  drivesError = $state<ApiError | null>(null);
  view = $state<View>({ kind: "home" });
  mode = $state<UiMode>("simple");
  sessions = $state<Record<string, ScanSession>>({});
  heuristics = $state<Record<ScanId, HeuristicsState>>({});
  cleanup = $state<CleanupState | null>(null);
  toasts = $state<Toast[]>([]);
  /** Bumped after a cleanup so views can offer a rescan. */
  cleanupRuns = $state(0);
  private toastId = 0;

  session(root: string | null | undefined): ScanSession | null {
    if (!root) return null;
    return this.sessions[key(root)] ?? null;
  }

  sessionById(scanId: ScanId): ScanSession | null {
    return Object.values(this.sessions).find((s) => s.scanId === scanId) ?? null;
  }

  toast(text: string, kind: Toast["kind"] = "info") {
    const id = ++this.toastId;
    this.toasts = [...this.toasts, { id, text, kind }];
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), 5000);
  }

  go(view: View) {
    this.view = view;
  }

  async init() {
    await this.loadSettings();
    this.listenAll();
    await this.refreshDrives();
    api.getAppInfo().then((i) => (this.info = i)).catch(() => {});
    this.ready = true;
  }

  async loadSettings() {
    try {
      this.settings = { ...DEFAULT_SETTINGS, ...(await api.getSettings()) };
    } catch {
      this.settings = { ...DEFAULT_SETTINGS };
    }
    this.mode = this.settings.defaultMode;
    setLang(this.settings.language);
    applyTheme(this.settings.theme);
  }

  async saveSettings(patch: Partial<AppSettings>) {
    const next = { ...this.settings, ...patch };
    this.settings = next;
    setLang(next.language);
    applyTheme(next.theme);
    try {
      this.settings = await api.setSettings(next);
    } catch (e) {
      console.error(e);
    }
  }

  async refreshDrives() {
    this.drivesLoading = true;
    try {
      this.drives = await api.listDrives();
      this.drivesError = null;
    } catch (e) {
      this.drivesError = e as ApiError;
    } finally {
      this.drivesLoading = false;
    }
  }

  async startScan(root: string, mode: ScanMode) {
    const old = this.session(root);
    if (old?.status === "scanning") return;
    const scanId = await api.startScan(root, mode);
    this.sessions[key(root)] = {
      root,
      scanId,
      mode,
      status: "scanning",
      progress: null,
      summary: null,
      error: null,
      startedAt: Date.now(),
    };
    this.view = { kind: "target", root };
  }

  async cancelScan(root: string) {
    const s = this.session(root);
    if (s && s.status === "scanning") await api.cancelScan(s.scanId);
  }

  async runHeuristics(scanId: ScanId, kinds: HeuristicKind[]) {
    const jobId = await api.runHeuristics(scanId, kinds);
    this.heuristics[scanId] = { jobId, running: true, kinds, progress: null, findings: [] };
  }

  openCleanup(targets: CleanupTarget[]) {
    if (targets.length === 0) return;
    this.cleanup = { targets, jobId: null, progress: null, report: null };
  }

  closeCleanup() {
    this.cleanup = null;
  }

  private listenAll() {
    listen<ScanProgress>(EVENTS.scanProgress, (e) => {
      const s = this.sessionById(e.payload.scanId);
      if (s && s.status === "scanning") s.progress = e.payload;
    });
    listen<ScanSummary>(EVENTS.scanDone, (e) => {
      const s = this.sessionById(e.payload.scanId);
      if (!s) return;
      s.status = "done";
      s.summary = e.payload;
      this.refreshDrives();
    });
    listen<{ scanId: ScanId }>(EVENTS.scanCancelled, (e) => {
      const s = this.sessionById(e.payload.scanId);
      if (s) s.status = "cancelled";
    });
    listen<ScanFailed>(EVENTS.scanError, (e) => {
      const s = this.sessionById(e.payload.scanId);
      if (!s) return;
      s.status = "error";
      s.error = e.payload.error;
    });
    listen<HeuristicsProgress>(EVENTS.heuristicsProgress, (e) => {
      const h = Object.values(this.heuristics).find((x) => x.jobId === e.payload.jobId);
      if (h) h.progress = e.payload;
    });
    listen<HeuristicsDone>(EVENTS.heuristicsDone, (e) => {
      const h = this.heuristics[e.payload.scanId];
      if (h && h.jobId === e.payload.jobId) {
        h.running = false;
        h.findings = e.payload.findings.sort((a, b) => b.bytes - a.bytes);
      }
    });
    listen<CleanupProgress>(EVENTS.cleanupProgress, (e) => {
      if (this.cleanup && this.cleanup.jobId === e.payload.jobId) this.cleanup.progress = e.payload;
    });
    listen<CleanupReport>(EVENTS.cleanupDone, (e) => {
      if (this.cleanup && this.cleanup.jobId === e.payload.jobId) this.cleanup.report = e.payload;
      if (!e.payload.dryRun) {
        this.cleanupRuns++;
        this.refreshDrives();
      }
    });
  }
}

export const app = new AppState();
