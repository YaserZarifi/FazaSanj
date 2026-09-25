// Fixed fake data for heuristics, snapshots and history.
import type {
  GrowthItem,
  HeuristicFinding,
  HeuristicKind,
  HistoryEntry,
  SnapshotInfo,
} from "../types";
import { GB, MB, type MockTree } from "./tree";

const DAY = 86_400_000;

export function heuristicFindings(tree: MockTree, kinds: HeuristicKind[], staleMonths: number): HeuristicFinding[] {
  const now = Date.now();
  const id = (p: string) => tree.findByPath(p)?.id ?? null;
  const size = (p: string, fallback: number) => tree.findByPath(p)?.size ?? fallback;
  const U = "C:\\Users\\Ali";
  const all: HeuristicFinding[] = [
    {
      kind: "orphan",
      path: `${U}\\AppData\\Roaming\\Adobe\\Premiere Pro`,
      nodeId: id(`${U}\\AppData\\Roaming\\Adobe\\Premiere Pro`),
      bytes: size(`${U}\\AppData\\Roaming\\Adobe\\Premiere Pro`, 2.1 * GB),
      confidence: 0.82,
      safety: "probably_safe",
      reason: {
        fa: "Adobe Premiere Pro دیگر نصب نیست، ولی تنظیمات و حافظهٔ موقتش هنوز اینجاست.",
        en: "Adobe Premiere Pro is no longer installed, but its settings and cache are still here.",
      },
      details: { kind: "orphan", appName: "Adobe Premiere Pro" },
    },
    {
      kind: "orphan",
      path: `${U}\\AppData\\Local\\JetBrains\\PyCharm2021.3`,
      nodeId: id(`${U}\\AppData\\Local\\JetBrains\\PyCharm2021.3`),
      bytes: size(`${U}\\AppData\\Local\\JetBrains\\PyCharm2021.3`, 1.4 * GB),
      confidence: 0.7,
      safety: "probably_safe",
      reason: {
        fa: "این پوشه مال PyCharm 2021.3 است. نسخهٔ نصب‌شدهٔ فعلی جدیدتر است و از آن استفاده نمی‌کند.",
        en: "This folder is for PyCharm 2021.3. The installed version is newer and does not use it.",
      },
      details: { kind: "orphan", appName: "PyCharm 2021.3" },
    },
    {
      kind: "stale",
      path: `${U}\\Documents\\Archive`,
      nodeId: id(`${U}\\Documents\\Archive`),
      bytes: size(`${U}\\Documents\\Archive`, 3.2 * GB),
      confidence: 0.6,
      safety: "careful",
      reason: {
        fa: "حدود دو سال است که هیچ فایلی در این پوشه تغییر نکرده.",
        en: "No file in this folder has changed in about two years.",
      },
      details: { kind: "stale", lastModified: now - 720 * DAY, lastAccessed: null, accessTimeReliable: false },
    },
    {
      kind: "stale",
      path: `${U}\\Videos\\Movies`,
      nodeId: id(`${U}\\Videos\\Movies`),
      bytes: size(`${U}\\Videos\\Movies`, 30 * GB),
      confidence: 0.55,
      safety: "careful",
      reason: {
        fa: "این فیلم‌ها بیش از یک سال است که باز نشده‌اند.",
        en: "These movies have not been opened in over a year.",
      },
      details: { kind: "stale", lastModified: now - 500 * DAY, lastAccessed: now - 420 * DAY, accessTimeReliable: true },
    },
    {
      kind: "duplicates",
      path: `${U}\\Videos\\wedding.mp4`,
      nodeId: id(`${U}\\Videos\\wedding.mp4`),
      bytes: 2.3 * GB,
      confidence: 0.99,
      safety: "probably_safe",
      reason: {
        fa: "دو فایل دقیقاً یکسان پیدا شد. نگه داشتن یکی کافی است.",
        en: "Two identical files were found. Keeping one is enough.",
      },
      details: {
        kind: "duplicates",
        fileSize: 2.3 * GB,
        keepIndex: 0,
        files: [
          { path: `${U}\\Videos\\wedding.mp4`, modified: now - 900 * DAY },
          { path: `${U}\\Downloads\\wedding (1).mp4`, modified: now - 260 * DAY },
        ],
      },
    },
    {
      kind: "duplicates",
      path: `${U}\\Downloads\\misc\\backup3.zip`,
      nodeId: null,
      bytes: 180 * MB,
      confidence: 0.97,
      safety: "probably_safe",
      reason: {
        fa: "سه نسخهٔ یکسان از این فایل وجود دارد.",
        en: "There are three identical copies of this file.",
      },
      details: {
        kind: "duplicates",
        fileSize: 90 * MB,
        keepIndex: 2,
        files: [
          { path: `${U}\\Downloads\\misc\\backup3.zip`, modified: now - 300 * DAY },
          { path: `${U}\\Desktop\\backup3.zip`, modified: now - 200 * DAY },
          { path: `${U}\\Documents\\Work\\backup3.zip`, modified: now - 100 * DAY },
        ],
      },
    },
    {
      kind: "old_project",
      path: `${U}\\Projects\\thesis-ml`,
      nodeId: id(`${U}\\Projects\\thesis-ml`),
      bytes: size(`${U}\\Projects\\thesis-ml`, 10.4 * GB),
      confidence: 0.85,
      safety: "careful",
      reason: {
        fa: "یک پروژهٔ پایتون که حدود دو سال است دست نخورده. محیط مجازی آن را می‌شود دوباره ساخت.",
        en: "A Python project untouched for about two years. Its virtual environment can be rebuilt.",
      },
      details: {
        kind: "old_project",
        projectKind: "python",
        lastTouched: now - 700 * DAY,
        rebuildableBytes: 4.1 * GB,
        rebuildableDirs: [`${U}\\Projects\\thesis-ml\\.venv`],
      },
    },
    {
      kind: "old_project",
      path: `${U}\\Projects\\shop-api`,
      nodeId: id(`${U}\\Projects\\shop-api`),
      bytes: size(`${U}\\Projects\\shop-api`, 3.3 * GB),
      confidence: 0.8,
      safety: "careful",
      reason: {
        fa: "یک پروژهٔ Node.js که بیش از یک سال است باز نشده. پوشهٔ node_modules آن با npm install برمی‌گردد.",
        en: "A Node.js project not opened in over a year. Its node_modules comes back with npm install.",
      },
      details: {
        kind: "old_project",
        projectKind: "node",
        lastTouched: now - 540 * DAY,
        rebuildableBytes: 2.9 * GB,
        rebuildableDirs: [`${U}\\Projects\\shop-api\\node_modules`],
      },
    },
    {
      kind: "old_project",
      path: `${U}\\Projects\\mobile-app`,
      nodeId: id(`${U}\\Projects\\mobile-app`),
      bytes: size(`${U}\\Projects\\mobile-app`, 3.8 * GB),
      confidence: 0.75,
      safety: "careful",
      reason: {
        fa: "یک پروژهٔ React Native که بیش از یک سال است باز نشده.",
        en: "A React Native project not opened in over a year.",
      },
      details: {
        kind: "old_project",
        projectKind: "node",
        lastTouched: now - 420 * DAY,
        rebuildableBytes: 3.8 * GB,
        rebuildableDirs: [`${U}\\Projects\\mobile-app\\node_modules`, `${U}\\Projects\\mobile-app\\android\\build`],
      },
    },
  ];
  const staleCut = now - staleMonths * 30 * DAY;
  const root = tree.rootPath.toLowerCase().replace(/\\$/, "");
  return all.filter(
    (x) =>
      kinds.includes(x.kind) &&
      x.path.toLowerCase().startsWith(root) &&
      (x.details.kind !== "stale" || (x.details.lastModified ?? 0) < staleCut),
  );
}

/** Deltas since the previous snapshot of C:, adds up to about +12 GB. */
export const C_GROWTH: { path: string; delta: number }[] = [
  { path: "C:\\Users\\Ali\\AppData\\Roaming\\Telegram Desktop\\tdata\\user_data", delta: 8.1 * GB },
  { path: "C:\\Windows\\SoftwareDistribution\\Download", delta: 3.0 * GB },
  { path: "C:\\Users\\Ali\\AppData\\Local\\Docker\\wsl\\disk\\docker_data.vhdx", delta: 2.4 * GB },
  { path: "C:\\Users\\Ali\\Videos\\Captures", delta: 1.5 * GB },
  { path: "C:\\Users\\Ali\\AppData\\Local\\Temp", delta: 1.1 * GB },
  { path: "C:\\Users\\Ali\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache", delta: 0.6 * GB },
  { path: "C:\\Users\\Ali\\AppData\\Local\\npm-cache", delta: 0.4 * GB },
  { path: "C:\\Users\\Ali\\Downloads", delta: -4.6 * GB },
];

export function seedSnapshots(rootPath: string, totalBytes: number, files: number, driveTotal: number, driveFree: number): SnapshotInfo[] {
  const now = Date.now();
  const out: SnapshotInfo[] = [];
  const growthPerMonth = 12 * GB;
  for (let i = 5; i >= 1; i--) {
    const taken = now - i * 30 * DAY - 3 * 3_600_000;
    const bytes = totalBytes - i * growthPerMonth + (i % 2) * 1.3 * GB;
    out.push({
      id: 0,
      rootPath,
      takenAt: taken,
      totalBytes: bytes,
      files: files - i * 21_000,
      driveTotal,
      driveFree: driveFree + (totalBytes - bytes),
    });
  }
  return out;
}

export function scaleGrowth(factor: number, lookup: (p: string) => GrowthItem["explanation"], sizeOf: (p: string) => number): GrowthItem[] {
  return C_GROWTH.map((g) => {
    const delta = Math.round(g.delta * factor);
    const after = sizeOf(g.path) || Math.abs(delta) * 3;
    return { path: g.path, before: Math.max(0, after - delta), after, delta, explanation: lookup(g.path) };
  }).sort((a, b) => Math.abs(b.delta) - Math.abs(a.delta));
}

export function seedHistory(): HistoryEntry[] {
  const now = Date.now();
  const U = "C:\\Users\\Ali";
  return [
    {
      runId: 2,
      startedAt: now - 12 * DAY,
      finishedAt: now - 12 * DAY + 41_000,
      dryRun: false,
      bytesFreed: 6.9 * GB,
      actions: [
        { path: `${U}\\AppData\\Local\\Temp`, method: "delete_contents", bytes: 3.1 * GB, status: "partial", restorable: false, errorCode: "in_use" },
        { path: `${U}\\AppData\\Local\\npm-cache`, method: "delete_contents", bytes: 2.6 * GB, status: "done", restorable: false, errorCode: null },
        { path: `${U}\\Downloads\\Win10_22H2_x64.iso`, method: "recycle", bytes: 5.8 * GB, status: "done", restorable: true, errorCode: null },
        { path: `${U}\\AppData\\Local\\Docker\\wsl\\disk\\docker_data.vhdx`, method: "compact_vhdx", bytes: 49 * GB, status: "failed", restorable: false, errorCode: "vhdx_in_use" },
      ],
    },
    {
      runId: 1,
      startedAt: now - 64 * DAY,
      finishedAt: now - 64 * DAY + 9_000,
      dryRun: true,
      bytesFreed: 0,
      actions: [
        { path: "C:\\Windows\\SoftwareDistribution\\Download", method: "delete_contents", bytes: 2.2 * GB, status: "dry_run", restorable: false, errorCode: null },
        { path: `${U}\\AppData\\Local\\NVIDIA\\DXCache`, method: "delete_contents", bytes: 0.9 * GB, status: "dry_run", restorable: false, errorCode: null },
      ],
    },
  ];
}
