<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { ActionStatus } from "../../lib/api/types";
  import { formatDuration, formatNumber, formatSize } from "../../lib/format";
  import { errorText, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import type { IconName } from "../common/icons";
  import StatusBadge from "../history/StatusBadge.svelte";

  const r = $derived(cleanup.report);
  const gained = $derived(r ? r.freeAfter - r.freeBefore : 0);
  const inBin = $derived(
    r ? r.results.filter((x) => (x.method === "recycle" || x.method === "delete_contents") && (x.status === "done" || x.status === "partial")).reduce((s, x) => s + x.bytesFreed, 0) - Math.max(0, gained) : 0,
  );
  const problems = $derived(r?.results.filter((x) => x.status === "failed" || x.status === "blocked" || x.status === "partial" || x.status === "skipped_in_use") ?? []);

  const ICON: Partial<Record<ActionStatus, IconName>> = { done: "check-circle", failed: "alert", blocked: "lock" };
</script>

{#if r}
  <div class="page">
    <header class="hero card">
      <span class="ok"><Icon name={ICON.done ?? "check-circle"} size={30} /></span>
      <div>
        <h2>{t("result.title")}</h2>
        <p class="muted">{t("result.took", { time: formatDuration(r.finishedAt - r.startedAt, i18n.lang) })}</p>
      </div>
    </header>

    <div class="stats">
      <div class="card stat">
        <p class="eyebrow">{t("result.freeBefore")}</p>
        <p class="v num">{formatSize(r.freeBefore, i18n.lang)}</p>
      </div>
      <div class="card stat">
        <p class="eyebrow">{t("result.freeAfter")}</p>
        <p class="v num">{formatSize(r.freeAfter, i18n.lang)}</p>
      </div>
      <div class="card stat good">
        <p class="eyebrow">{t("result.gained")}</p>
        <p class="v num">{formatSize(Math.max(0, gained), i18n.lang)}</p>
      </div>
    </div>

    {#if inBin > 0}
      <div class="bin" role="note">
        <Icon name="recycle" size={20} />
        <p>{t("result.inBin", { size: formatSize(inBin, i18n.lang) })}</p>
        <button type="button" class="btn btn-sm" onclick={() => backend.openRecycleBin().catch((e: unknown) => toasts.error(e))}>
          {t("history.openBin")}
        </button>
      </div>
    {/if}

    {#if r.restorePointCreated === true}
      <p class="note"><Icon name="shield-check" size={16} /> {t("result.restorePoint")}</p>
    {:else if r.restorePointCreated === false}
      <p class="note warnnote"><Icon name="alert" size={16} /> {t("result.restorePointFailed")}</p>
    {/if}

    <section class="card list">
      <h3 class="section-title">{t("result.details")}</h3>
      <ul>
        {#each r.results as x (x.index)}
          <li>
            <PathText path={x.path} max={70} />
            <StatusBadge status={x.status} />
            <span class="num">{x.bytesFreed ? formatSize(x.bytesFreed, i18n.lang) : ""}</span>
          </li>
        {/each}
      </ul>
    </section>

    {#if problems.length}
      <section class="card list problems">
        <h3 class="section-title">{t("result.problems")}</h3>
        <ul>
          {#each problems as x (x.index)}
            <li class="prob">
              <PathText path={x.path} max={70} />
              <p>
                {#if x.error}{errorText(x.error)}{/if}
                {#if x.filesSkipped > 0}{t("result.skipped", { count: x.filesSkipped, n: formatNumber(x.filesSkipped, i18n.lang) })}{/if}
              </p>
              {#if x.skippedPaths.length}
                <ul class="skipped">
                  {#each x.skippedPaths.slice(0, 5) as sp (sp)}<li class="path">{sp}</li>{/each}
                </ul>
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    <div class="btns">
      <button type="button" class="btn" onclick={() => ui.go("history")}><Icon name="history" size={16} />{t("nav.history")}</button>
      <button type="button" class="btn btn-primary" onclick={() => cleanup.close()}>{t("result.done")}</button>
    </div>
  </div>
{/if}

<style>
  .page {
    max-width: 920px;
    padding: var(--sp-6) var(--sp-6) var(--sp-12);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .hero {
    display: flex;
    align-items: center;
    gap: var(--sp-4);
    padding: var(--sp-6);
  }

  .ok {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--safe-soft);
    color: var(--safe);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--sp-3);
  }

  .stat {
    padding: var(--sp-4) var(--sp-5);
  }

  .v {
    font-size: var(--fs-2xl);
    font-weight: var(--fw-bold);
  }

  .good .v {
    color: var(--safe);
  }

  .bin {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border-radius: var(--r-lg);
    background: var(--probably-safe-soft);
  }

  .bin > :global(svg) {
    color: var(--probably-safe);
  }

  .bin p {
    flex: 1;
    font-size: var(--fs-sm);
  }

  .note {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-sm);
    color: var(--safe);
  }

  .warnnote {
    color: var(--careful);
  }

  .list {
    padding: var(--sp-5);
  }

  .list > ul {
    list-style: none;
    margin: var(--sp-3) 0 0;
    padding: 0;
  }

  .list > ul > li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto 90px;
    align-items: center;
    gap: var(--sp-3);
    padding: 6px 0;
    border-top: 1px solid var(--border);
    font-size: var(--fs-sm);
  }

  .list > ul > li > .num {
    text-align: end;
  }

  .list > ul > li.prob {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .problems {
    border-color: color-mix(in srgb, var(--careful) 40%, var(--border));
  }

  .skipped {
    margin: 0;
    padding-inline-start: var(--sp-5);
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .btns {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
  }
</style>
