import { describe, expect, it } from "vitest";
import { EVENTS, on } from "../events";
import type { CleanupReport, ScanSummary } from "../types";
import { mockBackend as b } from "./index";

async function scanOnce(path: string, mode: "normal" | "fast" = "fast"): Promise<ScanSummary> {
  const done = new Promise<ScanSummary>((resolve) => {
    void on(EVENTS.scanDone, (s) => resolve(s));
  });
  await b.startScan(path, mode);
  return done;
}

describe("mock backend", () => {
  it("lists drives with believable free space", async () => {
    const drives = await b.listDrives();
    const c = drives.find((d) => d.letter === "C:");
    expect(c?.total).toBe(476 * 1024 ** 3);
    expect(c && c.free > 0 && c.free < c.total).toBe(true);
  });

  it("scans, pages children and builds a story", async () => {
    const s = await scanOnce("C:\\");
    expect(s.files).toBeGreaterThan(100_000);
    expect(s.fallbackReason).toBeNull();

    const page = await b.getChildren(s.scanId, s.rootNode, "size", 0, 5);
    expect(page.items.length).toBe(5);
    expect(page.items[0].size).toBeGreaterThanOrEqual(page.items[1].size);

    const story = await b.getStory(s.scanId);
    expect(story.reasons.length).toBe(5);
    expect(story.safeItems.every((r) => r.explanation.safety === "safe")).toBe(true);
    expect(story.safeBytes).toBeGreaterThan(0);

    const tm = await b.getTreemap(s.scanId, s.rootNode, 2, 60);
    expect(tm.children.length).toBeGreaterThan(3);
  }, 10_000);

  it("falls back to the normal scan on non NTFS drives", async () => {
    const s = await scanOnce("E:\\");
    expect(s.fallbackReason).toBe("not_ntfs");
    expect(s.scanner).toBe("normal");
  }, 10_000);

  it("blocks protected paths in a cleanup plan and reports a dry run", async () => {
    const s = await scanOnce("C:\\");
    const story = await b.getStory(s.scanId);
    const installer = { path: "C:\\Windows\\Installer", bytes: 1, explanation: { ...story.reasons[0].explanation, safety: "do_not_touch" as const } };
    const plan = await b.buildCleanupPlan([...story.safeItems.map((r) => ({ path: r.path, bytes: r.bytes, explanation: r.explanation })), installer]);
    expect(plan.actions.some((a) => a.blocked)).toBe(true);
    expect(plan.warnings.some((w) => w.code === "recycle_same_drive")).toBe(true);

    const report = new Promise<CleanupReport>((resolve) => {
      void on(EVENTS.cleanupDone, (r) => resolve(r));
    });
    await b.runCleanup(plan.planId, { dryRun: true, permanentForSafe: false, createRestorePoint: false });
    const r = await report;
    expect(r.dryRun).toBe(true);
    expect(r.freeAfter).toBe(r.freeBefore);
    expect(r.results.find((x) => x.path === "C:\\Windows\\Installer")?.status).toBe("blocked");
  }, 20_000);
});
