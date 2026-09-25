<script lang="ts">
  import type { Reason } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { bi, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import CategoryIcon from "../common/CategoryIcon.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import SourceBadge from "../common/SourceBadge.svelte";

  let { reason, rank }: { reason: Reason; rank: number } = $props();
  let open = $state(false);
  const e = $derived(reason.explanation);
  const inBasket = $derived(cleanup.has(reason.path));
  const canClean = $derived(e.safety !== "do_not_touch" && e.method !== "manual_only");
  const panelId = `rc-${Math.random().toString(36).slice(2, 8)}`;
</script>

<article class="card reason" style:--delay="{rank * 50}ms">
  <div class="top">
    <CategoryIcon category={reason.category} size={42} />
    <div class="main">
      <div class="line">
        <h4>{bi(e.title)}</h4>
        <span class="size num">{formatSize(reason.bytes, i18n.lang)}</span>
      </div>
      <PathText path={reason.path} max={64} />
      <p class="why">{bi(e.whyBig)}</p>
      <div class="meta">
        <SafetyBadge level={e.safety} />
        {#if e.source !== "knowledge_base"}<SourceBadge source={e.source} />{/if}
        {#if e.needsAdmin}<span class="admin" title={t("cleanup.needsAdmin")}><Icon name="shield" size={14} />{t("cleanup.adminShort")}</span>{/if}
      </div>
    </div>
  </div>

  <div class="actions">
    <button type="button" class="btn btn-sm btn-ghost more" aria-expanded={open} aria-controls={panelId} onclick={() => (open = !open)}>
      <Icon name="chevron-down" size={16} class={open ? "rot" : ""} />
      {t("reason.whatIf")}
    </button>
    {#if canClean}
      <button
        type="button"
        class="btn btn-sm"
        class:added={inBasket}
        aria-pressed={inBasket}
        onclick={() => cleanup.toggle({ path: reason.path, bytes: reason.bytes, explanation: e })}
      >
        <Icon name={inBasket ? "check" : "plus"} size={15} />
        {inBasket ? t("reason.inCleanup") : t("reason.addToCleanup")}
      </button>
    {/if}
  </div>

  {#if open}
    <div class="expand fade-in" id={panelId}>
      <p>{bi(e.ifDeleted)}</p>
      {#if e.instructions}
        <p class="instr"><Icon name="info" size={15} /> <span>{bi(e.instructions)}</span></p>
      {/if}
    </div>
  {/if}
</article>

<style>
  .reason {
    padding: var(--sp-4) var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    animation: fade-in var(--dur-slow) var(--ease) both;
    animation-delay: var(--delay);
  }

  .top {
    display: flex;
    gap: var(--sp-4);
  }

  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .line {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-3);
  }

  h4 {
    font-size: var(--fs-lg);
  }

  .size {
    font-size: var(--fs-xl);
    font-weight: var(--fw-bold);
    white-space: nowrap;
  }

  .why {
    color: var(--text-2);
    font-size: var(--fs-sm);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2);
    margin-top: 4px;
  }

  .admin {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .actions {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-2);
    padding-top: var(--sp-2);
    border-top: 1px solid var(--border);
  }

  .more {
    color: var(--accent-text);
  }

  .more :global(.rot) {
    transform: rotate(180deg);
  }

  .more :global(svg) {
    transition: transform var(--dur-med) var(--ease);
  }

  .added {
    background: var(--safe-soft);
    border-color: transparent;
    color: var(--safe);
  }

  .expand {
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--surface-2);
    font-size: var(--fs-sm);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }

  .instr {
    display: flex;
    gap: var(--sp-2);
    color: var(--text-2);
  }

  .instr :global(svg) {
    margin-top: 4px;
  }
</style>
