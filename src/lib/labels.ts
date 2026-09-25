// Colors and label keys for categories, safety levels and cleanup methods.

import type { Category, CleanupMethod, SafetyLevel } from "./api/types";

export const CATEGORY_COLORS: Record<Category, string> = {
  system: "#7b8494",
  apps: "#4f7fd9",
  games: "#8d5bd6",
  media: "#d6568a",
  dev: "#2f9e8f",
  cache: "#e0a030",
  user_files: "#3aa3d9",
  virtualization: "#6d6bd6",
  messaging: "#38b36b",
  browsers: "#e07a3a",
  unknown: "#a3abb6",
};

export const CATEGORY_ICONS: Record<Category, string> = {
  system: "shield",
  apps: "box",
  games: "play",
  media: "eye",
  dev: "code",
  cache: "layers",
  user_files: "file",
  virtualization: "drive",
  messaging: "copy",
  browsers: "globe",
  unknown: "help",
};

export const categoryColor =(c: Category) => CATEGORY_COLORS[c] ?? CATEGORY_COLORS.unknown;
export const categoryKey = (c: Category) => `category.${c}`;

export const SAFETY_ORDER: SafetyLevel[] = ["safe", "probably_safe", "careful", "do_not_touch"];
export const safetyKey = (s: SafetyLevel) => `safety.${s}`;
/** CSS custom property name (without `var()`) for a safety color. */
export const safetyVar = (s: SafetyLevel) =>
  ({ safe: "--safe", probably_safe: "--probably-safe", careful: "--careful", do_not_touch: "--danger" })[s];

export const methodKey = (m: CleanupMethod) => `method.${m}`;

/** Methods the app can carry out by itself (the rest need the user or open something). */
export const AUTO_METHODS: CleanupMethod[] = ["recycle", "delete_contents", "command", "compact_vhdx"];

/** Fraction used, and which bar color to show. */
export function usage(total: number, free: number): { share: number; level: "ok" | "warn" | "full" } {
  const share = total > 0 ? Math.min(1, Math.max(0, (total - free) / total)) : 0;
  return { share, level: share >= 0.95 ? "full" : share >= 0.85 ? "warn" : "ok" };
}
