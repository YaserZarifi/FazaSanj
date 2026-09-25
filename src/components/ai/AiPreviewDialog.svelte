<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { ai } from "../../lib/stores/ai.svelte";
  import AiWarning from "../common/AiWarning.svelte";
  import Dialog from "../common/Dialog.svelte";

  const json = $derived(ai.preview ? JSON.stringify(ai.preview.payload, null, 2) : "");
</script>

<Dialog open={ai.preview != null} title={t("ai.previewTitle")} width={620} onclose={() => ai.cancelPreview()}>
  <p class="muted">{t("ai.previewText")}</p>
  <!-- focusable so keyboard users can scroll a long payload -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <pre class="ltr" tabindex="0" aria-label={t("ai.previewTitle")}>{json}</pre>
  <AiWarning />
  {#snippet footer()}
    <button type="button" class="btn" onclick={() => ai.cancelPreview()}>{t("common.cancel")}</button>
    <button type="button" class="btn btn-primary" onclick={() => ai.confirmPreview()}>{t("ai.previewConfirm")}</button>
  {/snippet}
</Dialog>

<style>
  pre {
    margin: var(--sp-3) 0;
    padding: var(--sp-4);
    max-height: 340px;
    overflow: auto;
    border-radius: var(--r-md);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 12px;
    text-align: left;
  }
</style>
