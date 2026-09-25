<script lang="ts">
  import { formatSize, truncateMiddle } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import Spinner from "../common/Spinner.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  const p = $derived(cleanup.progress);
</script>

<section class="wrap">
  <div class="card panel" role="status" aria-live="polite">
    <Spinner size={40} />
    <h2>{t("cleanup.running")}</h2>
    {#if p}
      <p class="muted num">{t("cleanup.step", { n: p.index + 1, total: p.total })}</p>
      <UsageBar value={(p.index + 0.5) / p.total} height={8} label={t("cleanup.running")} />
      <p class="path cur" title={p.currentPath}>{truncateMiddle(p.currentPath, 70)}</p>
      <p class="num">{t("cleanup.freedSoFar", { size: formatSize(p.bytesFreed, i18n.lang) })}</p>
    {:else if cleanup.options.createRestorePoint}
      <p class="muted">{t("cleanup.makingRestorePoint")}</p>
    {/if}
    <p class="faint small">{t("cleanup.inUseSkipped")}</p>
  </div>
</section>

<style>
  .wrap {
    display: grid;
    place-items: center;
    padding: var(--sp-12) var(--sp-6);
  }

  .panel {
    width: min(560px, 100%);
    padding: var(--sp-10) var(--sp-8);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-3);
    text-align: center;
    color: var(--accent);
  }

  h2,
  p {
    color: var(--text);
  }

  .cur {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .small {
    font-size: var(--fs-xs);
  }
</style>
