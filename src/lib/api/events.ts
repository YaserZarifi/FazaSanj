import { isTauri } from "./env";
import { EVENTS } from "./commands";
import type {
  CleanupProgress,
  CleanupReport,
  HeuristicsDone,
  HeuristicsProgress,
  ScanFailed,
  ScanId,
  ScanProgress,
  ScanSummary,
} from "./types";

export interface EventMap {
  [EVENTS.scanProgress]: ScanProgress;
  [EVENTS.scanDone]: ScanSummary;
  [EVENTS.scanCancelled]: { scanId: ScanId };
  [EVENTS.scanError]: ScanFailed;
  [EVENTS.heuristicsProgress]: HeuristicsProgress;
  [EVENTS.heuristicsDone]: HeuristicsDone;
  [EVENTS.cleanupProgress]: CleanupProgress;
  [EVENTS.cleanupDone]: CleanupReport;
}

export type EventName = keyof EventMap;
type Handler<K extends EventName> = (payload: EventMap[K]) => void;

// Local bus used by the mock backend. Real events come from Tauri.
const local = new Map<EventName, Set<(p: unknown) => void>>();

export function emitLocal<K extends EventName>(name: K, payload: EventMap[K]): void {
  const set = local.get(name);
  if (!set) return;
  for (const h of [...set]) h(payload);
}

/** Subscribes to a backend event. Returns an unlisten function. */
export async function on<K extends EventName>(name: K, handler: Handler<K>): Promise<() => void> {
  if (isTauri) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<EventMap[K]>(name, (e) => handler(e.payload));
  }
  let set = local.get(name);
  if (!set) {
    set = new Set();
    local.set(name, set);
  }
  const h = handler as (p: unknown) => void;
  set.add(h);
  return () => set.delete(h);
}

export { EVENTS };
