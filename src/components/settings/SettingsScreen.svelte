<script lang="ts">
  import { backend } from "../../lib/api/client";
  import { pickFolder } from "../../lib/api/platform";
  import type { Language, ThemePref, UiMode } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { ai } from "../../lib/stores/ai.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import Dialog from "../common/Dialog.svelte";
  import Icon from "../common/Icon.svelte";
  import Segmented from "../common/Segmented.svelte";
  import Toggle from "../common/Toggle.svelte";
  import AiSettings from "./AiSettings.svelte";
  import SettingRow from "./SettingRow.svelte";

  const s = $derived(settings.value);
  const info = $derived(settings.info);
  let resetOpen = $state(false);
  let resetText = $state("");
  let resetting = $state(false);
  let sandbox = $state("");

  $effect(() => {
    sandbox = settings.value?.devSandbox ?? "";
  });

  function num(v: string, min: number, max: number, fallback: number): number {
    const n = Math.round(Number(v));
    return Number.isFinite(n) ? Math.min(max, Math.max(min, n)) : fallback;
  }

  async function addExcluded() {
    try {
      const p = await pickFolder();
      if (p && s && !s.excludedPaths.includes(p)) await settings.update({ excludedPaths: [...s.excludedPaths, p] });
    } catch (e) {
      toasts.error(e);
    }
  }

  async function doReset() {
    resetting = true;
    try {
      await backend.resetEverything();
      await settings.load();
      await ai.loadProviders();
      scan.reset();
      resetOpen = false;
      resetText = "";
      toasts.push("success", t("settings.resetDone"));
    } catch (e) {
      toasts.error(e);
    } finally {
      resetting = false;
    }
  }

  const resetWord = $derived(t("settings.resetWord"));
</script>

{#if s}
  <div class="page">
    <header>
      <h2>{t("settings.title")}</h2>
    </header>

    <section class="card sec" aria-labelledby="st-general">
      <h3 id="st-general" class="section-title">{t("settings.general")}</h3>
      <SettingRow label={t("settings.language")}>
        <Segmented
          label={t("settings.language")}
          value={s.language}
          options={[
            { value: "fa" as Language, label: "فارسی" },
            { value: "en" as Language, label: "English" },
          ]}
          onchange={(v) => settings.update({ language: v })}
        />
      </SettingRow>
      <SettingRow label={t("settings.theme")}>
        <Segmented
          label={t("settings.theme")}
          value={s.theme}
          options={[
            { value: "system" as ThemePref, label: t("theme.system"), icon: "monitor" },
            { value: "light" as ThemePref, label: t("theme.light"), icon: "sun" },
            { value: "dark" as ThemePref, label: t("theme.dark"), icon: "moon" },
          ]}
          onchange={(v) => settings.update({ theme: v })}
        />
      </SettingRow>
      <SettingRow label={t("settings.defaultMode")} hint={t("settings.defaultModeHint")}>
        <Segmented
          label={t("settings.defaultMode")}
          value={s.defaultMode}
          options={[
            { value: "simple" as UiMode, label: t("top.simple") },
            { value: "expert" as UiMode, label: t("top.expert") },
          ]}
          onchange={(v) => settings.update({ defaultMode: v })}
        />
      </SettingRow>
    </section>

    <section class="card sec" aria-labelledby="st-scan">
      <h3 id="st-scan" class="section-title">{t("settings.scanning")}</h3>
      <div class="block">
        <p class="lbl">{t("settings.excluded")}</p>
        <p class="muted hint">{t("settings.excludedHint")}</p>
        {#if s.excludedPaths.length === 0}
          <p class="faint none">{t("settings.excludedNone")}</p>
        {:else}
          <ul class="excl">
            {#each s.excludedPaths as p (p)}
              <li>
                <span class="path">{p}</span>
                <button
                  type="button"
                  class="icon-btn"
                  aria-label={t("settings.removeExcluded", { path: p })}
                  onclick={() => settings.update({ excludedPaths: s.excludedPaths.filter((x) => x !== p) })}
                >
                  <Icon name="x" size={16} />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
        <button type="button" class="btn btn-sm" onclick={addExcluded}><Icon name="plus" size={15} />{t("settings.addExcluded")}</button>
      </div>
      <SettingRow label={t("settings.staleMonths")} hint={t("settings.staleMonthsHint")} forId="st-stale">
        <input id="st-stale" class="field numf num" type="number" min="1" max="120" value={s.staleMonths} onchange={(e) => settings.update({ staleMonths: num(e.currentTarget.value, 1, 120, s.staleMonths) })} />
      </SettingRow>
      <SettingRow label={t("settings.oldProjectMonths")} hint={t("settings.oldProjectMonthsHint")} forId="st-proj">
        <input id="st-proj" class="field numf num" type="number" min="1" max="120" value={s.oldProjectMonths} onchange={(e) => settings.update({ oldProjectMonths: num(e.currentTarget.value, 1, 120, s.oldProjectMonths) })} />
      </SettingRow>
    </section>

    <section class="card sec" aria-labelledby="st-bg">
      <h3 id="st-bg" class="section-title">{t("settings.background")}</h3>
      <SettingRow label={t("settings.lowSpace")} hint={t("settings.lowSpaceHint")} forId="st-low">
        <input id="st-low" class="field numf num" type="number" min="1" max="1000" value={s.lowSpaceThresholdGb} onchange={(e) => settings.update({ lowSpaceThresholdGb: num(e.currentTarget.value, 1, 1000, s.lowSpaceThresholdGb) })} />
      </SettingRow>
      <div class="tg"><Toggle checked={s.trayEnabled} label={t("settings.tray")} description={t("settings.trayHint")} onchange={(v) => settings.update({ trayEnabled: v })} /></div>
      <div class="tg"><Toggle checked={s.weeklyCheck} label={t("settings.weekly")} description={t("settings.weeklyHint")} disabled={!s.trayEnabled} onchange={(v) => settings.update({ weeklyCheck: v })} /></div>
    </section>

    <section class="card sec" aria-labelledby="st-ai">
      <h3 id="st-ai" class="section-title">{t("settings.ai")}</h3>
      <AiSettings />
    </section>

    <section class="card sec" aria-labelledby="st-data">
      <h3 id="st-data" class="section-title">{t("settings.data")}</h3>
      {#if info}
        <SettingRow label={t("settings.dataDir")} hint={t("settings.dataDirHint")}>
          <button type="button" class="btn btn-sm" onclick={() => backend.revealInExplorer(info.dataDir).catch((e: unknown) => toasts.error(e))}>
            <Icon name="folder-open" size={15} />{t("common.open")}
          </button>
        </SettingRow>
        <p class="path dir">{info.dataDir}</p>
        {#if info.debugBuild}
          <SettingRow label={t("settings.sandbox")} hint={t("settings.sandboxHint")} forId="st-sb">
            <div class="sb">
              <input id="st-sb" class="field ltr" placeholder="T:\" bind:value={sandbox} />
              <button type="button" class="btn btn-sm" onclick={() => settings.update({ devSandbox: sandbox.trim() || null })}>{t("common.save")}</button>
            </div>
          </SettingRow>
        {/if}
      {/if}
    </section>

    <section class="card sec danger" aria-labelledby="st-reset">
      <h3 id="st-reset" class="section-title">{t("settings.reset")}</h3>
      <SettingRow label={t("settings.resetLabel")} hint={t("settings.resetHint")}>
        <button type="button" class="btn btn-sm btn-danger" onclick={() => (resetOpen = true)}>{t("settings.resetButton")}</button>
      </SettingRow>
    </section>
  </div>

  <Dialog open={resetOpen} title={t("settings.resetTitle")} tone="danger" onclose={() => (resetOpen = false)}>
    <p>{t("settings.resetConfirmText")}</p>
    <label class="confirm">
      <span>{t("settings.resetType", { word: resetWord })}</span>
      <input class="field" bind:value={resetText} />
    </label>
    {#snippet footer()}
      <button type="button" class="btn" onclick={() => (resetOpen = false)}>{t("common.cancel")}</button>
      <button type="button" class="btn btn-danger" disabled={resetText.trim() !== resetWord || resetting} onclick={doReset}>{t("settings.resetButton")}</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .page {
    max-width: 860px;
    padding: var(--sp-6) var(--sp-6) var(--sp-12);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .sec {
    padding: var(--sp-5) var(--sp-6);
    display: flex;
    flex-direction: column;
  }

  .sec > h3 {
    margin-bottom: var(--sp-2);
  }

  .block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
    padding: var(--sp-3) 0;
  }

  .lbl {
    font-weight: var(--fw-medium);
  }

  .hint {
    font-size: var(--fs-sm);
  }

  .none {
    font-size: var(--fs-sm);
  }

  .excl {
    list-style: none;
    margin: 0;
    padding: 0;
    width: 100%;
  }

  .excl li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--sp-2);
    padding: 4px var(--sp-3);
    border-radius: var(--r-md);
    background: var(--surface-2);
    margin-bottom: 4px;
  }

  .numf {
    width: 90px;
  }

  .tg {
    padding: var(--sp-3) 0;
  }

  .dir {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .sb {
    display: flex;
    gap: var(--sp-2);
  }

  .sb .field {
    width: 180px;
  }

  .danger {
    border-color: color-mix(in srgb, var(--danger) 35%, var(--border));
  }

  .confirm {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: var(--sp-3);
    font-size: var(--fs-sm);
  }
</style>
