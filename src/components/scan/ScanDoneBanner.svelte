<script lang="ts">
  import { formatCompact, formatDuration, formatNumber } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";

  const s = $derived(scan.summary);
  let hideFallback = $state(false);
</script>

{#if s}
  <div class="banners">
    <div class="done" role="status">
      <Icon name="check-circle" size={18} />
      <span>
        {t("done.scanned", { files: formatCompact(s.files, i18n.lang), time: formatDuration(s.durationMs, i18n.lang) })}
        <span class="faint">· {s.scanner === "fast" ? t("scanning.fastMode") : t("scanning.normalMode")}</span>
      </span>
      {#if s.accessDenied > 0}
        <button type="button" class="link" onclick={() => { ui.mode = "expert"; ui.expertTab = "denied"; }}>
          {t("done.denied", { count: s.accessDenied, n: formatNumber(s.accessDenied, i18n.lang) })}
        </button>
      {/if}
      <button type="button" class="btn btn-sm btn-ghost rescan" onclick={() => scan.target && scan.choose(scan.target)}>
        <Icon name="refresh" size={15} />{t("done.rescan")}
      </button>
    </div>

    {#if s.fallbackReason && !hideFallback}
      <div class="fallback" role="note">
        <Icon name="info" size={18} />
        <p>{t(`fallback.${s.fallbackReason}`)}</p>
        <button type="button" class="icon-btn" aria-label={t("common.dismiss")} onclick={() => (hideFallback = true)}>
          <Icon name="x" size={16} />
        </button>
      </div>
    {/if}

    {#if scan.version > 0}
      <div class="fallback stale" role="note">
        <Icon name="refresh" size={18} />
        <p>{t("done.stale")}</p>
        <button type="button" class="btn btn-sm" onclick={() => scan.target && scan.choose(scan.target)}>{t("done.rescan")}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .banners {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-4) var(--sp-6) 0;
  }

  .done {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sp-2) var(--sp-3);
    font-size: var(--fs-sm);
    color: var(--text-2);
  }

  .done > :global(svg) {
    color: var(--safe);
  }

  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--accent-text);
    font-size: var(--fs-sm);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .rescan {
    margin-inline-start: auto;
  }

  .fallback {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .fallback p {
    flex: 1;
    color: var(--text);
    font-size: var(--fs-sm);
  }

  .stale {
    background: var(--probably-safe-soft);
    color: var(--probably-safe);
  }
</style>
