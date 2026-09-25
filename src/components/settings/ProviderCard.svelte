<script lang="ts">
  import { backend } from "../../lib/api/client";
  import { toApiError } from "../../lib/api/errors";
  import type { AiProviderStatus, ApiError } from "../../lib/api/types";
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import { ai } from "../../lib/stores/ai.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { status }: { status: AiProviderStatus } = $props();

  const CUSTOM = "__custom__";
  const id = $derived(`prov-${status.provider}`);
  let key = $state("");
  let saving = $state(false);
  let testing = $state(false);
  let test = $state<{ ok: true } | { ok: false; error: ApiError } | null>(null);
  let custom = $state("");
  let modelChoice = $state("");

  $effect(() => {
    const m = status.model;
    const known = status.availableModels.includes(m);
    modelChoice = known ? m : CUSTOM;
    custom = known ? "" : m;
  });

  const isDefault = $derived(settings.value?.aiDefaultProvider === status.provider);

  async function saveKey() {
    if (!key.trim()) return;
    saving = true;
    try {
      await backend.aiSetKey(status.provider, key.trim());
      key = "";
      test = null;
      await ai.loadProviders();
      if (!settings.value?.aiDefaultProvider) await settings.update({ aiDefaultProvider: status.provider });
      toasts.push("success", t("aiSettings.keySavedToast"));
    } catch (e) {
      toasts.error(e);
    } finally {
      saving = false;
    }
  }

  async function removeKey() {
    try {
      await backend.aiDeleteKey(status.provider);
      test = null;
      await ai.loadProviders();
      if (isDefault) await settings.update({ aiDefaultProvider: null });
    } catch (e) {
      toasts.error(e);
    }
  }

  async function runTest() {
    testing = true;
    test = null;
    try {
      await backend.aiTestKey(status.provider);
      test = { ok: true };
    } catch (e) {
      test = { ok: false, error: toApiError(e) };
    } finally {
      testing = false;
    }
  }

  async function setModel(m: string) {
    if (!m.trim()) return;
    try {
      await backend.aiSetModel(status.provider, m.trim());
      await ai.loadProviders();
    } catch (e) {
      toasts.error(e);
    }
  }
</script>

<div class="card prov" class:def={isDefault}>
  <div class="head">
    <h4>{t(`aiSettings.providers.${status.provider}`)}</h4>
    {#if status.hasKey}
      <span class="saved"><Icon name="check-circle" size={14} />{t("aiSettings.keySaved")}</span>
    {:else}
      <span class="faint small">{t("aiSettings.noKey")}</span>
    {/if}
    <label class="radio">
      <input
        type="radio"
        name="default-provider"
        checked={isDefault}
        disabled={!status.hasKey}
        onchange={() => settings.update({ aiDefaultProvider: status.provider })}
      />
      {t("aiSettings.default")}
    </label>
  </div>

  <div class="keyrow">
    <label class="sr-only" for="{id}-key">{t("aiSettings.keyLabel")}</label>
    <input
      id="{id}-key"
      class="field ltr"
      type="password"
      autocomplete="off"
      spellcheck="false"
      placeholder={status.hasKey ? t("aiSettings.replaceKey") : t("aiSettings.pasteKey")}
      bind:value={key}
      onkeydown={(e) => e.key === "Enter" && saveKey()}
    />
    <button type="button" class="btn btn-sm" disabled={!key.trim() || saving} onclick={saveKey}>
      {#if saving}<Spinner size={14} />{/if}{t("aiSettings.saveKey")}
    </button>
    {#if status.hasKey}
      <button type="button" class="btn btn-sm" disabled={testing} onclick={runTest}>
        {#if testing}<Spinner size={14} />{/if}{t("aiSettings.test")}
      </button>
      <button type="button" class="icon-btn" aria-label={t("aiSettings.removeKey")} title={t("aiSettings.removeKey")} onclick={removeKey}>
        <Icon name="trash" size={16} />
      </button>
    {/if}
  </div>

  {#if test}
    <p class="test" class:ok={test.ok} role="status">
      <Icon name={test.ok ? "check-circle" : "alert"} size={15} />
      {test.ok ? t("aiSettings.testOk") : errorText(test.error)}
    </p>
  {/if}

  <div class="model">
    <label for="{id}-model">{t("aiSettings.model")}</label>
    <select
      id="{id}-model"
      class="field"
      bind:value={modelChoice}
      onchange={() => modelChoice !== CUSTOM && setModel(modelChoice)}
    >
      {#each status.availableModels as m (m)}<option value={m}>{m}</option>{/each}
      <option value={CUSTOM}>{t("aiSettings.customModel")}</option>
    </select>
    {#if modelChoice === CUSTOM}
      <input
        class="field ltr"
        aria-label={t("aiSettings.customModel")}
        placeholder={t("aiSettings.customModelHint")}
        bind:value={custom}
        onblur={() => setModel(custom)}
        onkeydown={(e) => e.key === "Enter" && setModel(custom)}
      />
    {/if}
  </div>
</div>

<style>
  .prov {
    padding: var(--sp-4);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    box-shadow: none;
  }

  .def {
    border-color: var(--accent);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  h4 {
    font-size: var(--fs-md);
  }

  .saved {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-xs);
    color: var(--safe);
  }

  .small {
    font-size: var(--fs-xs);
  }

  .radio {
    margin-inline-start: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
  }

  .radio input {
    accent-color: var(--accent);
  }

  .keyrow {
    display: flex;
    gap: var(--sp-2);
    align-items: center;
  }

  .keyrow .field {
    flex: 1;
  }

  .test {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
    color: var(--danger);
  }

  .test.ok {
    color: var(--safe);
  }

  .model {
    display: grid;
    grid-template-columns: auto 1fr 1fr;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-sm);
  }
</style>
