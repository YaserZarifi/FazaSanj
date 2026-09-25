<script lang="ts">
  import type { Language, ThemePref, UiMode } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";
  import Segmented from "../common/Segmented.svelte";
  import type { IconName } from "../common/icons";

  const title = $derived.by(() => {
    if (ui.view === "home" || ui.view === "heuristics") {
      const tg = scan.target;
      if (!tg) return t("home.title");
      if (tg.isFolder) return tg.path;
      return tg.drive ? `${tg.drive.letter} ${tg.drive.label || t("drives.localDisk")}` : tg.path;
    }
    return t(`nav.${ui.view}`);
  });

  const subtitle = $derived.by(() => {
    if (ui.view !== "home") return "";
    switch (scan.phase) {
      case "prescan":
        return t("top.readyToScan");
      case "starting":
      case "scanning":
        return t("top.scanning");
      case "done":
        return ui.mode === "simple" ? t("top.simpleHint") : t("top.expertHint");
      default:
        return "";
    }
  });

  const THEME_ORDER: ThemePref[] = ["system", "light", "dark"];
  const THEME_ICON: Record<ThemePref, IconName> = { system: "monitor", light: "sun", dark: "moon" };
  const theme = $derived(settings.value?.theme ?? "system");

  function cycleTheme() {
    const next = THEME_ORDER[(THEME_ORDER.indexOf(theme) + 1) % THEME_ORDER.length];
    void settings.update({ theme: next });
  }
</script>

<header class="top">
  <div class="title">
    <h1 class:path={scan.target?.isFolder && (ui.view === "home" || ui.view === "heuristics")}>{title}</h1>
    {#if subtitle}<p class="sub">{subtitle}</p>{/if}
  </div>

  <div class="tools">
    <Segmented
      label={t("top.modeLabel")}
      value={ui.mode}
      options={[
        { value: "simple" as UiMode, label: t("top.simple") },
        { value: "expert" as UiMode, label: t("top.expert") },
      ]}
      onchange={(m) => (ui.mode = m)}
    />
    <Segmented
      label={t("top.languageLabel")}
      value={settings.value?.language ?? "fa"}
      options={[
        { value: "fa" as Language, label: "فا" },
        { value: "en" as Language, label: "EN" },
      ]}
      onchange={(l) => void settings.update({ language: l })}
    />
    <button
      type="button"
      class="icon-btn theme"
      onclick={cycleTheme}
      title={t("top.themeNow", { theme: t(`theme.${theme}`) })}
      aria-label={t("top.themeNow", { theme: t(`theme.${theme}`) })}
    >
      <Icon name={THEME_ICON[theme]} size={18} />
    </button>
  </div>
</header>

<style>
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-4);
    min-height: var(--topbar-h);
    padding: var(--sp-3) var(--sp-6);
    border-bottom: 1px solid var(--border);
    background: color-mix(in srgb, var(--bg) 85%, transparent);
    backdrop-filter: blur(8px);
  }

  .title {
    min-width: 0;
  }

  h1 {
    font-size: var(--fs-lg);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  h1.path {
    direction: ltr;
    unicode-bidi: isolate;
    text-align: start;
  }

  .sub {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .tools {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-shrink: 0;
  }

  .theme {
    border: 1px solid var(--border);
    background: var(--surface);
  }
</style>
