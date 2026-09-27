<script lang="ts">
  // Compare snapshots: "since last scan C: grew 12 GB, mostly here".
  import * as api from "../../lib/api/commands";
  import type { SnapshotComparison, SnapshotInfo } from "../../lib/api/types";
  import { formatDate, formatDelta, formatSize, ltr } from "../../lib/format";
  import { bi, errorText, lang, t } from "../../lib/i18n/index.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";
  import GrowthChart from "./GrowthChart.svelte";

  let snapshots = $state<SnapshotInfo[]>([]);
  let root = $state<string>("");
  let fromId = $state<number | null>(null);
  let toId = $state<number | null>(null);
  let cmp = $state<SnapshotComparison | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let focus = $state<string | null>(null);

  $effect(() => {
    api
      .listSnapshots(null)
      .then((s) => {
        snapshots = s;
        if (s.length) root = s[0].rootPath;
      })
      .catch((e) => (error = errorText(e)))
      .finally(() => (loading = false));
  });

  const roots = $derived([...new Set(snapshots.map((s) => s.rootPath))]);
  const ofRoot = $derived(snapshots.filter((s) => s.rootPath === root));

  $effect(() => {
    const list = ofRoot;
    toId = list[0]?.id ?? null;
    fromId = list[1]?.id ?? null;
    focus = null;
  });

  $effect(() => {
    const [f, to] = [fromId, toId];
    cmp = null;
    if (f === null || to === null || f === to) return;
    api.compareSnapshots(f, to).then((c) => (cmp = c)).catch((e) => (error = errorText(e)));
  });

  const grown = $derived(cmp ? cmp.items.filter((i) => i.delta > 0).slice(0, 25) : []);
  const shrunk = $derived(cmp ? cmp.items.filter((i) => i.delta < 0).slice(0, 10) : []);
</script>

<section class="page">
  <p class="muted">{t("growth.intro")}</p>
  {#if error}<div class="notice error">{error}</div>{/if}
  {#if loading}
    <Spinner />
  {:else if snapshots.length === 0}
    <div class="empty"><Icon name="growth" size={40} /><p class="muted">{t("growth.empty")}</p></div>
  {:else}
    <div class="row wrap controls">
      <label>
        <span class="faint">{t("growth.root")}</span>
        <select class="select" bind:value={root}>
          {#each roots as r (r)}<option value={r}>{ltr(r)}</option>{/each}
        </select>
      </label>
      <label>
        <span class="faint">{t("growth.from")}</span>
        <select class="select" bind:value={fromId}>
          {#each ofRoot as s (s.id)}<option value={s.id}>{formatDate(s.takenAt, lang(), true)}</option>{/each}
        </select>
      </label>
      <label>
        <span class="faint">{t("growth.to")}</span>
        <select class="select" bind:value={toId}>
          {#each ofRoot as s (s.id)}<option value={s.id}>{formatDate(s.takenAt, lang(), true)}</option>{/each}
        </select>
      </label>
    </div>

    <GrowthChart path={focus ?? root} />

    {#if ofRoot.length < 2}
      <div class="notice info">{t("growth.needTwo")}</div>
    {:else if cmp}
      <div class="card summary">
        <h2>
          {cmp.totalDelta >= 0
            ? t("growth.grewBy", { root: ltr(root), delta: formatDelta(cmp.totalDelta, lang()) })
            : t("growth.shrankBy", { root: ltr(root), delta: formatDelta(cmp.totalDelta, lang()) })}
        </h2>
        <p class="muted">
          {t("growth.between", { from: formatDate(cmp.from.takenAt, lang()), to: formatDate(cmp.to.takenAt, lang()) })}
        </p>
      </div>
      {#if grown.length}
        <h3>{t("growth.grown")}</h3>
        <div class="card">
          <table class="list">
            <tbody>
              {#each grown as g (g.path)}
                <tr class="clickable" class:selected={focus === g.path} onclick={() => (focus = g.path)}>
                  <td>
                    <bdi class="path">{g.path}</bdi>
                    {#if g.explanation}<div class="faint">{bi(g.explanation.title)}</div>{/if}
                  </td>
                  <td class="num faint">{formatSize(g.before, lang())} → {formatSize(g.after, lang())}</td>
                  <td class="num up">{formatDelta(g.delta, lang())}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
      {#if shrunk.length}
        <h3>{t("growth.shrunk")}</h3>
        <div class="card">
          <table class="list">
            <tbody>
              {#each shrunk as g (g.path)}
                <tr class="clickable" onclick={() => (focus = g.path)}>
                  <td><bdi class="path">{g.path}</bdi></td>
                  <td class="num faint">{formatSize(g.before, lang())} → {formatSize(g.after, lang())}</td>
                  <td class="num down">{formatDelta(g.delta, lang())}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .page {
    max-width: 960px;
    margin: 0 auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 48px 0;
    color: var(--text-3);
  }
  .controls label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .wrap {
    flex-wrap: wrap;
    gap: 12px;
  }
  .summary {
    padding: 16px 18px;
  }
  .up {
    color: var(--danger);
    font-weight: 700;
  }
  .down {
    color: var(--safe);
    font-weight: 700;
  }
</style>
