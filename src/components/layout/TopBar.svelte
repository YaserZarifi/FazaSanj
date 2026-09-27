<script lang="ts">
  import type { UiMode } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { effectiveTheme } from "../../lib/theme/theme";
  import Icon from "../common/Icon.svelte";

  let { title }: { title: string } = $props();

  const modes: UiMode[] = ["simple", "expert"];

  function toggleTheme() {
    app.saveSettings({ theme: effectiveTheme() === "dark" ? "light" : "dark" });
  }

  function toggleLang() {
    app.saveSettings({ language: app.settings.language === "fa" ? "en" : "fa" });
  }
</script>

<header class="topbar">
  <h1 class="title">{title}</h1>
  <span class="spacer"></span>

  <div class="segmented" role="radiogroup" aria-label={t("topbar.mode")}>
    {#each modes as m (m)}
      <button role="radio" aria-checked={app.mode === m} class:on={app.mode === m} onclick={() => (app.mode = m)}>
        {t(`topbar.${m}`)}
      </button>
    {/each}
  </div>

  <button class="btn ghost" onclick={toggleLang} title={t("topbar.language")}>
    <Icon name="globe" size={17} />
    <span>{app.settings.language === "fa" ? "English" : "فارسی"}</span>
  </button>
  <button class="btn ghost" onclick={toggleTheme} aria-label={t("topbar.theme")} title={t("topbar.theme")}>
    <Icon name={app.settings.theme === "dark" || (app.settings.theme === "system" && effectiveTheme() === "dark") ? "sun" : "moon"} size={17} />
  </button>
</header>

<style>
  .topbar {
    height: var(--topbar-h);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 20px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .title {
    font-size: 17px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .segmented {
    display: flex;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    padding: 3px;
    gap: 2px;
  }
  .segmented button {
    border: none;
    background: none;
    padding: 4px 14px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-2);
    font-weight: 500;
  }
  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
  }
</style>
