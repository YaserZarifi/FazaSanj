<script lang="ts">
  import { onMount } from "svelte";
  import { backend } from "../../lib/api/client";
  import type { SnapshotComparison, SnapshotInfo } from "../../lib/api/types";
  import { formatDate, formatSize, formatSizeDelta } from "../../lib/format";
  import { bi, i18n, t } from "../../lib/i18n/index.svelte";
  import { growthSentence, lastSegment } from "../../lib/story";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import PathText from "../common/PathText.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import GrowthChart from "./GrowthChart.svelte";

  let all = $state<SnapshotInfo[] | null>(null);
  let root = $state<string | null>(null);
  let fromId = $state<number | null>(null);
  let toId = $state<number | null>(null);
  let cmp = $state<SnapshotComparison | null>(null);
  let cmpLoading = $state(false);
  let chartPath = $state<string | null>(null);

  const roots = $derived([...new Set((all ?? []).map((s) => s.rootPath))]);
  const snaps = $derived((all ?? []).filter((s) => s.rootPath === root).sort((a, b) => b.takenAt - a.takenAt));

  onMount(async () => {
    try {
      all = await backend.listSnapshots(null);
      root = all[0]?.rootPath ?? null;
    } catch (e) {
      all = [];
      toasts.error(e);
    }
  });

  // latest vs previous by default whenever the root changes
  $effect(() => {
    const list = snaps;
    toId = list[0]?.id ?? null;
    fromId = list[1]?.id ?? null;
  });

  $effect(() => {
    const f = fromId;
    const to = toId;
    if (f == null || to == null || f === to) {
      cmp = null;
      return;
    }
    cmpLoading = true;
    backend
      .compareSnapshots(f, to)
      .then((c) => {
        cmp = c;
        chartPath = c.items[0]?.path ?? c.to.rootPath;
      })
      .catch((e: unknown) => toasts.error(e))
      .finally(() => (cmpLoading = false));
  });

  const label = (s: SnapshotInfo) => `${formatDate(s.takenAt, i18n.lang, true)} · ${formatSize(s.totalBytes, i18n.lang)}`;
</script>

<div class="page">
  <header>
    <h2>{t("growth.title")}</h2>
    <p class="muted">{t("growth.subtitle")}</p>
  </header>

  {#if all == null}
    <Skeleton height={80} radius="var(--r-lg)" />
    <Skeleton height={260} radius="var(--r-lg)" />
  {:else if all.length === 0}
    <EmptyState icon="growth" title={t("growth.emptyTitle")} text={t("growth.emptyText")} />
  {:else}
    <div class="card pickers">
      <label>
        <span class="eyebrow">{t("growth.location")}</span>
        <select class="field" bind:value={root}>
          {#each roots as r (r)}<option value={r}>{r}</option>{/each}
        </select>
      </label>
      <label>
        <span class="eyebrow">{t("growth.from")}</span>
        <select class="field" bind:value={fromId}>
          {#each snaps as s (s.id)}<option value={s.id}>{label(s)}</option>{/each}
        </select>
      </label>
      <label>
        <span class="eyebrow">{t("growth.to")}</span>
        <select class="field" bind:value={toId}>
          {#each snaps as s (s.id)}<option value={s.id}>{label(s)}</option>{/each}
        </select>
      </label>
    </div>

    {#if snaps.length < 2}
      <EmptyState compact icon="growth" title={t("growth.notEnough")} />
    {:else if cmpLoading && !cmp}
      <Skeleton height={200} radius="var(--r-lg)" />
    {:else if cmp}
      <p class="sentence card">{growthSentence(cmp, i18n.lang)}</p>

      <section class="card table">
        <h3 class="section-title">{t("growth.changes")}</h3>
        {#if cmp.items.length === 0}
          <EmptyState compact icon="check-circle" title={t("growth.noChanges")} />
        {:else}
          <table>
            <thead>
              <tr>
                <th>{t("growth.colItem")}</th>
                <th class="n">{t("growth.colBefore")}</th>
                <th class="n">{t("growth.colAfter")}</th>
                <th class="n">{t("growth.colChange")}</th>
              </tr>
            </thead>
            <tbody>
              {#each cmp.items as g (g.path)}
                <tr class:sel={chartPath === g.path}>
                  <td>
                    <button type="button" class="linkish" onclick={() => (chartPath = g.path)} aria-label={t("growth.showChart", { name: lastSegment(g.path) })}>
                      <span class="nm">{g.explanation ? bi(g.explanation.title) : lastSegment(g.path)}</span>
                      <PathText path={g.path} max={60} />
                    </button>
                  </td>
                  <td class="n num">{formatSize(g.before, i18n.lang)}</td>
                  <td class="n num">{formatSize(g.after, i18n.lang)}</td>
                  <td class="n num delta" class:up={g.delta > 0} class:down={g.delta < 0}>{formatSizeDelta(g.delta, i18n.lang)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </section>

      {#if chartPath}
        <section class="card chart">
          <h3 class="section-title">{t("growth.overTime")}</h3>
          <p class="path faint">{chartPath}</p>
          <GrowthChart path={chartPath} />
        </section>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .page {
    max-width: 1000px;
    padding: var(--sp-6) var(--sp-6) var(--sp-12);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .pickers {
    display: grid;
    grid-template-columns: 1fr 1.4fr 1.4fr;
    gap: var(--sp-4);
    padding: var(--sp-4) var(--sp-5);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .sentence {
    padding: var(--sp-5);
    font-size: var(--fs-lg);
    line-height: 1.8;
  }

  .table,
  .chart {
    padding: var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-sm);
  }

  th {
    text-align: start;
    font-size: var(--fs-xs);
    font-weight: var(--fw-medium);
    color: var(--text-3);
    padding: 0 var(--sp-2) var(--sp-2);
    border-bottom: 1px solid var(--border);
  }

  td {
    padding: var(--sp-2);
    border-bottom: 1px solid var(--border);
  }

  .n {
    text-align: end;
    white-space: nowrap;
  }

  tr.sel td {
    background: var(--accent-soft);
  }

  .linkish {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    border: 0;
    background: none;
    padding: 0;
    text-align: start;
    cursor: pointer;
  }

  .nm {
    font-weight: var(--fw-medium);
  }

  .delta {
    font-weight: var(--fw-bold);
  }

  .up {
    color: var(--careful);
  }

  .down {
    color: var(--safe);
  }

  .chart .path {
    font-size: var(--fs-xs);
  }
</style>
