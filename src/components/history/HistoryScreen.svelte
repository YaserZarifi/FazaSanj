<script lang="ts">
  import { onMount } from "svelte";
  import { backend } from "../../lib/api/client";
  import type { HistoryEntry } from "../../lib/api/types";
  import { formatDate, formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import StatusBadge from "./StatusBadge.svelte";

  const PAGE = 20;
  let entries = $state<HistoryEntry[] | null>(null);
  let more = $state(false);

  async function load(reset: boolean) {
    try {
      const list = await backend.getCleanupHistory(PAGE, reset ? 0 : (entries?.length ?? 0));
      entries = reset ? list : [...(entries ?? []), ...list];
      more = list.length === PAGE;
    } catch (e) {
      entries = entries ?? [];
      toasts.error(e);
    }
  }

  onMount(() => void load(true));

  const anyRestorable = $derived(entries?.some((e) => e.actions.some((a) => a.restorable)) ?? false);

  function openBin() {
    backend.openRecycleBin().catch((e: unknown) => toasts.error(e));
  }
</script>

<div class="page">
  <header>
    <div>
      <h2>{t("history.title")}</h2>
      <p class="muted">{t("history.subtitle")}</p>
    </div>
    {#if anyRestorable}
      <button type="button" class="btn" onclick={openBin}><Icon name="recycle" size={16} />{t("history.openBin")}</button>
    {/if}
  </header>

  {#if entries == null}
    {#each [0, 1] as i (i)}<Skeleton height={140} radius="var(--r-lg)" />{/each}
  {:else if entries.length === 0}
    <EmptyState icon="history" title={t("history.emptyTitle")} text={t("history.emptyText")} />
  {:else}
    {#each entries as e (e.runId)}
      <article class="card run">
        <div class="head">
          <span class="ic" class:dry={e.dryRun}><Icon name={e.dryRun ? "eye" : "broom"} size={18} /></span>
          <div class="hmain">
            <h3>{e.dryRun ? t("history.dryRun") : t("history.cleanup")}</h3>
            <p class="faint small">{formatDate(e.startedAt, i18n.lang, true)}</p>
          </div>
          {#if !e.dryRun}
            <p class="freed num">{t("history.freed", { size: formatSize(e.bytesFreed, i18n.lang) })}</p>
          {/if}
        </div>
        <ul>
          {#each e.actions as a, i (i)}
            <li>
              <PathText path={a.path} max={64} />
              <span class="tags">
                <StatusBadge status={a.status} />
                {#if a.restorable}<span class="bin"><Icon name="recycle" size={12} />{t("history.inBin")}</span>{/if}
                {#if a.errorCode}<span class="faint small">{t(`errors.${a.errorCode}`)}</span>{/if}
              </span>
              <span class="num size">{formatSize(a.bytes, i18n.lang)}</span>
            </li>
          {/each}
        </ul>
      </article>
    {/each}
    {#if more}
      <button type="button" class="btn more" onclick={() => load(false)}>{t("history.loadMore")}</button>
    {/if}
  {/if}
</div>

<style>
  .page {
    max-width: 920px;
    padding: var(--sp-6) var(--sp-6) var(--sp-12);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: var(--sp-4);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .run {
    padding: var(--sp-5);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    margin-bottom: var(--sp-3);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--r-md);
    background: var(--safe-soft);
    color: var(--safe);
  }

  .ic.dry {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .hmain {
    flex: 1;
  }

  h3 {
    font-size: var(--fs-md);
  }

  .small {
    font-size: var(--fs-xs);
  }

  .freed {
    font-weight: var(--fw-bold);
    color: var(--safe);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 90px;
    align-items: center;
    gap: var(--sp-3);
    padding: 6px 0;
    border-top: 1px solid var(--border);
  }

  .tags {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .bin {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--probably-safe-soft);
    color: var(--probably-safe);
  }

  .size {
    text-align: end;
    font-size: var(--fs-sm);
  }

  .more {
    align-self: center;
  }
</style>
