import type { Category, SafetyLevel, ThemePref } from "../api/types";
import "./tokens.css";
import "./base.css";

export type ResolvedTheme = "light" | "dark";

const media = typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;
let currentPref: ThemePref = "system";
const listeners = new Set<(t: ResolvedTheme) => void>();

export function resolveTheme(pref: ThemePref): ResolvedTheme {
  if (pref === "system") return media?.matches ? "dark" : "light";
  return pref;
}

export function applyTheme(pref: ThemePref): void {
  currentPref = pref;
  const resolved = resolveTheme(pref);
  document.documentElement.dataset.theme = resolved;
  for (const l of listeners) l(resolved);
}

media?.addEventListener("change", () => {
  if (currentPref === "system") applyTheme("system");
});

/** Called when the resolved theme changes, so canvas charts can repaint. */
export function onThemeChange(cb: (t: ResolvedTheme) => void): () => void {
  listeners.add(cb);
  return () => listeners.delete(cb);
}

export function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

export const CATEGORIES: Category[] = [
  "system",
  "apps",
  "games",
  "media",
  "dev",
  "cache",
  "user_files",
  "virtualization",
  "messaging",
  "browsers",
  "unknown",
];

export function categoryColor(cat: Category): string {
  return cssVar(`--cat-${cat}`) || "#999";
}

export const SAFETY_LEVELS: SafetyLevel[] = ["safe", "probably_safe", "careful", "do_not_touch"];

export const SAFETY_META: Record<SafetyLevel, { color: string; soft: string; icon: string }> = {
  safe: { color: "var(--safe)", soft: "var(--safe-soft)", icon: "check-circle" },
  probably_safe: { color: "var(--probably-safe)", soft: "var(--probably-safe-soft)", icon: "check-dashed" },
  careful: { color: "var(--careful)", soft: "var(--careful-soft)", icon: "alert" },
  do_not_touch: { color: "var(--do-not-touch)", soft: "var(--do-not-touch-soft)", icon: "lock" },
};
