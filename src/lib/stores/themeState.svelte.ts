import { onThemeChange, type ResolvedTheme } from "../theme";

/** Reactive copy of the resolved theme, so canvas charts can rebuild their colors. */
class ThemeState {
  resolved = $state<ResolvedTheme>(document.documentElement.dataset.theme === "dark" ? "dark" : "light");
}

export const themeState = new ThemeState();

onThemeChange((t) => {
  themeState.resolved = t;
});
