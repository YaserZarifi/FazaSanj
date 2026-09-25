<script lang="ts">
  import type { ActionResult, CleanupAction } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { bi, errorText, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";

  let { action, dry }: { action: CleanupAction; dry: ActionResult | null } = $props();

  // what "delete" really means for this item with the current options
  const permanent = $derived(
    action.permanent || (cleanup.options.permanentForSafe && action.safety === "safe" && action.method === "delete_contents"),
  );
  const methodText = $derived.by(() => {
    if (action.method === "recycle" || action.method === "delete_contents") {
      return t(`methodExplain.${action.method}${permanent ? "Permanent" : ""}`);
    }
    return t(`methodExplain.${action.method}`);
  });
</script>

<li class="action" class:blocked={action.blocked}>
  <div class="top">
    <div class="main">
      <div class="line">
        <h4>{bi(action.title)}</h4>
        {#if action.needsAdmin}
          <span class="admin" title={t("cleanup.needsAdmin")}><Icon name="shield" size={15} /><span class="sr-only">{t("cleanup.needsAdmin")}</span></span>
        {/if}
      </div>
      <PathText path={action.path} max={80} />
    </div>
    <SafetyBadge level={action.safety} />
    <span class="size num">{formatSize(action.bytes, i18n.lang)}</span>
  </div>

  {#if action.blocked}
    <p class="why-blocked"><Icon name="lock" size={15} /> {t("cleanup.blockedReason")}</p>
  {:else}
    <p class="method"><Icon name={permanent ? "trash" : action.method === "recycle" || action.method === "delete_contents" ? "recycle" : "info"} size={15} /> {methodText}</p>
    {#if action.command}<p class="cmd ltr"><code>{action.command}</code></p>{/if}
    <p class="cons">{bi(action.consequence)}</p>
    {#if action.instructions}<p class="instr">{bi(action.instructions)}</p>{/if}
  {/if}

  {#if dry}
    <div class="dry">
      {#if dry.status === "blocked"}
        <p>{errorText(dry.error)}</p>
      {:else if dry.wouldRemove.length}
        <p class="dl">{t("dry.wouldRemove", { size: formatSize(dry.bytesFreed, i18n.lang), count: dry.filesRemoved })}</p>
        <ul>
          {#each dry.wouldRemove as p (p)}<li class="path">{p}</li>{/each}
        </ul>
      {:else}
        <p class="dl">{t("dry.nothingRemoved")}</p>
      {/if}
    </div>
  {/if}
</li>

<style>
  .action {
    padding: var(--sp-4) var(--sp-5);
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .action:first-child {
    border-top: 0;
  }

  .blocked {
    opacity: 0.65;
    background: var(--surface-2);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  .main {
    flex: 1;
    min-width: 0;
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  h4 {
    font-size: var(--fs-md);
  }

  .admin {
    color: var(--accent-text);
    display: inline-flex;
  }

  .size {
    font-weight: var(--fw-bold);
    min-width: 90px;
    text-align: end;
  }

  .method,
  .why-blocked {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-sm);
    font-weight: var(--fw-medium);
  }

  .why-blocked {
    color: var(--do-not-touch);
  }

  .cmd code {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 2px 8px;
    border-radius: var(--r-sm);
    background: var(--surface-3);
  }

  .cons,
  .instr {
    font-size: var(--fs-sm);
    color: var(--text-2);
  }

  .dry {
    margin-top: 4px;
    padding: var(--sp-3);
    border-radius: var(--r-md);
    background: var(--accent-soft);
    font-size: var(--fs-sm);
  }

  .dl {
    font-weight: var(--fw-medium);
  }

  .dry ul {
    margin: 4px 0 0;
    padding-inline-start: var(--sp-5);
    font-size: var(--fs-xs);
    color: var(--text-2);
  }
</style>
