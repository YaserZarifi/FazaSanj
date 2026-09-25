<script lang="ts">
  import { formatDuration, formatInt, formatSize } from "../../lib/format";
  import { lang, t } from "../../lib/i18n/index.svelte";
  import { app, type ScanSession } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { session }: { session: ScanSession } = $props();
  const p = $derived(session.progress);
  let cancelling = $state(false);

  async function cancel() {
    cancelling = true;
    await app.cancelScan(session.root);
  }
</script>

<section class="progress">
  <div class="spin"><Spinner size={48} /></div>
  <h2>{t("progress.title")}</h2>
  <p class="muted">
    {p?.scanner === "fast" ? t("progress.fast") : session.mode === "fast" ? t("progress.waitingUac") : t("progress.normal")}
  </p>

  <div class="stats">
    <div class="stat">
      <span class="faint">{t("progress.files")}</span>
      <strong>{formatInt(p?.files ?? 0, lang())}</strong>
    </div>
    <div class="stat">
      <span class="faint">{t("progress.folders")}</span>
      <strong>{formatInt(p?.dirs ?? 0, lang())}</strong>
    </div>
    <div class="stat">
      <span class="faint">{t("progress.size")}</span>
      <strong>{formatSize(p?.bytes ?? 0, lang())}</strong>
    </div>
    <div class="stat">
      <span class="faint">{t("progress.elapsed")}</span>
      <strong>{formatDuration(p?.elapsedMs ?? 0, lang())}</strong>
    </div>
  </div>

  <div class="current">
    {#if p?.currentPath}<bdi class="path">{p.currentPath}</bdi>{:else}&nbsp;{/if}
  </div>

  <button class="btn" onclick={cancel} disabled={cancelling}>
    <Icon name="stop" size={14} />
    {cancelling ? t("progress.cancelling") : t("progress.cancel")}
  </button>
</section>

<style>
  .progress {
    max-width: 640px;
    width: 100%;
    margin: 0 auto;
    padding: 56px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 14px;
    text-align: center;
  }
  .spin {
    margin-bottom: 8px;
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
    width: 100%;
    margin-top: 10px;
  }
  .stat {
    display: flex;
    flex-direction: column;
    padding: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .stat strong {
    font-size: 18px;
    font-variant-numeric: tabular-nums;
  }
  .current {
    width: 100%;
    min-height: 24px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text-3);
  }
</style>
