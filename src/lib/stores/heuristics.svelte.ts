import { backend } from "../api/client";
import { EVENTS, on } from "../api/events";
import type { HeuristicFinding, HeuristicKind, JobId, ScanId } from "../api/types";
import { toasts } from "./toasts.svelte";

interface KindState {
  scanId: ScanId;
  running: boolean;
  done: number;
  total: number;
  findings: HeuristicFinding[] | null;
}

class HeuristicsStore {
  byKind = $state<Partial<Record<HeuristicKind, KindState>>>({});
  private jobs = new Map<JobId, HeuristicKind[]>();
  private started = false;

  async init(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await on(EVENTS.heuristicsProgress, (p) => {
      const kinds = this.jobs.get(p.jobId);
      if (!kinds) return;
      for (const k of kinds) {
        const s = this.byKind[k];
        if (s) this.byKind[k] = { ...s, done: p.done, total: p.total };
      }
    });
    await on(EVENTS.heuristicsDone, (d) => {
      const kinds = this.jobs.get(d.jobId);
      if (!kinds) return;
      this.jobs.delete(d.jobId);
      for (const k of kinds) {
        this.byKind[k] = {
          scanId: d.scanId,
          running: false,
          done: 1,
          total: 1,
          findings: d.findings.filter((f) => f.kind === k).sort((a, b) => b.bytes - a.bytes),
        };
      }
    });
  }

  state(kind: HeuristicKind, scanId: ScanId | null): KindState | null {
    const s = this.byKind[kind];
    return s && s.scanId === scanId ? s : null;
  }

  async run(scanId: ScanId, kind: HeuristicKind, force = false): Promise<void> {
    const cur = this.state(kind, scanId);
    if (cur && (cur.running || (cur.findings && !force))) return;
    this.byKind[kind] = { scanId, running: true, done: 0, total: 0, findings: null };
    try {
      const jobId = await backend.runHeuristics(scanId, [kind]);
      this.jobs.set(jobId, [kind]);
    } catch (e) {
      this.byKind[kind] = undefined;
      toasts.error(e);
    }
  }
}

export const heuristics = new HeuristicsStore();
