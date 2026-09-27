<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { NodeId, NodeInfo, ScanId } from "../../lib/api/types";
  import { formatDate, formatInt, formatSize } from "../../lib/format";
  import { bi, errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { CATEGORY_ICONS, categoryColor, categoryKey, methodKey } from "../../lib/labels";
  import { app } from "../../lib/stores/app.svelte";
  import AiExplain from "../ai/AiExplain.svelte";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import GrowthChart from "../growth/GrowthChart.svelte";

  let {
    scanId,
    nodeId,
    currentId,
    onopen,
  }: { scanId: ScanId; nodeId: NodeId; currentId: NodeId; onopen: (id: NodeId) => void } = $props();

  let node = $state<NodeInfo | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const [s, n] = [scanId, nodeId];
    error = null;
    api
      .getNode(s, n)
      .then((x) => (node = x))
      .catch((e) => (error = errorText(e)));
  });

  const e = $derived(node?.explanation ?? null);
  const canClean = $derived(!!e && e.method !== "manual_only" && e.safety !== "do_not_touch");
</script>

<aside class="panel" aria-label={t("detail.title")}>
  {#if error}
    <div class="notice error">{error}</div>
  {:else if node}
    <div class="head">
      <span class="icon" style:background={categoryColor(node.category)}>
        <Icon name={node.isDir ? "folder" : CATEGORY_ICONS[node.category]} size={18} />
      </span>
      <div class="title">
        <h3><bdi>{node.name}</bdi></h3>
        <span class="faint">{t(categoryKey(node.category))}</span>
      </div>
    </div>

    <div class="big">{formatSize(node.size, lang())}</div>
    <bdi class="path faint">{node.path}</bdi>

    <dl>
      {#if node.isDir}
        <dt>{t("detail.contents")}</dt>
        <dd>{t("detail.counts", { files: formatInt(node.fileCount, lang()), folders: formatInt(node.dirCount, lang()) })}</dd>
      {/if}
      {#if node.modified}
        <dt>{t("detail.modified")}</dt>
        <dd>{formatDate(node.modified, lang(), true)}</dd>
      {/if}
      {#if node.flags.cloudOnly || node.flags.accessDenied || node.flags.reparse || node.flags.compressed || node.flags.hardlinkDup}
        <dt>{t("detail.notes")}</dt>
        <dd class="flags">
          {#if node.flags.cloudOnly}<span>{t("flags.cloud")}</span>{/if}
          {#if node.flags.accessDenied}<span>{t("flags.denied")}</span>{/if}
          {#if node.flags.reparse}<span>{t("flags.link")}</span>{/if}
          {#if node.flags.compressed}<span>{t("flags.compressed")}</span>{/if}
          {#if node.flags.hardlinkDup}<span>{t("flags.hardlink")}</span>{/if}
        </dd>
      {/if}
    </dl>

    <div class="row wrap">
      {#if node.isDir && node.id !== currentId}
        <button class="btn small" onclick={() => onopen(node!.id)}><Icon name="folder-open" size={14} />{t("detail.open")}</button>
      {/if}
      <button class="btn small" onclick={() => api.revealInExplorer(node!.path)}>
        <Icon name="external" size={14} />{t("common.showInExplorer")}
      </button>
    </div>

    {#if e}
      <section class="explain">
        <div class="row">
          <h3>{bi(e.title)}</h3>
          <span class="spacer"></span>
          <SafetyBadge level={e.safety} small />
        </div>
        <p>{bi(e.whyBig)}</p>
        <h4>{t("reason.whatHappens")}</h4>
        <p>{bi(e.ifDeleted)}</p>
        {#if e.instructions}
          <h4>{t("reason.howTo")}</h4>
          <p class="pre">{bi(e.instructions)}</p>
        {/if}
        <p class="faint">{t("reason.method")}: {t(methodKey(e.method))}{e.needsAdmin ? ` · ${t("detail.needsAdmin")}` : ""}</p>
        {#if canClean}
          <button class="btn primary" onclick={() => app.openCleanup([{ path: node!.path, bytes: node!.size, explanation: e! }])}>
            <Icon name="trash" size={15} />
            {e.method === "open_app_setting" ? t("detail.openSetting") : t("detail.cleanUp")}
          </button>
        {/if}
      </section>
    {:else if node.isDir && node.parent !== null}
      <section class="explain">
        <p class="muted">{t("detail.unknown")}</p>
        {#if app.settings.aiEnabled}
          <AiExplain {scanId} nodeId={node.id} />
        {:else}
          <p class="faint">{t("detail.aiOff")}</p>
        {/if}
      </section>
    {/if}

    {#if node.isDir}
      <GrowthChart path={node.path} compact />
    {/if}
  {/if}
</aside>

<style>
  .panel {
    width: 340px;
    flex-shrink: 0;
    border-inline-start: 1px solid var(--border);
    background: var(--surface);
    padding: 16px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .icon {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: #fff;
    flex-shrink: 0;
  }
  .title {
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title h3 {
    overflow-wrap: anywhere;
  }
  .big {
    font-size: 24px;
    font-weight: 700;
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 12px;
  }
  dt {
    color: var(--text-3);
  }
  dd {
    margin: 0;
  }
  .flags {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
  }
  .wrap {
    flex-wrap: wrap;
  }
  .explain {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    background: var(--surface-2);
    border-radius: var(--radius);
  }
  h4 {
    font-size: 13px;
    color: var(--text-2);
  }
  .pre {
    white-space: pre-line;
  }
</style>
