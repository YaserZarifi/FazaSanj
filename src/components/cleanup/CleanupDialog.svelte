<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { ActionStatus, CleanupAction, CleanupPlan } from "../../lib/api/types";
  import { formatInt, formatSize } from "../../lib/format";
  import { bi, errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { methodKey } from "../../lib/labels";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Modal from "../common/Modal.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import Spinner from "../common/Spinner.svelte";

  const c = $derived(app.cleanup!);
  let plan = $state<CleanupPlan | null>(null);
  let error = $state<string | null>(null);
  let permanentForSafe = $state(false);
  let createRestorePoint = $state(true);
  let starting = $state(false);
  let expanded = $state<Record<number, boolean>>({});

  $effect(() => {
    const targets = app.cleanup?.targets;
    if (!targets) return;
    api
      .buildCleanupPlan(targets)
      .then((p) => (plan = p))
      .catch((e) => (error = errorText(e)));
  });

  const runnable = $derived(plan ? plan.actions.filter((a) => !a.blocked) : []);
  const blocked = $derived(plan ? plan.actions.filter((a) => a.blocked) : []);
  const sameDrive = $derived(plan?.warnings.some((w) => w.code === "recycle_same_drive") ?? false);
  const hasSafe = $derived(runnable.some((a) => a.safety === "safe" && (a.method === "recycle" || a.method === "delete_contents")));
  const running = $derived(c.jobId !== null && !c.report);
  const report = $derived(c.report);

  async function run(dryRun: boolean) {
    if (!plan) return;
    starting = true;
    error = null;
    try {
      const jobId = await api.runCleanup(plan.planId, {
        dryRun,
        permanentForSafe: permanentForSafe && hasSafe,
        createRestorePoint: createRestorePoint && plan.hasSystemActions,
      });
      if (app.cleanup) {
        app.cleanup.jobId = jobId;
        app.cleanup.report = null;
        app.cleanup.progress = null;
      }
    } catch (e) {
      error = errorText(e);
    } finally {
      starting = false;
    }
  }

  function again() {
    if (app.cleanup) {
      app.cleanup.jobId = null;
      app.cleanup.report = null;
    }
  }

  const statusIcon: Record<ActionStatus, string> = {
    done: "check",
    dry_run: "eye",
    partial: "alert",
    skipped_in_use: "lock",
    blocked: "shield",
    failed: "x",
    needs_manual: "tool",
    opened_setting: "external",
  };

  const actionLabel = (a: CleanupAction) => bi(a.title) || a.path;
</script>

<Modal title={t("cleanup.title")} onclose={() => app.closeCleanup()} width={760} closable={!running}>
  {#if error}
    <div class="notice error">{error}</div>
  {/if}

  {#if !plan && !error}
    <div class="center"><Spinner size={24} /></div>
  {:else if plan && report}
    <!-- result -->
    {#if report.dryRun}
      <div class="notice info"><Icon name="eye" />{t("cleanup.dryDone", { size: formatSize(plan.totalBytes, lang()) })}</div>
    {:else}
      <div class="result-head">
        <Icon name="check" size={28} />
        <div>
          <h3>{t("cleanup.freed", { size: formatSize(report.bytesFreed, lang()) })}</h3>
          {#if report.freeAfter > 0}
            <p class="muted">
              {t("cleanup.freeNow", { before: formatSize(report.freeBefore, lang()), after: formatSize(report.freeAfter, lang()) })}
            </p>
          {/if}
        </div>
      </div>
      {#if report.restorePointCreated === false}
        <div class="notice warn"><Icon name="alert" />{t("cleanup.restoreFailed")}</div>
      {:else if report.restorePointCreated}
        <div class="notice ok"><Icon name="shield-check" />{t("cleanup.restoreMade")}</div>
      {/if}
    {/if}
    <ul class="results">
      {#each report.results as r (r.index)}
        {@const a = plan.actions.find((x) => x.index === r.index)}
        <li class="status-{r.status}">
          <Icon name={statusIcon[r.status]} size={16} />
          <div class="grow">
            <div class="row">
              <strong>{a ? actionLabel(a) : r.path}</strong>
              <span class="spacer"></span>
              <span class="faint">{t(`status.${r.status}`)}</span>
            </div>
            <bdi class="path faint">{r.path}</bdi>
            {#if r.bytesFreed > 0}<span class="faint">{t("cleanup.itemFreed", { size: formatSize(r.bytesFreed, lang()) })}</span>{/if}
            {#if r.filesSkipped > 0}<span class="faint">{t("cleanup.skipped", { count: formatInt(r.filesSkipped, lang()) })}</span>{/if}
            {#if r.error}<span class="err">{errorText(r.error)}</span>{/if}
            {#if r.status === "needs_manual" && a?.instructions}<p class="pre">{bi(a.instructions)}</p>{/if}
            {#if report.dryRun && r.wouldRemove.length}
              <details>
                <summary>{t("cleanup.wouldRemove", { count: formatInt(r.wouldRemove.length, lang()) })}</summary>
                <ul class="would">
                  {#each r.wouldRemove.slice(0, 50) as p (p)}<li><bdi class="path">{p}</bdi></li>{/each}
                </ul>
              </details>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
    {#if !report.dryRun && sameDrive && !permanentForSafe}
      <div class="notice info">
        <Icon name="info" />
        <span>{t("cleanup.recycleReminder")}</span>
        <button class="btn small" onclick={() => api.openRecycleBin()}>{t("cleanup.openBin")}</button>
      </div>
    {/if}
  {:else if plan && running}
    <!-- progress -->
    <div class="running">
      <Spinner size={32} />
      <h3>{t("cleanup.running")}</h3>
      {#if c.progress}
        <progress max={c.progress.total} value={c.progress.index}></progress>
        <p class="faint">{formatInt(c.progress.index + 1, lang())} / {formatInt(c.progress.total, lang())} · {formatSize(c.progress.bytesFreed, lang())}</p>
        <bdi class="path faint current">{c.progress.currentPath}</bdi>
      {/if}
    </div>
  {:else if plan}
    <!-- review -->
    <p>{t("cleanup.intro", { count: runnable.length, size: formatSize(runnable.reduce((s, a) => s + a.bytes, 0), lang()) })}</p>

    <ul class="actions">
      {#each runnable as a (a.index)}
        <li>
          <div class="row">
            <strong class="grow">{actionLabel(a)}</strong>
            <SafetyBadge level={a.safety} small />
            <strong class="size">{formatSize(a.bytes, lang())}</strong>
          </div>
          <bdi class="path faint">{a.path}</bdi>
          <div class="row faint">
            <span>{t(methodKey(a.method))}</span>
            {#if a.permanent}<span>· {t("cleanup.permanent")}</span>{/if}
            {#if a.needsAdmin}<span>· {t("detail.needsAdmin")}</span>{/if}
            {#if a.command}<span>· <bdi class="mono">{a.command}</bdi></span>{/if}
            <span class="spacer"></span>
            <button class="btn ghost small" onclick={() => (expanded[a.index] = !expanded[a.index])}>
              {t("reason.whatHappens")}
            </button>
          </div>
          {#if expanded[a.index]}
            <p class="consequence">{bi(a.consequence)}</p>
            {#if a.instructions}<p class="pre">{bi(a.instructions)}</p>{/if}
          {/if}
        </li>
      {/each}
    </ul>

    {#if blocked.length}
      <div class="notice warn">
        <Icon name="shield" />
        <div>
          <strong>{t("cleanup.blockedTitle", { count: blocked.length })}</strong>
          <ul class="would">
            {#each blocked as a (a.index)}<li><bdi class="path">{a.path}</bdi></li>{/each}
          </ul>
        </div>
      </div>
    {/if}

    {#if sameDrive}
      <div class="notice info">
        <Icon name="info" />
        <div class="col">
          <span>{t("warning.recycle_same_drive")}</span>
          {#if hasSafe}
            <label class="check"><input type="checkbox" bind:checked={permanentForSafe} /> {t("cleanup.permanentSafe")}</label>
          {/if}
        </div>
      </div>
    {/if}
    {#if plan.hasSystemActions}
      <label class="check card opt">
        <input type="checkbox" bind:checked={createRestorePoint} />
        <span>
          <strong>{t("cleanup.restorePoint")}</strong>
          <span class="faint">{t("cleanup.restorePointHint")}</span>
        </span>
      </label>
    {/if}
    {#if plan.needsAdmin}
      <div class="notice"><Icon name="shield" />{t("warning.needs_admin")}</div>
    {/if}
    {#each plan.warnings.filter((w) => w.code === "outside_sandbox") as w, i (i)}
      <div class="notice warn"><Icon name="alert" />{t("warning.outside_sandbox")} <bdi class="path">{w.path}</bdi></div>
    {/each}
  {/if}

  {#snippet footer()}
    {#if report}
      {#if report.dryRun}
        <button class="btn" onclick={again}>{t("common.back")}</button>
      {/if}
      <button class="btn primary" onclick={() => app.closeCleanup()}>{t("common.close")}</button>
    {:else if !running}
      <button class="btn" onclick={() => app.closeCleanup()}>{t("common.cancel")}</button>
      <button class="btn" onclick={() => run(true)} disabled={!plan || runnable.length === 0 || starting}>
        <Icon name="eye" size={15} />{t("cleanup.dryRun")}
      </button>
      <button class="btn primary" onclick={() => run(false)} disabled={!plan || runnable.length === 0 || starting}>
        <Icon name="trash" size={15} />{t("cleanup.run")}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .center {
    display: grid;
    place-items: center;
    min-height: 120px;
  }
  .actions,
  .results {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .actions li,
  .results li {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .results li {
    flex-direction: row;
    gap: 10px;
    align-items: flex-start;
  }
  .actions li:last-child,
  .results li:last-child {
    border-bottom: none;
  }
  .grow {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .size {
    white-space: nowrap;
  }
  .consequence {
    padding: 8px 10px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
  }
  .pre {
    white-space: pre-line;
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    cursor: pointer;
  }
  .opt {
    padding: 12px;
  }
  .opt span {
    display: flex;
    flex-direction: column;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .would {
    margin: 4px 0 0;
    padding-inline-start: 18px;
    max-height: 180px;
    overflow: auto;
  }
  .running {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 20px 0;
    text-align: center;
  }
  progress {
    width: 100%;
    accent-color: var(--accent);
  }
  .current {
    max-width: 100%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .result-head {
    display: flex;
    gap: 12px;
    align-items: center;
    color: var(--safe);
  }
  .result-head h3 {
    color: var(--text);
    font-size: 18px;
  }
  .status-done,
  .status-opened_setting {
    color: var(--safe);
  }
  .status-failed,
  .status-blocked {
    color: var(--danger);
  }
  .status-partial,
  .status-skipped_in_use,
  .status-needs_manual {
    color: var(--careful);
  }
  .results li > .grow {
    color: var(--text);
  }
  .err {
    color: var(--danger);
    font-size: 13px;
  }
</style>
