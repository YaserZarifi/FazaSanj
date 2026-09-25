<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { NodeInfo } from "../../lib/api/types";
  import { formatDate, formatNumber, formatSize } from "../../lib/format";
  import { bi, categoryName, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import AskAi from "../ai/AskAi.svelte";
  import AiWarning from "../common/AiWarning.svelte";
  import CategoryIcon from "../common/CategoryIcon.svelte";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import SourceBadge from "../common/SourceBadge.svelte";

  const node = $derived<NodeInfo | null>(scan.selected ?? scan.current);
  const e = $derived(node?.explanation ?? null);
  const inBasket = $derived(node ? cleanup.has(node.path) : false);
  const canClean = $derived(!!e && e.safety !== "do_not_touch" && e.method !== "manual_only");

  const flags = $derived.by(() => {
    if (!node) return [];
    const out: string[] = [];
    const f = node.flags;
    if (f.cloudOnly) out.push(t("flags.cloudOnly"));
    if (f.hardlinkDup) out.push(t("flags.hardlink"));
    if (f.compressed) out.push(t("flags.compressed"));
    if (f.sparse) out.push(t("flags.sparse"));
    if (f.reparse) out.push(t("flags.reparse"));
    if (f.accessDenied) out.push(t("flags.accessDenied"));
    if (f.system) out.push(t("flags.system"));
    return out;
  });
</script>

<aside class="panel" aria-label={t("details.title")}>
  {#if !node}
    <p class="muted">{t("details.pick")}</p>
  {:else}
    <div class="head">
      <CategoryIcon category={node.category} size={40} />
      <div class="hn">
        <h3 class="ltr" title={node.name}>{node.name}</h3>
        <p class="faint small">{node.isDir ? t("details.folder") : t("details.file")} · {categoryName(node.category)}</p>
      </div>
    </div>

    <p class="path full">{node.path}</p>

    <dl>
      <div><dt>{t("details.size")}</dt><dd class="num">{formatSize(node.size, i18n.lang)}</dd></div>
      {#if node.isDir}
        <div><dt>{t("details.files")}</dt><dd class="num">{formatNumber(node.fileCount, i18n.lang)}</dd></div>
        <div><dt>{t("details.folders")}</dt><dd class="num">{formatNumber(node.dirCount, i18n.lang)}</dd></div>
      {/if}
      <div><dt>{t("details.modified")}</dt><dd>{formatDate(node.modified, i18n.lang, true) || "-"}</dd></div>
    </dl>

    {#if flags.length}
      <div class="flags">
        {#each flags as f (f)}<span class="flag">{f}</span>{/each}
      </div>
    {/if}

    {#if e}
      <section class="exp">
        <div class="badges">
          <SafetyBadge level={e.safety} />
          <SourceBadge source={e.source} />
          {#if e.needsAdmin}<span class="admin"><Icon name="shield" size={14} />{t("cleanup.adminShort")}</span>{/if}
        </div>
        <h4>{bi(e.title)}</h4>
        <p>{bi(e.whyBig)}</p>
        <p class="lbl">{t("reason.whatIf")}</p>
        <p>{bi(e.ifDeleted)}</p>
        {#if e.instructions}<p class="instr">{bi(e.instructions)}</p>{/if}
        <p class="faint small">{t(`methods.${e.method}`)}</p>
        {#if e.source === "ai"}<AiWarning />{/if}
      </section>
    {:else}
      <AskAi {node} />
    {/if}

    <div class="actions">
      {#if canClean && e}
        <button type="button" class="btn btn-primary btn-sm" aria-pressed={inBasket} onclick={() => cleanup.toggle({ path: node.path, bytes: node.size, explanation: e })}>
          <Icon name={inBasket ? "check" : "plus"} size={15} />{inBasket ? t("reason.inCleanup") : t("reason.addToCleanup")}
        </button>
      {/if}
      <button type="button" class="btn btn-sm" onclick={() => backend.revealInExplorer(node.path).catch((err: unknown) => toasts.error(err))}>
        <Icon name="external" size={15} />{t("tree.reveal")}
      </button>
      {#if node.isDir && node.childCount > 0 && scan.current?.id !== node.id}
        <button type="button" class="btn btn-sm" onclick={() => scan.openFolder(node.id)}>
          <Icon name="folder-open" size={15} />{t("tree.open")}
        </button>
      {/if}
    </div>
  {/if}
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
    padding: var(--sp-5);
    border-inline-start: 1px solid var(--border);
    background: var(--surface);
    overflow-y: auto;
    min-height: 0;
  }

  .head {
    display: flex;
    gap: var(--sp-3);
    align-items: center;
  }

  .hn {
    min-width: 0;
  }

  h3 {
    font-size: var(--fs-lg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .small {
    font-size: var(--fs-xs);
  }

  .full {
    font-size: var(--fs-xs);
    color: var(--text-2);
    padding: var(--sp-2) var(--sp-3);
    background: var(--surface-2);
    border-radius: var(--r-sm);
    text-align: left;
  }

  dl {
    margin: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-2);
  }

  dl div {
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--r-md);
    background: var(--surface-2);
  }

  dt {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  dd {
    margin: 0;
    font-weight: var(--fw-medium);
    font-size: var(--fs-sm);
  }

  .flags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .flag {
    font-size: 11px;
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--surface-3);
    color: var(--text-2);
  }

  .exp {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    font-size: var(--fs-sm);
    padding-top: var(--sp-3);
    border-top: 1px solid var(--border);
  }

  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .admin {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  h4 {
    font-size: var(--fs-md);
  }

  .lbl {
    font-weight: var(--fw-bold);
    margin-top: var(--sp-1);
  }

  .instr {
    padding: var(--sp-2) var(--sp-3);
    border-radius: var(--r-md);
    background: var(--surface-2);
    color: var(--text-2);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-2);
    padding-top: var(--sp-2);
  }
</style>
