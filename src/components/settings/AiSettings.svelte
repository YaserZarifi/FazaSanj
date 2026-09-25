<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { AiProvider, AiProviderStatus } from "../../lib/api/types";
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Modal from "../common/Modal.svelte";
  import Spinner from "../common/Spinner.svelte";

  const NAMES: Record<AiProvider, string> = { open_ai: "OpenAI", gemini: "Google Gemini", anthropic: "Anthropic", groq: "Groq" };

  let providers = $state<AiProviderStatus[]>([]);
  let keys = $state<Record<string, string>>({});
  let busy = $state<Record<string, boolean>>({});
  let result = $state<Record<string, { ok: boolean; text: string }>>({});
  let confirmEnable = $state(false);

  async function refresh() {
    try {
      providers = await api.aiGetProviders();
    } catch {
      providers = [];
    }
  }

  $effect(() => {
    refresh();
  });

  async function saveKey(p: AiProvider) {
    const key = (keys[p] ?? "").trim();
    if (!key) return;
    busy[p] = true;
    try {
      await api.aiSetKey(p, key);
      keys[p] = "";
      await refresh();
      await test(p);
    } catch (e) {
      result[p] = { ok: false, text: errorText(e) };
    } finally {
      busy[p] = false;
    }
  }

  async function test(p: AiProvider) {
    busy[p] = true;
    try {
      await api.aiTestKey(p);
      result[p] = { ok: true, text: t("ai.keyWorks") };
    } catch (e) {
      result[p] = { ok: false, text: errorText(e) };
    } finally {
      busy[p] = false;
    }
  }

  async function remove(p: AiProvider) {
    await api.aiDeleteKey(p).catch(() => {});
    delete result[p];
    if (app.settings.aiDefaultProvider === p) app.saveSettings({ aiDefaultProvider: null });
    await refresh();
  }

  async function setModel(p: AiProvider, model: string) {
    await api.aiSetModel(p, model).catch(() => {});
    await refresh();
  }

  function toggle(on: boolean) {
    if (on) confirmEnable = true;
    else app.saveSettings({ aiEnabled: false });
  }
</script>

<div class="card group">
  <h2>{t("ai.title")}</h2>
  <p class="muted">{t("ai.intro")}</p>
  <label class="toggle">
    <input type="checkbox" checked={app.settings.aiEnabled} onchange={(e) => { const on = e.currentTarget.checked; e.currentTarget.checked = app.settings.aiEnabled; toggle(on); }} />
    <span><strong>{t("ai.enable")}</strong><span class="faint">{t("ai.enableHint")}</span></span>
  </label>

  {#if app.settings.aiEnabled}
    <label class="toggle">
      <input type="checkbox" checked={app.settings.aiMaskNames} onchange={(e) => app.saveSettings({ aiMaskNames: e.currentTarget.checked })} />
      <span><strong>{t("ai.mask")}</strong><span class="faint">{t("ai.maskHint")}</span></span>
    </label>
    <div class="field">
      <label for="prov">{t("ai.defaultProvider")}</label>
      <select
        id="prov"
        class="select"
        value={app.settings.aiDefaultProvider ?? ""}
        onchange={(e) => app.saveSettings({ aiDefaultProvider: (e.currentTarget.value || null) as AiProvider | null })}
      >
        <option value="">{t("ai.firstWithKey")}</option>
        {#each providers.filter((p) => p.hasKey) as p (p.provider)}
          <option value={p.provider}>{NAMES[p.provider]}</option>
        {/each}
      </select>
    </div>

    <div class="providers">
      {#each providers as p (p.provider)}
        <div class="provider">
          <div class="row">
            <strong>{NAMES[p.provider]}</strong>
            {#if p.hasKey}<span class="has"><Icon name="key" size={13} />{t("ai.keySaved")}</span>{/if}
            <span class="spacer"></span>
            <select class="select small" value={p.model} onchange={(e) => setModel(p.provider, e.currentTarget.value)} aria-label={t("ai.model")}>
              {#each p.availableModels.includes(p.model) ? p.availableModels : [p.model, ...p.availableModels] as m (m)}
                <option value={m}>{m}</option>
              {/each}
            </select>
          </div>
          <div class="row">
            <input
              class="input grow"
              type="password"
              dir="ltr"
              autocomplete="off"
              placeholder={p.hasKey ? t("ai.replaceKey") : t("ai.pasteKey")}
              bind:value={keys[p.provider]}
              onkeydown={(e) => e.key === "Enter" && saveKey(p.provider)}
            />
            <button class="btn small" disabled={!keys[p.provider]?.trim() || busy[p.provider]} onclick={() => saveKey(p.provider)}>{t("common.save")}</button>
            {#if p.hasKey}
              <button class="btn small" disabled={busy[p.provider]} onclick={() => test(p.provider)}>
                {#if busy[p.provider]}<Spinner size={12} />{/if}{t("ai.test")}
              </button>
              <button class="btn ghost small" onclick={() => remove(p.provider)} aria-label={t("common.remove")} title={t("common.remove")}>
                <Icon name="trash" size={14} />
              </button>
            {/if}
          </div>
          {#if result[p.provider]}
            <span class={result[p.provider].ok ? "ok" : "bad"}>{result[p.provider].text}</span>
          {/if}
        </div>
      {/each}
    </div>
    <p class="faint">{t("ai.keysLocal")}</p>
  {/if}
</div>

{#if confirmEnable}
  <Modal title={t("ai.enableTitle")} onclose={() => (confirmEnable = false)} width={520}>
    <div class="notice warn"><Icon name="alert" />{t("ai.warning")}</div>
    <ul class="points">
      <li>{t("ai.point1")}</li>
      <li>{t("ai.point2")}</li>
      <li>{t("ai.point3")}</li>
      <li>{t("ai.point4")}</li>
    </ul>
    {#snippet footer()}
      <button class="btn" onclick={() => (confirmEnable = false)}>{t("common.cancel")}</button>
      <button class="btn primary" onclick={() => { confirmEnable = false; app.saveSettings({ aiEnabled: true }); }}>{t("ai.enableYes")}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .providers {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .provider {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .grow {
    flex: 1;
  }
  .select.small {
    min-height: 30px;
    font-size: 13px;
  }
  .has {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    font-size: 12px;
    color: var(--safe);
  }
  .ok {
    color: var(--safe);
    font-size: 13px;
  }
  .bad {
    color: var(--danger);
    font-size: 13px;
  }
  .points {
    margin: 0;
    padding-inline-start: 20px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
</style>
