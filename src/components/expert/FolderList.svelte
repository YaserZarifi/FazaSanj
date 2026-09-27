<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { ChildSort, NodeId, NodeInfo, ScanId } from "../../lib/api/types";
  import { formatDate, formatInt, formatPercent, formatSize } from "../../lib/format";
  import { bi, errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { categoryColor, safetyVar } from "../../lib/labels";
  import Icon from "../common/Icon.svelte";
  import ShareBar from "../common/ShareBar.svelte";
  import Spinner from "../common/Spinner.svelte";

  let {
    scanId,
    nodeId,
    selectedId,
    onopen,
    onselect,
  }: {
    scanId: ScanId;
    nodeId: NodeId;
    selectedId: NodeId | null;
    onopen: (id: NodeId) => void;
    onselect: (id: NodeId) => void;
  } = $props();

  const PAGE = 100;
  let sort = $state<ChildSort>("size");
  let items = $state<NodeInfo[]>([]);
  let total = $state(0);
  let parentSize = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    const [s, n, so] = [scanId, nodeId, sort];
    items = [];
    total = 0;
    error = null;
    api.getNode(s, n).then((p) => (parentSize = p.size)).catch(() => {});
    load(s, n, so, 0);
  });

  async function load(s: ScanId, n: NodeId, so: ChildSort, offset: number) {
    loading = true;
    try {
      const page = await api.getChildren(s, n, so, offset, PAGE);
      if (s !== scanId || n !== nodeId || so !== sort) return;
      items = offset === 0 ? page.items : [...items, ...page.items];
      total = page.total;
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  function onkey(e: KeyboardEvent, item: NodeInfo) {
    if (e.key === "Enter" && item.isDir) onopen(item.id);
    else if (e.key === " ") {
      e.preventDefault();
      onselect(item.id);
    }
  }

  const sorts: ChildSort[] = ["size", "name", "modified"];
</script>

<div class="wrap">
  {#if error}
    <div class="notice error">{error}</div>
  {/if}
  <table class="list">
    <thead>
      <tr>
        {#each sorts as s (s)}
          <th class:num={s !== "name"} aria-sort={sort === s ? "descending" : "none"}>
            <button class="sort" class:on={sort === s} onclick={() => (sort = s)}>{t(`list.${s}`)}</button>
          </th>
        {/each}
        <th class="num">{t("list.share")}</th>
        <th class="num">{t("list.files")}</th>
      </tr>
    </thead>
    <tbody>
      {#each items as item (item.id)}
        <tr
          class="clickable"
          class:selected={selectedId === item.id}
          tabindex="0"
          onclick={() => onselect(item.id)}
          ondblclick={() => item.isDir && onopen(item.id)}
          onkeydown={(e) => onkey(e, item)}
        >
          <td class="name-cell">
            <span class="row">
              <span class="cat" style:background={categoryColor(item.category)}></span>
              <Icon name={item.isDir ? "folder" : "file"} size={16} />
              {#if item.isDir}
                <button class="link" onclick={(e) => { e.stopPropagation(); onopen(item.id); }}><bdi dir="auto" class="nm">{item.name}</bdi></button>
              {:else}
                <bdi dir="auto" class="nm">{item.name}</bdi>
              {/if}
              {#if item.flags.cloudOnly}<span title={t("flags.cloud")}><Icon name="cloud" size={14} /></span>{/if}
              {#if item.flags.accessDenied}<span title={t("flags.denied")}><Icon name="lock" size={14} /></span>{/if}
              {#if item.flags.reparse}<span class="faint" title={t("flags.link")}>↪</span>{/if}
              {#if item.explanation}
                <span class="tag" style:--c="var({safetyVar(item.explanation.safety)})">{bi(item.explanation.title)}</span>
              {/if}
            </span>
          </td>
          <td class="num"><strong>{formatSize(item.size, lang())}</strong></td>
          <td class="num faint">{formatDate(item.modified, lang())}</td>
          <td class="num share">
            <span class="row"><ShareBar share={parentSize ? item.size / parentSize : 0} color={categoryColor(item.category)} />
              <span class="faint pct">{formatPercent(parentSize ? item.size / parentSize : 0, lang())}</span></span>
          </td>
          <td class="num faint">{item.isDir ? formatInt(item.fileCount, lang()) : ""}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if loading}
    <div class="center"><Spinner /></div>
  {:else if items.length === 0 && !error}
    <div class="center faint">{t("list.empty")}</div>
  {:else if items.length < total}
    <div class="center">
      <button class="btn small" onclick={() => load(scanId, nodeId, sort, items.length)}>
        {t("list.more", { count: total - items.length })}
      </button>
    </div>
  {/if}
</div>

<style>
  .wrap {
    flex: 1;
    overflow: auto;
  }
  .sort {
    border: none;
    background: none;
    cursor: pointer;
    color: inherit;
    font: inherit;
    padding: 0;
  }
  .sort.on {
    color: var(--accent);
    font-weight: 700;
  }
  .name-cell {
    max-width: 0;
    width: 50%;
  }
  .name-cell .row {
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
  }
  .nm {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .cat {
    width: 4px;
    height: 18px;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .link {
    min-width: 0;
    border: none;
    background: none;
    padding: 0;
    color: inherit;
    cursor: pointer;
    font: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .link:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .tag {
    font-size: 11.5px;
    padding: 0 8px;
    border-radius: 99px;
    color: var(--c);
    border: 1px solid var(--c);
    white-space: nowrap;
  }
  .share {
    width: 150px;
  }
  .pct {
    min-width: 44px;
  }
  .center {
    display: grid;
    place-items: center;
    padding: 16px;
  }
</style>
