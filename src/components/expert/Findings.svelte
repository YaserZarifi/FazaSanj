<script lang="ts">
  import type { HeuristicFinding, HeuristicKind, NodeId, ScanId } from "../../lib/api/types";
  import { findingTargets } from "../../lib/findings";
  import { formatDate, formatInt, formatPercent, formatSize } from "../../lib/format";
  import { bi, errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { scanId, onselect }: { scanId: ScanId; onselect: (id: NodeId) => void } = $props();

  const ALL: HeuristicKind[] = ["duplicates", "old_project", "orphan", "stale"];
  let kinds = $state<Record<HeuristicKind, boolean>>({ duplicates: true, old_project: true, orphan: true, stale: true });
  let chosen = $state<Record<string, boolean>>({});
  let error = $state<string | null>(null);

  const job = $derived(app.heuristics[scanId] ?? null);
  const findings = $derived(job?.findings ?? []);
  const chosenList = $derived(findings.filter((f, i) => chosen[keyOf(f, i)]));
  const chosenBytes = $derived(chosenList.reduce((a, f) => a + f.bytes, 0));

  function keyOf(f: HeuristicFinding, i: number) {
    return `${f.kind}:${f.path}:${i}`;
  }

  async function run() {
    error = null;
    chosen = {};
    try {
      await app.runHeuristics(scanId, ALL.filter((k) => kinds[k]));
    } catch (e) {
      error = errorText(e);
    }
  }

  function review() {
    app.openCleanup(chosenList.flatMap(findingTargets));
  }

  const grouped = $derived(ALL.map((k) => ({ kind: k, items: findings.map((f, i) => ({ f, i })).filter((x) => x.f.kind === k) })));
</script>

<div class="findings">
  <p class="muted">{t("findings.explain")}</p>
  <div class="row wrap">
    {#each ALL as k (k)}
      <label class="check"><input type="checkbox" bind:checked={kinds[k]} /> {t(`findings.kind.${k}`)}</label>
    {/each}
    <span class="spacer"></span>
    <button class="btn primary" onclick={run} disabled={job?.running || !ALL.some((k) => kinds[k])}>
      {#if job?.running}<Spinner size={14} />{:else}<Icon name="search" size={15} />{/if}
      {job?.running ? t("findings.running") : t("findings.run")}
    </button>
  </div>
  {#if job?.running && job.progress}
    <p class="faint">
      {t(`findings.kind.${job.progress.kind}`)}
      {#if job.progress.total > 1}· {formatInt(job.progress.done, lang())} / {formatInt(job.progress.total, lang())}{/if}
    </p>
  {/if}
  {#if error}<div class="notice error">{error}</div>{/if}

  {#if job && !job.running}
    {#if findings.length === 0}
      <div class="notice ok"><Icon name="check" />{t("findings.none")}</div>
    {:else}
      <div class="row sticky">
        <span>{t("findings.count", { count: findings.length })}</span>
        <span class="spacer"></span>
        <button class="btn" disabled={chosenList.length === 0} onclick={review}>
          {t("story.reviewChosen", { count: chosenList.length, size: formatSize(chosenBytes, lang()) })}
        </button>
      </div>
      {#each grouped as g (g.kind)}
        {#if g.items.length}
          <h3>{t(`findings.kind.${g.kind}`)}</h3>
          {#if g.kind === "stale" && g.items[0].f.details.kind === "stale" && !g.items[0].f.details.accessTimeReliable}
            <div class="notice warn"><Icon name="info" />{t("findings.atimeOff")}</div>
          {/if}
          <div class="card">
            {#each g.items as { f, i } (keyOf(f, i))}
              <div class="finding">
                <input
                  type="checkbox"
                  aria-label={f.path}
                  checked={!!chosen[keyOf(f, i)]}
                  onchange={(e) => (chosen[keyOf(f, i)] = e.currentTarget.checked)}
                />
                <div class="body">
                  <div class="row">
                    {#if f.nodeId !== null}
                      <button class="link" onclick={() => onselect(f.nodeId!)}><bdi class="path">{f.path}</bdi></button>
                    {:else}
                      <bdi class="path">{f.path}</bdi>
                    {/if}
                  </div>
                  <p class="muted">{bi(f.reason)}</p>
                  {#if f.details.kind === "duplicates"}
                    <ul class="copies">
                      {#each f.details.files as file, fi (file.path)}
                        <li class:keep={fi === f.details.keepIndex}>
                          <bdi class="path">{file.path}</bdi>
                          {#if fi === f.details.keepIndex}<span class="keep-tag">{t("findings.keep")}</span>{/if}
                          <span class="faint">{formatDate(file.modified, lang())}</span>
                        </li>
                      {/each}
                    </ul>
                  {:else if f.details.kind === "old_project"}
                    <p class="faint">
                      {t("findings.project", { kind: f.details.projectKind, date: formatDate(f.details.lastTouched, lang()) })}
                    </p>
                  {:else if f.details.kind === "orphan"}
                    <p class="faint">{t("findings.orphanOf", { app: f.details.appName })}</p>
                  {/if}
                  <div class="row faint">
                    <SafetyBadge level={f.safety} small />
                    <span>{t("findings.confidence", { value: formatPercent(f.confidence, lang()) })}</span>
                  </div>
                </div>
                <strong class="size">{formatSize(f.bytes, lang())}</strong>
              </div>
            {/each}
          </div>
        {/if}
      {/each}
    {/if}
  {/if}
</div>

<style>
  .findings {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .check {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    margin-inline-end: 10px;
  }
  .sticky {
    position: sticky;
    top: -16px;
    background: var(--surface);
    padding: 8px 0;
    z-index: 2;
  }
  .finding {
    display: flex;
    gap: 12px;
    padding: 12px 14px;
    border-bottom: 1px solid var(--border);
    align-items: flex-start;
  }
  .finding:last-child {
    border-bottom: none;
  }
  .finding input {
    margin-top: 5px;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .size {
    white-space: nowrap;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
    color: var(--accent);
    text-align: start;
  }
  .copies {
    margin: 0;
    padding-inline-start: 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .copies li.keep {
    font-weight: 500;
  }
  .keep-tag {
    font-size: 11.5px;
    color: var(--safe);
    border: 1px solid var(--safe);
    border-radius: 99px;
    padding: 0 6px;
    margin-inline: 6px;
  }
</style>
