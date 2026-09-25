<script lang="ts">
  import { formatClock, formatCompact, formatSize, truncateMiddle } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  const p = $derived(scan.progress);
  const drive = $derived(scan.target?.drive ?? null);
  // For whole drives the used space is a good estimate of where we will end up.
  const expected = $derived(drive && !scan.target?.isFolder ? drive.total - drive.free : 0);
  const ratio = $derived(expected > 0 && p ? Math.min(0.99, p.bytes / expected) : 0);
</script>

<section class="wrap">
  <div class="card panel">
    <div class="ring" class:fast={p?.scanner === "fast"} aria-hidden="true">
      <svg viewBox="0 0 120 120">
        <circle cx="60" cy="60" r="52" class="track" />
        <circle cx="60" cy="60" r="52" class="arc" />
      </svg>
      <span class="center"><Icon name={p?.scanner === "fast" ? "zap" : "search"} size={30} /></span>
    </div>

    <h2>{scan.phase === "starting" && !p ? t("scanning.starting") : t("scanning.title")}</h2>
    {#if p}
      <p class="badge">{p.scanner === "fast" ? t("scanning.fastMode") : t("scanning.normalMode")}</p>
    {/if}

    <div class="stats" aria-live="polite">
      <div class="stat">
        <span class="v num">{formatSize(p?.bytes ?? 0, i18n.lang)}</span>
        <span class="l">{t("scanning.counted")}</span>
      </div>
      <div class="stat">
        <span class="v num">{formatCompact(p?.files ?? 0, i18n.lang)}</span>
        <span class="l">{t("scanning.files")}</span>
      </div>
      <div class="stat">
        <span class="v num">{formatCompact(p?.dirs ?? 0, i18n.lang)}</span>
        <span class="l">{t("scanning.folders")}</span>
      </div>
      <div class="stat">
        <span class="v num">{formatClock(p?.elapsedMs ?? 0, i18n.lang)}</span>
        <span class="l">{t("scanning.elapsed")}</span>
      </div>
    </div>

    {#if expected > 0}
      <div class="bar"><UsageBar value={ratio} height={8} label={t("scanning.progressAria")} /></div>
    {/if}

    <p class="current" title={p?.currentPath ?? ""}>
      {#if p}
        <span class="path">{truncateMiddle(p.currentPath, 72)}</span>
      {:else}
        <Spinner size={14} />
      {/if}
    </p>

    <button type="button" class="btn" onclick={() => scan.cancel()} disabled={scan.cancelling || scan.scanId == null}>
      {#if scan.cancelling}<Spinner size={14} />{t("scanning.cancelling")}{:else}<Icon name="stop" size={15} />{t("scanning.cancel")}{/if}
    </button>
    <p class="faint small">{t("scanning.readOnly")}</p>
  </div>
</section>

<style>
  .wrap {
    display: grid;
    place-items: center;
    padding: var(--sp-10) var(--sp-6);
  }

  .panel {
    width: min(640px, 100%);
    padding: var(--sp-10) var(--sp-8);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-3);
    text-align: center;
  }

  .ring {
    position: relative;
    width: 120px;
    height: 120px;
    margin-bottom: var(--sp-2);
  }

  .ring svg {
    width: 100%;
    height: 100%;
    animation: spin 1.6s linear infinite;
  }

  .track {
    fill: none;
    stroke: var(--surface-3);
    stroke-width: 8;
  }

  .arc {
    fill: none;
    stroke: var(--accent);
    stroke-width: 8;
    stroke-linecap: round;
    stroke-dasharray: 90 240;
  }

  .fast .arc {
    stroke: var(--probably-safe);
  }

  .fast svg {
    animation-duration: 0.9s;
  }

  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: var(--accent-text);
  }

  .fast .center {
    color: var(--probably-safe);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  h2 {
    font-size: var(--fs-xl);
  }

  .badge {
    font-size: var(--fs-xs);
    padding: 1px 10px;
    border-radius: var(--r-full);
    background: var(--surface-3);
    color: var(--text-2);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--sp-3);
    width: 100%;
    margin-top: var(--sp-4);
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--sp-3);
    border-radius: var(--r-md);
    background: var(--surface-2);
  }

  .v {
    font-size: var(--fs-xl);
    font-weight: var(--fw-bold);
  }

  .l {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .bar {
    width: 100%;
    margin-top: var(--sp-2);
  }

  .current {
    min-height: 24px;
    width: 100%;
    color: var(--text-3);
    font-size: var(--fs-xs);
    overflow: hidden;
    white-space: nowrap;
  }

  .small {
    font-size: var(--fs-xs);
  }
</style>
