// Fake backend for `pnpm dev` in a normal browser. Implements every command
// from commands.ts with believable data and emits the same events.
import type { Backend } from "../client";
import { emitLocal, EVENTS } from "../events";
import type {
  ActionResult,
  AiAnswer,
  AiProvider,
  AiProviderStatus,
  AppInfo,
  AppSettings,
  CleanupAction,
  CleanupOptions,
  CleanupPlan,
  CleanupReport,
  CleanupTarget,
  DriveInfo,
  FallbackReason,
  GrowthPoint,
  HistoryEntry,
  NodeId,
  PlanWarning,
  ScanId,
  ScanMode,
  ScanSummary,
  SnapshotComparison,
  SnapshotInfo,
  Story,
} from "../types";
import { C_DRIVE_SPEC, D_DRIVE_SPEC, DRIVES, E_DRIVE_SPEC, HIDDEN_USED, PROJECTS_PATH, PROJECTS_SPEC } from "./data";
import { heuristicFindings, scaleGrowth, seedHistory, seedSnapshots } from "./extras";
import { GB, MB, MockTree } from "./tree";

const DAY = 86_400_000;
const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
const clone = <T>(v: T): T => structuredClone(v);
const fail = (code: string, detail: string | null = null) => Promise.reject({ code, detail });

// ---------- trees and drives ----------

const trees = new Map<string, MockTree>();

function specFor(root: string) {
  const key = root.toLowerCase();
  if (key === "c:\\") return C_DRIVE_SPEC;
  if (key === "d:\\") return D_DRIVE_SPEC;
  if (key === "e:\\") return E_DRIVE_SPEC;
  if (key === PROJECTS_PATH.toLowerCase()) return PROJECTS_SPEC;
  return null;
}

function treeFor(root: string): MockTree | null {
  const key = root.toLowerCase();
  let t = trees.get(key);
  if (!t) {
    const spec = specFor(root);
    if (!spec) return null;
    t = new MockTree(root, spec);
    trees.set(key, t);
  }
  return t;
}

const drives: DriveInfo[] = DRIVES.map((d) => {
  const t = treeFor(d.root);
  const used = (t ? t.get(t.root).size : 0) + (HIDDEN_USED[d.letter] ?? 0);
  return { ...d, free: Math.max(0, d.total - used) };
});

function driveOf(path: string): DriveInfo {
  const letter = path.slice(0, 2).toUpperCase();
  return drives.find((d) => d.letter === letter) ?? drives[0];
}

// ---------- scans ----------

interface ScanState {
  tree: MockTree;
  cancelled: boolean;
  summary: ScanSummary | null;
  snapshotId: number | null;
}

let nextScanId = 1;
let nextJobId = 1;
const scans = new Map<ScanId, ScanState>();

function scan(scanId: ScanId): ScanState {
  const s = scans.get(scanId);
  if (!s) throw { code: "not_found", detail: `scan ${scanId}` };
  return s;
}

async function runFakeScan(scanId: ScanId, state: ScanState, mode: ScanMode, fallback: FallbackReason | null) {
  const scanner: ScanMode = fallback ? "normal" : mode;
  const tree = state.tree;
  const root = tree.get(tree.root);
  const paths = tree.allPaths();
  const started = performance.now();
  const duration = scanner === "fast" ? 1800 : 4200;
  if (fallback === "uac_refused") await wait(700);
  let elapsed = 0;
  while (elapsed < duration) {
    await wait(90);
    if (state.cancelled) {
      emitLocal(EVENTS.scanCancelled, { scanId });
      return;
    }
    elapsed = performance.now() - started;
    const p = Math.min(1, elapsed / duration);
    const eased = 1 - (1 - p) ** 2;
    emitLocal(EVENTS.scanProgress, {
      scanId,
      files: Math.floor(root.fileCount * eased),
      dirs: Math.floor(root.dirCount * eased),
      bytes: Math.floor(root.size * eased),
      currentPath: paths[Math.floor(eased * (paths.length - 1))],
      elapsedMs: Math.round(elapsed),
      scanner,
    });
  }
  const drive = driveOf(tree.rootPath);
  const denied = tree.accessDenied().length;
  let cloud = 0;
  for (const n of tree.nodes.values()) if (n.flags.cloudOnly) cloud++;
  const summary: ScanSummary = {
    scanId,
    rootPath: tree.rootPath,
    rootNode: tree.root,
    totalBytes: root.size,
    files: root.fileCount,
    dirs: root.dirCount,
    accessDenied: denied,
    cloudOnly: cloud,
    durationMs: Math.round(performance.now() - started),
    scanner,
    fallbackReason: fallback,
    driveTotal: drive.total,
    driveFree: drive.free,
    finishedAt: Date.now(),
  };
  state.summary = summary;
  addScanSnapshot(state);
  emitLocal(EVENTS.scanDone, clone(summary));
}

// ---------- snapshots ----------

let nextSnapshotId = 1;
const snapshots: SnapshotInfo[] = [];
{
  const c = treeFor("C:\\");
  const drive = driveOf("C:");
  if (c) {
    const root = c.get(c.root);
    for (const s of seedSnapshots("C:\\", root.size - 12 * GB, root.fileCount, drive.total, drive.free + 12 * GB)) {
      snapshots.push({ ...s, id: nextSnapshotId++ });
    }
  }
}

function addScanSnapshot(state: ScanState) {
  const s = state.summary;
  if (!s) return;
  const snap: SnapshotInfo = {
    id: nextSnapshotId++,
    rootPath: s.rootPath,
    takenAt: s.finishedAt,
    totalBytes: s.totalBytes,
    files: s.files,
    driveTotal: s.driveTotal,
    driveFree: s.driveFree,
  };
  snapshots.push(snap);
  state.snapshotId = snap.id;
}

function compare(from: SnapshotInfo, to: SnapshotInfo): SnapshotComparison {
  const c = treeFor("C:\\");
  const isC = from.rootPath.toLowerCase() === "c:\\";
  const factor = (to.takenAt - from.takenAt) / (30 * DAY);
  const items =
    isC && c
      ? scaleGrowth(
          (to.totalBytes - from.totalBytes) / (12 * GB) || factor,
          (p) => c.findByPath(p)?.explanation ?? null,
          (p) => c.findByPath(p)?.size ?? 0,
        )
      : [];
  const totalDelta = to.totalBytes - from.totalBytes;
  return { from, to, totalDelta, items };
}

// ---------- settings ----------

const SETTINGS_KEY = "fazasanj-mock-settings";

const DEFAULT_SETTINGS: AppSettings = {
  language: "fa",
  theme: "system",
  defaultMode: "simple",
  excludedPaths: [],
  staleMonths: 12,
  oldProjectMonths: 6,
  aiEnabled: false,
  aiMaskNames: true,
  aiDefaultProvider: null,
  aiPreviewAcknowledged: false,
  lowSpaceThresholdGb: 10,
  trayEnabled: false,
  weeklyCheck: false,
  onboardingDone: false,
  devSandbox: null,
};

function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (raw) return { ...DEFAULT_SETTINGS, ...(JSON.parse(raw) as Partial<AppSettings>) };
  } catch {
    // storage blocked, fall back to defaults
  }
  return { ...DEFAULT_SETTINGS };
}

let settings = loadSettings();

function saveSettings() {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // not important for the mock
  }
}

// ---------- AI ----------

const MODELS: Record<AiProvider, string[]> = {
  open_ai: ["gpt-5-mini", "gpt-5", "gpt-4.1-mini"],
  gemini: ["gemini-2.5-flash", "gemini-2.5-pro"],
  anthropic: ["claude-haiku-4-5", "claude-sonnet-4-5"],
  groq: ["llama-3.3-70b-versatile", "openai/gpt-oss-120b"],
};

const providers = new Map<AiProvider, { key: string | null; model: string }>(
  (Object.keys(MODELS) as AiProvider[]).map((p) => [p, { key: null, model: MODELS[p][0] }]),
);

function providerStatus(): AiProviderStatus[] {
  return [...providers.entries()].map(([provider, v]) => ({
    provider,
    hasKey: v.key != null,
    model: v.model,
    availableModels: MODELS[provider],
  }));
}

function keyProblem(key: string | null): string | null {
  if (!key) return "invalid_key";
  if (key.includes("bad")) return "invalid_key";
  if (key.includes("limit")) return "rate_limited";
  if (key.includes("offline")) return "no_internet";
  return null;
}

// ---------- cleanup ----------

const plans = new Map<number, CleanupPlan>();
let nextPlanId = 1;
const history: HistoryEntry[] = seedHistory();
let nextRunId = 3;

const PROTECTED = ["c:\\windows\\system32", "c:\\windows\\winsxs\\manifests", "c:\\program files\\windowsapps"];

const COMMANDS: Record<string, string> = {
  "windows-winsxs": "dism.exe /Online /Cleanup-Image /StartComponentCleanup",
  hiberfil: "powercfg.exe /hibernate off",
  "windows-old": "cleanmgr.exe /d C: /sagerun:64",
  "recycle-bin": "SHEmptyRecycleBin",
};

function planFor(targets: CleanupTarget[]): CleanupPlan {
  const actions: CleanupAction[] = targets.map((t, index) => {
    const e = t.explanation;
    const lower = t.path.toLowerCase();
    const blocked = e.safety === "do_not_touch" || PROTECTED.some((p) => lower === p || lower.startsWith(`${p}\\`));
    return {
      index,
      path: t.path,
      title: e.title,
      method: e.method,
      bytes: t.bytes,
      safety: e.safety,
      consequence: e.ifDeleted,
      needsAdmin: e.needsAdmin,
      permanent: e.method === "command",
      instructions: e.instructions,
      command: e.method === "command" ? (COMMANDS[e.ruleId] ?? null) : null,
      blocked,
    };
  });
  const warnings: PlanWarning[] = [];
  const live = actions.filter((a) => !a.blocked);
  if (live.some((a) => a.method === "recycle" || a.method === "delete_contents")) warnings.push({ code: "recycle_same_drive", path: null });
  if (live.some((a) => a.needsAdmin)) warnings.push({ code: "needs_admin", path: null });
  if (live.some((a) => a.method === "command")) warnings.push({ code: "system_action", path: null });
  for (const a of actions) if (a.blocked) warnings.push({ code: "blocked", path: a.path });
  return {
    planId: nextPlanId++,
    actions,
    totalBytes: live.reduce((s, a) => s + a.bytes, 0),
    needsAdmin: live.some((a) => a.needsAdmin),
    hasSystemActions: live.some((a) => a.method === "command"),
    warnings,
  };
}

function resultFor(a: CleanupAction, opts: CleanupOptions): ActionResult {
  const base: ActionResult = {
    index: a.index,
    path: a.path,
    method: a.method,
    status: "done",
    bytesFreed: 0,
    filesRemoved: 0,
    filesSkipped: 0,
    skippedPaths: [],
    error: null,
    wouldRemove: [],
  };
  if (a.blocked) return { ...base, status: "blocked", error: { code: "blocked_protected", detail: null } };
  const t = treeFor(`${a.path.slice(0, 2)}\\`);
  const node = t?.findByPath(a.path);
  const files = node?.fileCount ?? 1;
  if (opts.dryRun) {
    const sample = t ? t.sampleChildren(a.path, 5) : [a.path];
    const removable = a.method === "recycle" || a.method === "delete_contents" || a.method === "command" || a.method === "compact_vhdx";
    return {
      ...base,
      status: "dry_run",
      bytesFreed: removable ? a.bytes : 0,
      filesRemoved: removable ? files : 0,
      wouldRemove: removable ? (a.method === "command" ? [a.command ?? a.path] : sample) : [],
    };
  }
  switch (a.method) {
    case "recycle":
      return { ...base, bytesFreed: a.bytes, filesRemoved: files };
    case "delete_contents":
      if (a.path.toLowerCase().includes("chrome")) {
        const skipped = [`${a.path}\\Cache_Data\\data_0`, `${a.path}\\Cache_Data\\data_1`, `${a.path}\\Cache_Data\\index`];
        return { ...base, status: "partial", bytesFreed: a.bytes - 38 * MB, filesRemoved: files - 3, filesSkipped: 3, skippedPaths: skipped, error: { code: "in_use", detail: null } };
      }
      return { ...base, bytesFreed: a.bytes, filesRemoved: files };
    case "command":
      return { ...base, bytesFreed: Math.round(a.bytes * 0.7) };
    case "open_app_setting":
      return { ...base, status: "opened_setting" };
    case "compact_vhdx":
      return { ...base, status: "failed", error: { code: "vhdx_in_use", detail: "Docker Desktop is running" } };
    case "manual_only":
      return { ...base, status: "needs_manual" };
  }
}

async function runFakeCleanup(jobId: number, plan: CleanupPlan, opts: CleanupOptions) {
  const drive = driveOf(plan.actions[0]?.path ?? "C:");
  const freeBefore = drive.free;
  const startedAt = Date.now();
  const results: ActionResult[] = [];
  let freed = 0;
  if (opts.createRestorePoint && !opts.dryRun) await wait(900);
  for (const a of plan.actions) {
    emitLocal(EVENTS.cleanupProgress, { jobId, index: a.index, total: plan.actions.length, currentPath: a.path, bytesFreed: freed });
    await wait(opts.dryRun ? 220 : 650);
    const r = resultFor(a, opts);
    results.push(r);
    freed += r.bytesFreed;
  }
  // moving to the Recycle Bin on the same drive frees nothing until it is emptied
  const reallyFreed = opts.dryRun
    ? 0
    : results.reduce((s, r) => {
        const a = plan.actions[r.index];
        const permanent = a.permanent || (opts.permanentForSafe && a.safety === "safe" && a.method === "delete_contents");
        return s + (permanent ? r.bytesFreed : 0);
      }, 0);
  drive.free += reallyFreed;
  const report: CleanupReport = {
    jobId,
    runId: nextRunId++,
    dryRun: opts.dryRun,
    freeBefore,
    freeAfter: freeBefore + reallyFreed,
    bytesFreed: opts.dryRun ? 0 : freed,
    restorePointCreated: opts.createRestorePoint && !opts.dryRun ? true : null,
    results,
    startedAt,
    finishedAt: Date.now(),
  };
  history.unshift({
    runId: report.runId,
    startedAt,
    finishedAt: report.finishedAt,
    dryRun: opts.dryRun,
    bytesFreed: report.bytesFreed,
    actions: results.map((r) => {
      const a = plan.actions[r.index];
      const permanent = a.permanent || (opts.permanentForSafe && a.safety === "safe" && a.method === "delete_contents");
      return {
        path: r.path,
        method: r.method,
        bytes: a.bytes,
        status: r.status,
        restorable: !opts.dryRun && (r.status === "done" || r.status === "partial") && !permanent && (a.method === "recycle" || a.method === "delete_contents"),
        errorCode: r.error?.code ?? null,
      };
    }),
  });
  emitLocal(EVENTS.cleanupDone, clone(report));
}

// ---------- the backend ----------

export const mockBackend: Backend = {
  async listDrives() {
    await wait(250);
    return clone(drives);
  },

  async startScan(path: string, mode: ScanMode) {
    await wait(120);
    const tree = treeFor(path);
    if (!tree) return fail("not_found", path);
    const drive = driveOf(path);
    let fallback: FallbackReason | null = null;
    if (mode === "fast" && drive.filesystem !== "NTFS") fallback = "not_ntfs";
    else if (mode === "fast" && drive.letter === "D:") fallback = "uac_refused";
    const scanId = nextScanId++;
    const state: ScanState = { tree, cancelled: false, summary: null, snapshotId: null };
    scans.set(scanId, state);
    void runFakeScan(scanId, state, mode, fallback);
    return scanId;
  },

  async cancelScan(scanId: ScanId) {
    scan(scanId).cancelled = true;
  },

  async getScanSummary(scanId: ScanId) {
    const s = scan(scanId).summary;
    if (!s) return fail("not_found", "scan still running");
    return clone(s);
  },

  async getNode(scanId: ScanId, nodeId: NodeId) {
    return scan(scanId).tree.info(nodeId);
  },

  async getChildren(scanId, nodeId, sort, offset, limit) {
    await wait(60);
    return scan(scanId).tree.children(nodeId, sort, offset, limit);
  },

  async getTreemap(scanId, nodeId, depth, maxItems) {
    await wait(80);
    return scan(scanId).tree.treemap(nodeId, depth, maxItems);
  },

  async getLargestFiles(scanId, limit) {
    await wait(150);
    return scan(scanId).tree.largestFiles(limit);
  },

  async getByType(scanId) {
    await wait(150);
    return scan(scanId).tree.byType();
  },

  async getAccessDenied(scanId) {
    await wait(80);
    return scan(scanId).tree.accessDenied();
  },

  async getStory(scanId): Promise<Story> {
    await wait(200);
    const s = scan(scanId);
    const t = s.tree;
    const drive = driveOf(t.rootPath);
    const explained = t.explained();
    const safeItems = explained.filter((r) => r.explanation.safety === "safe");
    return clone({
      scanId,
      rootPath: t.rootPath,
      driveTotal: drive.total,
      driveFree: drive.free,
      countedBytes: t.get(t.root).size,
      buckets: t.buckets(),
      reasons: explained.slice(0, 5),
      safeItems,
      safeBytes: safeItems.reduce((a, r) => a + r.bytes, 0),
      needsDecision: explained.filter((r) => r.explanation.safety === "probably_safe" || r.explanation.safety === "careful").slice(0, 12),
    });
  },

  async compareScanners(path: string) {
    await wait(2500);
    const t = treeFor(path);
    const bytes = t ? t.get(t.root).size : 100 * GB;
    const files = t ? t.get(t.root).fileCount : 100_000;
    return {
      normalBytes: bytes,
      fastBytes: bytes + 14 * MB,
      normalFiles: files,
      fastFiles: files + 212,
      normalMs: 38_400,
      fastMs: 4_100,
      differences: [
        { path: "C:\\System Volume Information", normalBytes: 0, fastBytes: 7.9 * GB },
        { path: "C:\\Windows\\CSC", normalBytes: 0, fastBytes: 12 * MB },
      ],
    };
  },

  async runHeuristics(scanId, kinds) {
    const s = scan(scanId);
    const jobId = nextJobId++;
    void (async () => {
      let done = 0;
      for (const kind of kinds) {
        for (let step = 1; step <= 4; step++) {
          await wait(kind === "duplicates" ? 420 : 220);
          done++;
          emitLocal(EVENTS.heuristicsProgress, { jobId, kind, done, total: kinds.length * 4 });
        }
      }
      emitLocal(EVENTS.heuristicsDone, { jobId, scanId, findings: heuristicFindings(s.tree, kinds, settings.staleMonths) });
    })();
    return jobId;
  },

  async buildCleanupPlan(targets) {
    await wait(300);
    if (targets.length === 0) return fail("invalid_plan", "empty");
    const plan = planFor(targets);
    plans.set(plan.planId, plan);
    return clone(plan);
  },

  async runCleanup(planId, options) {
    const plan = plans.get(planId);
    if (!plan) return fail("not_found", `plan ${planId}`);
    const jobId = nextJobId++;
    void runFakeCleanup(jobId, plan, options);
    return jobId;
  },

  async getCleanupHistory(limit, offset) {
    await wait(150);
    return clone(history.slice(offset, offset + limit));
  },

  async revealInExplorer(path) {
    console.info("[mock] reveal in Explorer:", path);
  },

  async openRecycleBin() {
    console.info("[mock] open Recycle Bin");
  },

  async openAppSetting(target) {
    console.info("[mock] open app setting:", target);
  },

  async aiGetProviders() {
    await wait(100);
    return providerStatus();
  },

  async aiSetKey(provider, key) {
    await wait(200);
    const p = providers.get(provider);
    if (p) p.key = key;
  },

  async aiDeleteKey(provider) {
    const p = providers.get(provider);
    if (p) p.key = null;
  },

  async aiTestKey(provider) {
    await wait(900);
    const problem = keyProblem(providers.get(provider)?.key ?? null);
    if (problem) return fail(problem);
  },

  async aiSetModel(provider, model) {
    const p = providers.get(provider);
    if (p) p.model = model;
  },

  async aiPreviewPayload(scanId, nodeId) {
    const n = scan(scanId).tree.info(nodeId);
    const mask = (p: string) => (settings.aiMaskNames ? p.replace(/\\Users\\[^\\]+/i, "\\Users\\<user>") : p);
    const kids = scan(scanId).tree.children(nodeId, "size", 0, 8).items;
    return {
      path: mask(n.path),
      isFolder: n.isDir,
      sizeBytes: n.size,
      fileCount: n.fileCount,
      lastModified: n.modified ? new Date(n.modified).toISOString() : null,
      largestChildren: kids.map((k) => ({ name: k.name, sizeBytes: k.size })),
      answerLanguage: settings.language,
      note: "Metadata only. No file contents are sent.",
    };
  },

  async aiExplain(scanId, nodeId, language) {
    const n = scan(scanId).tree.info(nodeId);
    if (!settings.aiEnabled) return fail("ai_disabled");
    const provider = settings.aiDefaultProvider ?? [...providers.entries()].find(([, v]) => v.key)?.[0] ?? null;
    if (!provider) return fail("invalid_key");
    const p = providers.get(provider);
    await wait(1500);
    const problem = keyProblem(p?.key ?? null);
    if (problem) return fail(problem);
    const fa = language === "fa";
    const answer: AiAnswer = {
      path: n.path,
      what: fa
        ? `به نظر می‌رسد «${n.name}» پوشهٔ به‌روزرسانی یک برنامهٔ جانبی باشد که نسخه‌های قدیمی نصب‌کننده را نگه می‌دارد.`
        : `"${n.name}" looks like an app's update folder that keeps old installer versions.`,
      whyBig: fa
        ? "هر بار که برنامه به‌روز می‌شود، فایل نصب جدید اینجا دانلود می‌شود و نسخه‌های قبلی پاک نمی‌شوند."
        : "Each update downloads a new installer here and the old ones are never removed.",
      safety: "probably_safe",
      consequence: fa
        ? "احتمالاً خود برنامه آسیبی نمی‌بیند، ولی اگر بخواهید به نسخهٔ قبلی برگردید، این فایل‌ها دیگر نیستند."
        : "The app itself is probably fine, but you can't roll back to an older version.",
      language,
      provider,
      model: p?.model ?? "",
      cached: false,
      createdAt: Date.now(),
    };
    return answer;
  },

  async listSnapshots(rootPath) {
    await wait(120);
    const list = rootPath ? snapshots.filter((s) => s.rootPath.toLowerCase() === rootPath.toLowerCase()) : snapshots;
    return clone([...list].sort((a, b) => b.takenAt - a.takenAt));
  },

  async compareSnapshots(fromId, toId) {
    await wait(200);
    const from = snapshots.find((s) => s.id === fromId);
    const to = snapshots.find((s) => s.id === toId);
    if (!from || !to) return fail("not_found");
    return clone(compare(from, to));
  },

  async compareWithLast(scanId) {
    await wait(150);
    const s = scan(scanId);
    const current = snapshots.find((x) => x.id === s.snapshotId);
    if (!current) return null;
    const prev = snapshots
      .filter((x) => x.rootPath.toLowerCase() === current.rootPath.toLowerCase() && x.takenAt < current.takenAt)
      .sort((a, b) => b.takenAt - a.takenAt)[0];
    if (!prev) return null;
    return clone(compare(prev, current));
  },

  async getGrowth(path): Promise<GrowthPoint[]> {
    await wait(150);
    const t = treeFor(`${path.slice(0, 2)}\\`);
    const now = t?.findByPath(path)?.size ?? GB;
    let h = 0;
    for (const ch of path) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
    const points: GrowthPoint[] = [];
    let v = now;
    for (let i = 0; i < 8; i++) {
      points.unshift({ takenAt: Date.now() - i * 30 * DAY, bytes: Math.max(0, Math.round(v)) });
      const step = ((h >> i) & 7) / 7;
      v = v * (0.82 + 0.14 * step);
    }
    return points;
  },

  async getSettings() {
    await wait(50);
    return clone(settings);
  },

  async setSettings(next: AppSettings) {
    settings = clone(next);
    saveSettings();
    return clone(settings);
  },

  async resetEverything() {
    await wait(400);
    settings = { ...DEFAULT_SETTINGS };
    try {
      localStorage.removeItem(SETTINGS_KEY);
    } catch {
      // ignore
    }
    for (const p of providers.values()) p.key = null;
  },

  async checkUpdate() {
    await wait(300);
    return null;
  },
  async installUpdate() {
    await wait(300);
  },
  async getAppInfo(): Promise<AppInfo> {
    return {
      version: "0.1.0",
      dataDir: "C:\\Users\\Ali\\AppData\\Roaming\\com.fazasanj.app",
      isElevated: false,
      debugBuild: true,
    };
  },
};
