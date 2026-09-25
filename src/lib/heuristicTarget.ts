import type { CleanupTarget, Explanation, HeuristicFinding, SafetyLevel } from "./api/types";
import { both } from "./i18n/index.svelte";

/** Heuristics are guesses, so they are never shown or cleaned as "safe". */
export function cappedSafety(s: SafetyLevel): SafetyLevel {
  return s === "safe" ? "probably_safe" : s;
}

export function findingExplanation(f: HeuristicFinding): Explanation {
  return {
    ruleId: `heuristic-${f.kind}`,
    source: "heuristic",
    title: both(`heur.kinds.${f.kind}.title`),
    whyBig: f.reason,
    ifDeleted: both(`heur.kinds.${f.kind}.ifDeleted`),
    safety: cappedSafety(f.safety),
    method: "recycle",
    needsAdmin: false,
    instructions: null,
    confidence: f.confidence,
  };
}

export function findingTarget(f: HeuristicFinding, path = f.path, bytes = f.bytes): CleanupTarget {
  return { path, bytes, explanation: findingExplanation(f) };
}
