<script lang="ts">
  import type { HeuristicFinding } from "../../lib/api/types";
  import { formatDate, formatPercent, formatRelative, formatSize } from "../../lib/format";
  import { cappedSafety, findingTarget } from "../../lib/heuristicTarget";
  import { bi, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";

  let { finding }: { finding: HeuristicFinding } = $props();
  const d = $derived(finding.details);
  const confLabel = $derived(
    finding.confidence >= 0.85 ? t("heur.confHigh") : finding.confidence >= 0.65 ? t("heur.confMedium") : t("heur.confLow"),
  );
</script>

<article class="card f">
  <div class="head">
    <div class="main">
      <PathText path={finding.path} max={80} />
      <p class="reason">{bi(finding.reason)}</p>
    </div>
    <span class="size num">{formatSize(finding.bytes, i18n.lang)}</span>
  </div>

  <div class="meta">
    <SafetyBadge level={cappedSafety(finding.safety)} />
    <span class="conf" title={t("heur.confidenceHint")}>
      <span class="meter" aria-hidden="true"><span style:width="{finding.confidence * 100}%"></span></span>
      {t("heur.confidence", { level: confLabel, pct: formatPercent(finding.confidence, i18n.lang) })}
    </span>
  </div>

  {#if d.kind === "orphan"}
    <p class="detail">{t("heur.orphanApp", { app: d.appName })}</p>
    <div class="actions">
      {@render addBtn(finding.path, finding.bytes)}
    </div>
  {:else if d.kind === "stale"}
    <p class="detail">
      {t("heur.lastModified", { when: formatRelative(d.lastModified, i18n.lang) })}
      {#if d.accessTimeReliable && d.lastAccessed}
        · {t("heur.lastOpened", { when: formatRelative(d.lastAccessed, i18n.lang) })}
      {/if}
    </p>
    {#if !d.accessTimeReliable}
      <p class="note"><Icon name="info" size={14} /> {t("heur.accessOff")}</p>
    {/if}
    <div class="actions">
      {@render addBtn(finding.path, finding.bytes)}
    </div>
  {:else if d.kind === "duplicates"}
    <p class="detail">{t("heur.dupCopies", { count: d.files.length, size: formatSize(d.fileSize, i18n.lang) })}</p>
    <ul class="files">
      {#each d.files as file, i (file.path)}
        <li>
          {#if i === d.keepIndex}
            <span class="keep"><Icon name="check" size={13} />{t("heur.keep")}</span>
          {:else}
            <span class="copy">{t("heur.copy")}</span>
          {/if}
          <PathText path={file.path} max={70} />
          <span class="faint small">{formatDate(file.modified, i18n.lang)}</span>
          {#if i !== d.keepIndex}
            {@render addBtn(file.path, d.fileSize)}
          {/if}
        </li>
      {/each}
    </ul>
  {:else if d.kind === "old_project"}
    <p class="detail">
      {t("heur.projectKind", { kind: d.projectKind })} · {t("heur.lastTouched", { when: formatRelative(d.lastTouched, i18n.lang) })}
    </p>
    <p class="detail">{t("heur.rebuildable", { size: formatSize(d.rebuildableBytes, i18n.lang) })}</p>
    <ul class="files">
      {#each d.rebuildableDirs as dir (dir)}
        <li>
          <span class="copy">{t("heur.rebuildTag")}</span>
          <PathText path={dir} max={70} />
          {@render addBtn(dir, d.rebuildableBytes / d.rebuildableDirs.length)}
        </li>
      {/each}
    </ul>
  {/if}
</article>

{#snippet addBtn(path: string, bytes: number)}
  {@const inBasket = cleanup.has(path)}
  <button
    type="button"
    class="btn btn-sm add"
    class:added={inBasket}
    aria-pressed={inBasket}
    onclick={() => cleanup.toggle(findingTarget(finding, path, bytes))}
  >
    <Icon name={inBasket ? "check" : "plus"} size={14} />
    {inBasket ? t("reason.inCleanup") : t("reason.addToCleanup")}
  </button>
{/snippet}

<style>
  .f {
    padding: var(--sp-4) var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }

  .head {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-4);
  }

  .main {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .reason {
    font-weight: var(--fw-medium);
  }

  .size {
    font-size: var(--fs-lg);
    font-weight: var(--fw-bold);
    white-space: nowrap;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-3);
  }

  .conf {
    display: inline-flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .meter {
    width: 54px;
    height: 5px;
    border-radius: var(--r-full);
    background: var(--surface-3);
    overflow: hidden;
  }

  .meter span {
    display: block;
    height: 100%;
    background: var(--text-3);
  }

  .detail {
    font-size: var(--fs-sm);
    color: var(--text-2);
  }

  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .files {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .files li {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: 6px var(--sp-3);
    border-radius: var(--r-md);
    background: var(--surface-2);
  }

  .files li :global(.pt) {
    flex: 1;
    min-width: 0;
  }

  .keep,
  .copy {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    padding: 0 7px;
    border-radius: var(--r-full);
    white-space: nowrap;
  }

  .keep {
    background: var(--safe-soft);
    color: var(--safe);
  }

  .copy {
    background: var(--surface-3);
    color: var(--text-2);
  }

  .small {
    font-size: var(--fs-xs);
    white-space: nowrap;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .added {
    background: var(--safe-soft);
    border-color: transparent;
    color: var(--safe);
  }
</style>
