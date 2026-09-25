<script lang="ts">
  import type { HeuristicKind } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { heuristics } from "../../lib/stores/heuristics.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import Segmented from "../common/Segmented.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import UsageBar from "../common/UsageBar.svelte";
  import FindingCard from "./FindingCard.svelte";

  const KINDS: HeuristicKind[] = ["duplicates", "old_project", "orphan", "stale"];
  const st = $derived(heuristics.state(ui.heuristicKind, scan.scanId));
  const total = $derived(st?.findings?.reduce((s, f) => s + f.bytes, 0) ?? 0);

  $effect(() => {
    const kind = ui.heuristicKind;
    if (scan.scanId != null) void heuristics.run(scan.scanId, kind);
  });
</script>

<div class="page">
  <button type="button" class="btn btn-sm btn-ghost back" onclick={() => ui.go("home")}>
    <Icon name="arrow-left" size={16} flip />{t("common.back")}
  </button>

  {#if scan.phase !== "done" || scan.scanId == null}
    <EmptyState icon="search" title={t("heur.needScan")} />
  {:else}
    <div class="head">
      <div>
        <h2>{t(`heur.kinds.${ui.heuristicKind}.title`)}</h2>
        <p class="muted">{t(`heur.kinds.${ui.heuristicKind}.long`)}</p>
      </div>
      <Segmented
        label={t("heur.entryTitle")}
        value={ui.heuristicKind}
        options={KINDS.map((k) => ({ value: k, label: t(`heur.kinds.${k}.tab`) }))}
        onchange={(k) => (ui.heuristicKind = k)}
      />
    </div>

    <p class="note"><Icon name="info" size={16} /> {t("heur.neverSafe")}</p>

    {#if !st || st.running}
      <div class="loading" role="status">
        <p class="muted">{t("heur.running")}</p>
        {#if st && st.total > 0}
          <UsageBar value={st.done / st.total} label={t("heur.running")} />
        {/if}
        {#each [0, 1, 2] as i (i)}
          <Skeleton height={96} radius="var(--r-lg)" />
        {/each}
      </div>
    {:else if st.findings && st.findings.length === 0}
      <EmptyState icon="check-circle" title={t("heur.noneTitle")} text={t(`heur.kinds.${ui.heuristicKind}.none`)} />
    {:else if st.findings}
      <p class="sum num">{t("heur.found", { count: st.findings.length, size: formatSize(total, i18n.lang) })}</p>
      <div class="list">
        {#each st.findings as f (f.path)}
          <FindingCard finding={f} />
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .page {
    padding: var(--sp-4) var(--sp-6) var(--sp-12);
    max-width: 980px;
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .back {
    align-self: flex-start;
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    flex-wrap: wrap;
    gap: var(--sp-4);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .note {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--probably-safe-soft);
    font-size: var(--fs-sm);
  }

  .note :global(svg) {
    color: var(--probably-safe);
  }

  .loading,
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .sum {
    font-weight: var(--fw-medium);
  }
</style>
