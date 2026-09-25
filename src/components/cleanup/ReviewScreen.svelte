<script lang="ts">
  import { formatSize } from "../../lib/format";
  import { errorText, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import Dialog from "../common/Dialog.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import Spinner from "../common/Spinner.svelte";
  import Toggle from "../common/Toggle.svelte";
  import ActionRow from "./ActionRow.svelte";

  let confirming = $state(false);

  const plan = $derived(cleanup.plan);
  const live = $derived(plan?.actions.filter((a) => !a.blocked) ?? []);
  const blockedCount = $derived((plan?.actions.length ?? 0) - live.length);
  const hasRecycle = $derived(live.some((a) => a.method === "recycle" || a.method === "delete_contents"));
  const hasSafeContents = $derived(live.some((a) => a.safety === "safe" && a.method === "delete_contents"));
  const otherWarnings = $derived(plan?.warnings.filter((w) => w.code !== "recycle_same_drive" && w.code !== "blocked") ?? []);

  function dryFor(index: number) {
    return cleanup.dryRunReport?.results.find((r) => r.index === index) ?? null;
  }
</script>

<div class="page">
  <button type="button" class="btn btn-sm btn-ghost back" onclick={() => cleanup.close()}>
    <Icon name="arrow-left" size={16} flip />{t("common.back")}
  </button>

  <header>
    <h2>{t("cleanup.reviewTitle")}</h2>
    <p class="muted">{t("cleanup.reviewText")}</p>
  </header>

  {#if cleanup.planLoading}
    <Skeleton height={120} radius="var(--r-lg)" />
    <Skeleton height={260} radius="var(--r-lg)" />
  {:else if cleanup.planError}
    <EmptyState icon="alert" title={t("cleanup.planFailed")} text={errorText(cleanup.planError)}>
      <button type="button" class="btn" onclick={() => cleanup.review()}>{t("common.retry")}</button>
    </EmptyState>
  {:else if plan}
    <div class="summary card">
      <div>
        <p class="eyebrow">{t("cleanup.total")}</p>
        <p class="big num">{formatSize(plan.totalBytes, i18n.lang)}</p>
      </div>
      <div>
        <p class="eyebrow">{t("cleanup.items")}</p>
        <p class="big num">{t("cleanup.itemCount", { count: live.length })}</p>
      </div>
      {#if plan.needsAdmin}
        <p class="adm"><Icon name="shield" size={16} /> {t("cleanup.planNeedsAdmin")}</p>
      {/if}
    </div>

    {#if hasRecycle}
      <div class="warn recycle" role="note">
        <Icon name="recycle" size={20} />
        <div>
          <p class="wt">{t("warnings.recycle_same_drive.title")}</p>
          <p>{t("warnings.recycle_same_drive.text")}</p>
          {#if hasSafeContents}
            <div class="opt">
              <Toggle
                checked={cleanup.options.permanentForSafe}
                label={t("cleanup.permanentForSafe")}
                description={t("cleanup.permanentForSafeHint")}
                onchange={(v) => (cleanup.options = { ...cleanup.options, permanentForSafe: v })}
              />
            </div>
          {/if}
        </div>
      </div>
    {/if}

    {#each otherWarnings as w, i (i)}
      <div class="warn" role="note">
        <Icon name={w.code === "needs_admin" ? "shield" : "alert"} size={20} />
        <div>
          <p class="wt">{t(`warnings.${w.code}.title`)}</p>
          <p>{t(`warnings.${w.code}.text`)}</p>
        </div>
      </div>
    {/each}

    {#if plan.hasSystemActions}
      <div class="card opt-card">
        <Toggle
          checked={cleanup.options.createRestorePoint}
          label={t("cleanup.restorePoint")}
          description={t("cleanup.restorePointHint")}
          onchange={(v) => (cleanup.options = { ...cleanup.options, createRestorePoint: v })}
        />
      </div>
    {/if}

    <ul class="card actions" aria-label={t("cleanup.actionsLabel")}>
      {#each plan.actions as a (a.index)}
        <ActionRow action={a} dry={dryFor(a.index)} />
      {/each}
    </ul>
    {#if blockedCount > 0}
      <p class="faint small"><Icon name="lock" size={14} /> {t("cleanup.blockedCount", { count: blockedCount })}</p>
    {/if}

    {#if cleanup.dryRunReport}
      <div class="dryres" role="status">
        <Icon name="eye" size={18} />
        <p>{t("dry.summary", { size: formatSize(cleanup.dryRunReport.results.reduce((s, r) => s + r.bytesFreed, 0), i18n.lang) })}</p>
      </div>
    {/if}

    <div class="bar">
      <p class="faint small">{t("cleanup.nothingYet")}</p>
      <div class="btns">
        <button type="button" class="btn" disabled={cleanup.runningDry || live.length === 0} onclick={() => cleanup.run(true)}>
          {#if cleanup.runningDry}<Spinner size={15} />{:else}<Icon name="eye" size={16} />{/if}
          {t("cleanup.dryRun")}
        </button>
        <button type="button" class="btn btn-primary" disabled={cleanup.runningDry || live.length === 0} onclick={() => (confirming = true)}>
          <Icon name="broom" size={17} />{t("cleanup.cleanNow")}
        </button>
      </div>
    </div>

    <Dialog open={confirming} title={t("cleanup.confirmTitle")} tone="danger" onclose={() => (confirming = false)}>
      <p>{t("cleanup.confirmText", { count: live.length, size: formatSize(plan.totalBytes, i18n.lang) })}</p>
      <ul class="confirm-list">
        {#each live as a (a.index)}
          <li><span class="ltr path">{a.path}</span><span class="num">{formatSize(a.bytes, i18n.lang)}</span></li>
        {/each}
      </ul>
      {#if hasRecycle && !cleanup.options.permanentForSafe}
        <p class="faint small">{t("cleanup.confirmRecycle")}</p>
      {/if}
      {#snippet footer()}
        <button type="button" class="btn" onclick={() => (confirming = false)}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn btn-danger"
          onclick={() => {
            confirming = false;
            void cleanup.run(false);
          }}
        >
          {t("cleanup.confirmGo", { size: formatSize(plan.totalBytes, i18n.lang) })}
        </button>
      {/snippet}
    </Dialog>
  {/if}
</div>

<style>
  .page {
    max-width: 920px;
    padding: var(--sp-4) var(--sp-6) var(--sp-12);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .back {
    align-self: flex-start;
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .summary {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sp-8);
    padding: var(--sp-5) var(--sp-6);
  }

  .big {
    font-size: var(--fs-2xl);
    font-weight: var(--fw-bold);
  }

  .adm {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-inline-start: auto;
    font-size: var(--fs-sm);
    color: var(--text-2);
  }

  .warn {
    display: flex;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border-radius: var(--r-lg);
    background: var(--careful-soft);
    font-size: var(--fs-sm);
  }

  .warn > :global(svg) {
    color: var(--careful);
    margin-top: 2px;
  }

  .warn.recycle {
    background: var(--probably-safe-soft);
  }

  .warn.recycle > :global(svg) {
    color: var(--probably-safe);
  }

  .warn > div {
    flex: 1;
  }

  .wt {
    font-weight: var(--fw-bold);
    margin-bottom: 2px;
  }

  .opt {
    margin-top: var(--sp-3);
    padding-top: var(--sp-3);
    border-top: 1px solid color-mix(in srgb, var(--probably-safe) 25%, transparent);
  }

  .opt-card {
    padding: var(--sp-4) var(--sp-5);
  }

  .actions {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow: hidden;
  }

  .small {
    font-size: var(--fs-xs);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .dryres {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .dryres p {
    color: var(--text);
  }

  .bar {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    padding: var(--sp-4) 0;
    background: linear-gradient(transparent, var(--bg) 30%);
  }

  .btns {
    display: flex;
    gap: var(--sp-2);
  }

  .confirm-list {
    list-style: none;
    margin: var(--sp-3) 0;
    padding: 0;
    max-height: 220px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--r-md);
  }

  .confirm-list li {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-3);
    padding: 6px var(--sp-3);
    font-size: var(--fs-xs);
    border-top: 1px solid var(--border);
  }

  .confirm-list li:first-child {
    border-top: 0;
  }

  .confirm-list .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
