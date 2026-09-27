<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import * as api from "../../lib/api/commands";
  import type { Language, ThemePref, UiMode } from "../../lib/api/types";
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Modal from "../common/Modal.svelte";
  import AiSettings from "./AiSettings.svelte";

  const s = $derived(app.settings);
  let confirmReset = $state(false);

  async function addExcluded() {
    const dir = await open({ directory: true, multiple: false, title: t("settings.addExcluded") });
    if (typeof dir === "string" && dir) app.saveSettings({ excludedPaths: [...s.excludedPaths, dir] });
  }

  function removeExcluded(p: string) {
    app.saveSettings({ excludedPaths: s.excludedPaths.filter((x) => x !== p) });
  }

  function num(e: Event, min: number, max: number): number {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    return Math.min(max, Math.max(min, Number.isFinite(v) ? Math.round(v) : min));
  }

  async function reset() {
    try {
      await api.resetEverything();
      confirmReset = false;
      await app.loadSettings();
      app.sessions = {};
      app.heuristics = {};
      app.toast(t("settings.resetDone"), "ok");
    } catch (e) {
      app.toast(errorText(e), "error");
    }
  }
</script>

<section class="page">
  <div class="card group">
    <h2>{t("settings.general")}</h2>
    <div class="field">
      <label for="lang">{t("settings.language")}</label>
      <select id="lang" class="select" value={s.language} onchange={(e) => app.saveSettings({ language: e.currentTarget.value as Language })}>
        <option value="fa">فارسی</option>
        <option value="en">English</option>
      </select>
    </div>
    <div class="field">
      <label for="theme">{t("settings.theme")}</label>
      <select id="theme" class="select" value={s.theme} onchange={(e) => app.saveSettings({ theme: e.currentTarget.value as ThemePref })}>
        <option value="system">{t("settings.themeSystem")}</option>
        <option value="light">{t("settings.themeLight")}</option>
        <option value="dark">{t("settings.themeDark")}</option>
      </select>
    </div>
    <div class="field">
      <label for="mode">{t("settings.defaultMode")}</label>
      <select id="mode" class="select" value={s.defaultMode} onchange={(e) => app.saveSettings({ defaultMode: e.currentTarget.value as UiMode })}>
        <option value="simple">{t("topbar.simple")}</option>
        <option value="expert">{t("topbar.expert")}</option>
      </select>
    </div>
  </div>

  <div class="card group">
    <h2>{t("settings.scanning")}</h2>
    <p class="muted">{t("settings.excludedHint")}</p>
    {#if s.excludedPaths.length}
      <ul class="paths">
        {#each s.excludedPaths as p (p)}
          <li>
            <bdi class="path">{p}</bdi>
            <span class="spacer"></span>
            <button class="btn ghost small" onclick={() => removeExcluded(p)} aria-label={t("common.remove")}>
              <Icon name="x" size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    <div><button class="btn small" onclick={addExcluded}><Icon name="folder" size={14} />{t("settings.addExcluded")}</button></div>
    <div class="field">
      <label for="stale">{t("settings.staleMonths")}</label>
      <input id="stale" class="input narrow" type="number" min="1" max="120" value={s.staleMonths} onchange={(e) => app.saveSettings({ staleMonths: num(e, 1, 120) })} />
    </div>
    <div class="field">
      <label for="proj">{t("settings.oldProjectMonths")}</label>
      <input id="proj" class="input narrow" type="number" min="1" max="120" value={s.oldProjectMonths} onchange={(e) => app.saveSettings({ oldProjectMonths: num(e, 1, 120) })} />
    </div>
  </div>

  <div class="card group">
    <h2>{t("settings.background")}</h2>
    <label class="toggle">
      <input type="checkbox" checked={s.trayEnabled} onchange={(e) => app.saveSettings({ trayEnabled: e.currentTarget.checked })} />
      <span><strong>{t("settings.tray")}</strong><span class="faint">{t("settings.trayHint")}</span></span>
    </label>
    <label class="toggle">
      <input type="checkbox" checked={s.weeklyCheck} onchange={(e) => app.saveSettings({ weeklyCheck: e.currentTarget.checked })} />
      <span><strong>{t("settings.weekly")}</strong><span class="faint">{t("settings.weeklyHint")}</span></span>
    </label>
    <div class="field">
      <label for="low">{t("settings.lowSpace")}</label>
      <input id="low" class="input narrow" type="number" min="1" max="1000" value={s.lowSpaceThresholdGb} onchange={(e) => app.saveSettings({ lowSpaceThresholdGb: num(e, 1, 1000) })} />
    </div>
  </div>

  <AiSettings />

  <div class="card group">
    <h2>{t("settings.advanced")}</h2>
    {#if app.info?.debugBuild}
      <div class="field">
        <label for="sandbox">{t("settings.sandbox")}</label>
        <input
          id="sandbox"
          class="input"
          dir="ltr"
          placeholder="T:\"
          value={s.devSandbox ?? ""}
          onchange={(e) => app.saveSettings({ devSandbox: e.currentTarget.value.trim() || null })}
        />
      </div>
      <p class="faint">{t("settings.sandboxHint")}</p>
    {/if}
    <label class="toggle">
      <input type="checkbox" checked={!s.onboardingDone} onchange={(e) => app.saveSettings({ onboardingDone: !e.currentTarget.checked })} />
      <span><strong>{t("settings.showOnboarding")}</strong></span>
    </label>
    <div>
      <button class="btn danger" onclick={() => (confirmReset = true)}><Icon name="trash" size={15} />{t("settings.reset")}</button>
    </div>
    <p class="faint">{t("settings.resetHint")}</p>
  </div>
</section>

{#if confirmReset}
  <Modal title={t("settings.reset")} onclose={() => (confirmReset = false)} width={460}>
    <p>{t("settings.resetConfirm")}</p>
    {#snippet footer()}
      <button class="btn" onclick={() => (confirmReset = false)}>{t("common.cancel")}</button>
      <button class="btn danger" onclick={reset}>{t("settings.resetYes")}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .page {
    max-width: 760px;
    margin: 0 auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .page :global(.group) {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .page :global(.field) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .page :global(.field .input:not(.narrow)) {
    flex: 1;
    max-width: 320px;
  }
  .narrow {
    width: 90px;
  }
  .page :global(.toggle) {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }
  .page :global(.toggle > span) {
    display: flex;
    flex-direction: column;
  }
  .page :global(.toggle input) {
    margin-top: 5px;
  }
  .paths {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .paths li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
  }
</style>
