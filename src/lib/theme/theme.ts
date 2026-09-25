import type { ThemePref } from "../api/types";

let media: MediaQueryList | null = null;
let current: ThemePref = "system";

function resolve(pref: ThemePref): "light" | "dark" {
  if (pref !== "system") return pref;
  return media?.matches ? "dark" : "light";
}

function paint() {
  document.documentElement.dataset.theme = resolve(current);
}

/** Applies a theme preference. "system" follows Windows and updates live. */
export function applyTheme(pref: ThemePref) {
  current = pref;
  if (typeof window === "undefined") return;
  if (!media) {
    media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", () => {
      if (current === "system") paint();
    });
  }
  paint();
}

/** "light" or "dark", what is on screen right now. */
export function effectiveTheme(): "light" | "dark" {
  return (document.documentElement.dataset.theme as "light" | "dark") || "light";
}
