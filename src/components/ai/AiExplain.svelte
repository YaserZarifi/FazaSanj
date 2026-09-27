<script lang="ts">
  // "Ask AI" for a folder nothing else could explain. Shows the exact payload before the first
  // request and a warning on every answer.
  import * as api from "../../lib/api/commands";
  import type { AiAnswer, NodeId, ScanId } from "../../lib/api/types";
  import { formatRelative } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Modal from "../common/Modal.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { scanId, nodeId }: { scanId: ScanId; nodeId: NodeId } = $props();

  let answer = $state<AiAnswer | null>(null);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let preview = $state<string | null>(null);

  $effect(() => {
    void [scanId, nodeId];
    answer = null;
    error = null;
  });

  async function showPreview() {
    try {
      preview = JSON.stringify(await api.aiPreviewPayload(scanId, nodeId), null, 2);
    } catch (e) {
      error = errorText(e);
    }
  }

  async function ask() {
    if (!app.settings.aiPreviewAcknowledged) {
      await showPreview();
      return;
    }
    busy = true;
    error = null;
    try {
      answer = await api.aiExplain(scanId, nodeId, lang());
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  async function confirmPreview() {
    const first = !app.settings.aiPreviewAcknowledged;
    preview = null;
    if (first) {
      await app.saveSettings({ aiPreviewAcknowledged: true });
      await ask();
    }
  }
</script>

{#if answer}
  <div class="answer">
    <div class="row">
      <span class="ai-badge"><Icon name="sparkles" size={13} />{t("ai.badge")}</span>
      {#if answer.cached}<span class="faint">{t("ai.cached", { when: formatRelative(answer.createdAt, lang()) })}</span>{/if}
      <span class="spacer"></span>
      <SafetyBadge level={answer.safety} small />
    </div>
    <p><strong>{answer.what}</strong></p>
    <p>{answer.whyBig}</p>
    <h4>{t("reason.whatHappens")}</h4>
    <p>{answer.consequence}</p>
    <div class="notice warn small"><Icon name="alert" size={16} />{t("ai.warning")}</div>
  </div>
{:else}
  <div class="row wrap">
    <button class="btn small" onclick={ask} disabled={busy}>
      {#if busy}<Spinner size={14} />{:else}<Icon name="sparkles" size={14} />{/if}
      {t("ai.ask")}
    </button>
    <button class="btn ghost small" onclick={showPreview}>{t("ai.whatIsSent")}</button>
  </div>
  {#if error}<div class="notice error small">{error}</div>{/if}
{/if}

{#if preview !== null}
  <Modal title={t("ai.previewTitle")} onclose={() => (preview = null)} width={620}>
    <p>{t("ai.previewBody")}</p>
    <pre class="payload">{preview}</pre>
    <div class="notice warn"><Icon name="alert" size={16} />{t("ai.warning")}</div>
    {#snippet footer()}
      <button class="btn" onclick={() => (preview = null)}>{t("common.cancel")}</button>
      {#if !app.settings.aiPreviewAcknowledged}
        <button class="btn primary" onclick={confirmPreview}>{t("ai.sendThis")}</button>
      {/if}
    {/snippet}
  </Modal>
{/if}

<style>
  .answer {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .ai-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0 8px;
    border-radius: 99px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
  }
  h4 {
    font-size: 13px;
    color: var(--text-2);
  }
  .small {
    font-size: 13px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .payload {
    direction: ltr;
    text-align: left;
    background: var(--surface-2);
    padding: 12px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    max-height: 320px;
    overflow: auto;
    margin: 0;
  }
</style>
