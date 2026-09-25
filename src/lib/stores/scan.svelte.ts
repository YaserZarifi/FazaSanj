import { backend } from "../api/client";
import { toApiError } from "../api/errors";
import { EVENTS, on } from "../api/events";
import type {
  ApiError,
  DriveInfo,
  NodeId,
  NodeInfo,
  ScanId,
  ScanMode,
  ScanProgress,
  ScanSummary,
  SnapshotComparison,
  Story,
} from "../api/types";
import { drives } from "./drives.svelte";
import { toasts } from "./toasts.svelte";
import { ui } from "./ui.svelte";

export type ScanPhase = "idle" | "prescan" | "starting" | "scanning" | "done" | "error";

export interface ScanTarget {
  path: string;
  drive: DriveInfo | null;
  isFolder: boolean;
}

class ScanStore {
  phase = $state<ScanPhase>("idle");
  target = $state<ScanTarget | null>(null);
  scanId = $state<ScanId | null>(null);
  progress = $state<ScanProgress | null>(null);
  summary = $state<ScanSummary | null>(null);
  error = $state<ApiError | null>(null);
  cancelling = $state(false);
  /** set when the user cancelled, so the pre-scan panel can say so */
  wasCancelled = $state(false);

  story = $state<Story | null>(null);
  storyLoading = $state(false);
  sinceLast = $state<SnapshotComparison | null>(null);

  /** Folder shown in the tree view, with the breadcrumb from the root. */
  trail = $state<NodeInfo[]>([]);
  selected = $state<NodeInfo | null>(null);
  /** Bumped after a cleanup so views refetch. */
  version = $state(0);

  private started = false;

  get current(): NodeInfo | null {
    return this.trail.length ? this.trail[this.trail.length - 1] : null;
  }

  get root(): NodeInfo | null {
    return this.trail[0] ?? null;
  }

  /** Subscribes to scan events once for the life of the app. */
  async init(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await on(EVENTS.scanProgress, (p) => {
      if (!this.accepts(p.scanId)) return;
      this.phase = "scanning";
      this.progress = p;
    });
    await on(EVENTS.scanDone, (s) => {
      if (!this.accepts(s.scanId)) return;
      this.scanId = s.scanId;
      void this.finish(s);
    });
    await on(EVENTS.scanCancelled, (e) => {
      if (!this.accepts(e.scanId)) return;
      this.cancelling = false;
      this.wasCancelled = true;
      this.phase = "prescan";
      this.scanId = null;
      this.progress = null;
    });
    await on(EVENTS.scanError, (e) => {
      if (!this.accepts(e.scanId)) return;
      this.error = e.error;
      this.phase = "error";
    });
  }

  // the first progress event can arrive before start_scan returns the id
  private accepts(id: ScanId): boolean {
    if (this.scanId === id) return true;
    return this.scanId == null && (this.phase === "starting" || this.phase === "scanning");
  }

  choose(target: ScanTarget): void {
    if (this.phase === "scanning" || this.phase === "starting") return;
    this.target = target;
    this.phase = "prescan";
    this.error = null;
    this.wasCancelled = false;
    ui.go("home");
  }

  chooseDrive(d: DriveInfo): void {
    this.choose({ path: d.root, drive: d, isFolder: false });
  }

  chooseFolder(path: string): void {
    this.choose({ path, drive: drives.byPath(path), isFolder: true });
  }

  reset(): void {
    this.phase = "idle";
    this.target = null;
    this.clearResults();
  }

  private clearResults(): void {
    this.scanId = null;
    this.summary = null;
    this.progress = null;
    this.story = null;
    this.sinceLast = null;
    this.trail = [];
    this.selected = null;
    this.error = null;
  }

  async start(mode: ScanMode): Promise<void> {
    if (!this.target) return;
    this.clearResults();
    this.wasCancelled = false;
    this.phase = "starting";
    try {
      const id = await backend.startScan(this.target.path, mode);
      // done may already have been handled if the scan was tiny
      if (this.scanId == null) this.scanId = id;
    } catch (e) {
      this.error = toApiError(e);
      this.phase = "error";
    }
  }

  async cancel(): Promise<void> {
    if (this.scanId == null) return;
    this.cancelling = true;
    try {
      await backend.cancelScan(this.scanId);
    } catch (e) {
      this.cancelling = false;
      toasts.error(e);
    }
  }

  private async finish(s: ScanSummary): Promise<void> {
    this.summary = s;
    this.progress = null;
    try {
      const root = await backend.getNode(s.scanId, s.rootNode);
      this.trail = [root];
      this.selected = null;
      this.phase = "done";
    } catch (e) {
      this.error = toApiError(e);
      this.phase = "error";
      return;
    }
    void drives.load();
    void this.loadStory();
    void this.loadSinceLast();
  }

  async loadStory(): Promise<void> {
    if (this.scanId == null) return;
    this.storyLoading = true;
    try {
      this.story = await backend.getStory(this.scanId);
    } catch (e) {
      toasts.error(e);
    } finally {
      this.storyLoading = false;
    }
  }

  async loadSinceLast(): Promise<void> {
    if (this.scanId == null) return;
    try {
      this.sinceLast = await backend.compareWithLast(this.scanId);
    } catch {
      // growth is optional, the story works without it
      this.sinceLast = null;
    }
  }

  /** Opens a folder in the tree and rebuilds the breadcrumb from its parents. */
  async openFolder(nodeId: NodeId): Promise<void> {
    if (this.scanId == null) return;
    const idx = this.trail.findIndex((n) => n.id === nodeId);
    if (idx >= 0) {
      this.trail = this.trail.slice(0, idx + 1);
      return;
    }
    try {
      const chain: NodeInfo[] = [];
      let n: NodeInfo | null = await backend.getNode(this.scanId, nodeId);
      while (n) {
        chain.unshift(n);
        n = n.parent != null ? await backend.getNode(this.scanId, n.parent) : null;
      }
      this.trail = chain;
    } catch (e) {
      toasts.error(e);
    }
  }

  enter(node: NodeInfo): void {
    if (!node.isDir || node.childCount === 0) return;
    this.trail = [...this.trail, node];
  }

  up(): void {
    if (this.trail.length > 1) this.trail = this.trail.slice(0, -1);
  }

  async select(nodeId: NodeId | null): Promise<void> {
    if (nodeId == null || this.scanId == null) {
      this.selected = null;
      return;
    }
    try {
      this.selected = await backend.getNode(this.scanId, nodeId);
    } catch (e) {
      toasts.error(e);
    }
  }

  /** Called after a real cleanup. The tree in memory is stale, so ask for a rescan. */
  markStale(): void {
    this.version++;
  }
}

export const scan = new ScanStore();
