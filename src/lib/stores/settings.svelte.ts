import { backend } from "../api/client";
import type { AppInfo, AppSettings } from "../api/types";
import { setLanguage } from "../i18n/index.svelte";
import { applyTheme } from "../theme";
import { toasts } from "./toasts.svelte";
import { ui } from "./ui.svelte";

export const DEFAULT_SETTINGS: AppSettings = {
  language: "fa",
  theme: "system",
  defaultMode: "simple",
  excludedPaths: [],
  staleMonths: 12,
  oldProjectMonths: 6,
  aiEnabled: false,
  aiMaskNames: true,
  aiDefaultProvider: null,
  aiPreviewAcknowledged: false,
  lowSpaceThresholdGb: 10,
  trayEnabled: false,
  weeklyCheck: false,
  onboardingDone: false,
  devSandbox: null,
};

class SettingsStore {
  value = $state<AppSettings | null>(null);
  info = $state<AppInfo | null>(null);

  async load(): Promise<void> {
    const [s, info] = await Promise.all([backend.getSettings(), backend.getAppInfo()]);
    this.info = info;
    this.value = s;
    setLanguage(s.language);
    applyTheme(s.theme);
    ui.mode = s.defaultMode;
    ui.showOnboarding = !s.onboardingDone;
  }

  /** Used when the backend can't give us settings, so the app still opens. */
  useDefaults(): void {
    this.value = { ...DEFAULT_SETTINGS };
    setLanguage(DEFAULT_SETTINGS.language);
    applyTheme(DEFAULT_SETTINGS.theme);
  }

  /** Applies the change right away, then saves. Rolls back if saving fails. */
  async update(patch: Partial<AppSettings>): Promise<boolean> {
    const prev = this.value;
    if (!prev) return false;
    const next = { ...prev, ...patch };
    this.value = next;
    if (patch.language) setLanguage(patch.language);
    if (patch.theme) applyTheme(patch.theme);
    try {
      this.value = await backend.setSettings(next);
      return true;
    } catch (e) {
      this.value = prev;
      setLanguage(prev.language);
      applyTheme(prev.theme);
      toasts.error(e);
      return false;
    }
  }
}

export const settings = new SettingsStore();
