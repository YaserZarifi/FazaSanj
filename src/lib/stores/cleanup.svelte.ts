import { backend } from "../api/client";
import { toApiError } from "../api/errors";
import { EVENTS, on } from "../api/events";
import type {
  ApiError,
  CleanupOptions,
  CleanupPlan,
  CleanupProgress,
  CleanupReport,
  CleanupTarget,
  Explanation,
  JobId,
} from "../api/types";
import { drives } from "./drives.svelte";
import { scan } from "./scan.svelte";
import { toasts } from "./toasts.svelte";
import { ui } from "./ui.svelte";

export type CleanupStage = "review" | "running" | "result";

class CleanupStore {
  basket = $state<CleanupTarget[]>([]);

  plan = $state<CleanupPlan | null>(null);
  planLoading = $state(false);
  planError = $state<ApiError | null>(null);
  options = $state<CleanupOptions>({ dryRun: false, permanentForSafe: false, createRestorePoint: true });

  stage = $state<CleanupStage>("review");
  jobId = $state<JobId | null>(null);
  runningDry = $state(false);
  progress = $state<CleanupProgress | null>(null);
  dryRunReport = $state<CleanupReport | null>(null);
  report = $state<CleanupReport | null>(null);

  private started = false;

  get basketBytes(): number {
    return this.basket.reduce((s, t) => s + t.bytes, 0);
  }

  async init(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await on(EVENTS.cleanupProgress, (p) => {
      if (p.jobId === this.jobId || this.jobId == null) this.progress = p;
    });
    await on(EVENTS.cleanupDone, (r) => {
      if (this.jobId != null && r.jobId !== this.jobId) return;
      this.jobId = null;
      this.progress = null;
      if (r.dryRun) {
        this.dryRunReport = r;
        this.runningDry = false;
        this.stage = "review";
        return;
      }
      this.report = r;
      this.stage = "result";
      this.basket = [];
      scan.markStale();
      void drives.load();
    });
  }

  has(path: string): boolean {
    return this.basket.some((t) => t.path === path);
  }

  add(target: CleanupTarget): void {
    if (this.has(target.path)) return;
    // a folder already in the basket covers everything inside it
    const lower = target.path.toLowerCase();
    if (this.basket.some((t) => lower.startsWith(`${t.path.toLowerCase()}\\`))) return;
    this.basket = [...this.basket.filter((t) => !t.path.toLowerCase().startsWith(`${lower}\\`)), target];
  }

  addExplained(path: string, bytes: number, explanation: Explanation): void {
    this.add({ path, bytes, explanation });
  }

  remove(path: string): void {
    this.basket = this.basket.filter((t) => t.path !== path);
  }

  toggle(target: CleanupTarget): void {
    if (this.has(target.path)) this.remove(target.path);
    else this.add(target);
  }

  clear(): void {
    this.basket = [];
  }

  /** Builds a plan and opens the review screen. Nothing is deleted here. */
  async review(targets: CleanupTarget[] = this.basket): Promise<void> {
    this.plan = null;
    this.planError = null;
    this.dryRunReport = null;
    this.report = null;
    this.stage = "review";
    this.planLoading = true;
    ui.go("cleanup");
    try {
      const plan = await backend.buildCleanupPlan(targets);
      this.plan = plan;
      this.options = { dryRun: false, permanentForSafe: false, createRestorePoint: plan.hasSystemActions };
    } catch (e) {
      this.planError = toApiError(e);
    } finally {
      this.planLoading = false;
    }
  }

  async run(dryRun: boolean): Promise<void> {
    if (!this.plan) return;
    const opts: CleanupOptions = { ...this.options, dryRun };
    this.progress = null;
    if (dryRun) {
      this.runningDry = true;
      this.dryRunReport = null;
    } else {
      this.stage = "running";
    }
    try {
      this.jobId = await backend.runCleanup(this.plan.planId, opts);
    } catch (e) {
      this.runningDry = false;
      this.stage = "review";
      toasts.error(e);
    }
  }

  close(): void {
    this.plan = null;
    this.report = null;
    this.dryRunReport = null;
    this.stage = "review";
    ui.go("home");
  }
}

export const cleanup = new CleanupStore();
