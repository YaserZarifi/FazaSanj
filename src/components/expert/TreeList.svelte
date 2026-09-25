<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { ChildSort, NodeInfo } from "../../lib/api/types";
  import { formatCompact, formatPercent, formatRelative, formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import Breadcrumb from "./Breadcrumb.svelte";
  import RowMenu from "./RowMenu.svelte";

  const PAGE = 100;

  let sort = $state<ChildSort>("size");
  let items = $state<NodeInfo[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let loadingMore = $state(false);
  let menu = $state<{ node: NodeInfo; x: number; y: number } | null>(null);
  let listEl = $state<HTMLElement>();
  let sentinel = $state<HTMLElement>();
  let request = 0;

  const folder = $derived(scan.current);

  async function load(reset: boolean) {
    const f = folder;
    if (!f || scan.scanId == null) return;
    const my = ++request;
    if (reset) loading = true;
    else loadingMore = true;
    try {
      const page = await backend.getChildren(scan.scanId, f.id, sort, reset ? 0 : items.length, PAGE);
      if (my !== request) return;
      items = reset ? page.items : [...items, ...page.items];
      total = page.total;
    } catch (e) {
      toasts.error(e);
    } finally {
      if (my === request) {
        loading = false;
        loadingMore = false;
      }
    }
  }

  $effect(() => {
    void folder?.id;
    void sort;
    void scan.version;
    items = [];
    void load(true);
  });

  // keep loading pages as the user scrolls to the end
  $effect(() => {
    if (!sentinel) return;
    const io = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting && !loadingMore && !loading && items.length < total) void load(false);
    });
    io.observe(sentinel);
    return () => io.disconnect();
  });

  function drill(n: NodeInfo) {
    if (n.isDir && n.childCount > 0) {
      scan.enter(n);
      scan.selected = null;
    }
  }

  function focusRow(i: number) {
    const rows = listEl?.querySelectorAll<HTMLElement>("[data-row]");
    const r = rows?.[Math.max(0, Math.min(i, (rows?.length ?? 1) - 1))];
    r?.focus();
  }

  function onRowKey(e: KeyboardEvent, n: NodeInfo, i: number) {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        focusRow(i + 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        focusRow(i - 1);
        break;
      case "Home":
        e.preventDefault();
        focusRow(0);
        break;
      case "End":
        e.preventDefault();
        focusRow(items.length - 1);
        break;
      case "Enter":
        e.preventDefault();
        drill(n);
        break;
      case "Backspace":
        e.preventDefault();
        scan.up();
        break;
      case "ContextMenu":
      case "F10": {
        if (e.key === "F10" && !e.shiftKey) return;
        e.preventDefault();
        const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
        menu = { node: n, x: r.left + 40, y: r.bottom };
        break;
      }
    }
  }

  function openMenu(e: MouseEvent, n: NodeInfo) {
    e.preventDefault();
    menu = { node: n, x: e.clientX, y: e.clientY };
  }

  const SORTS: { key: ChildSort; label: string }[] = $derived([
    { key: "name", label: t("tree.colName") },
    { key: "size", label: t("tree.colSize") },
    { key: "modified", label: t("tree.colModified") },
  ]);
</script>

<div class="tree">
  <div class="bar">
    {#if scan.trail.length > 1}
      <button type="button" class="icon-btn" aria-label={t("tree.up")} title={t("tree.up")} onclick={() => scan.up()}>
        <Icon name="arrow-up" size={17} />
      </button>
    {/if}
    <Breadcrumb items={scan.trail.map((n) => ({ id: n.id, name: n.name }))} onpick={(id) => scan.openFolder(id)} />
  </div>

  <div class="head" role="presentation">
    {#each SORTS as s (s.key)}
      <button
        type="button"
        class="sort {s.key}"
        class:on={sort === s.key}
        aria-pressed={sort === s.key}
        onclick={() => (sort = s.key)}
      >
        {s.label}
        {#if sort === s.key}<Icon name={s.key === "name" ? "arrow-up" : "arrow-down"} size={13} />{/if}
      </button>
    {/each}
    <span class="files-h">{t("tree.colFiles")}</span>
  </div>

  {#if loading && items.length === 0}
    <div class="sk">
      {#each Array(8) as _, i (i)}
        <Skeleton height={34} />
      {/each}
    </div>
  {:else if items.length === 0}
    <EmptyState compact icon="folder" title={t("tree.empty")} />
  {:else}
    <ul class="rows" bind:this={listEl} aria-label={t("tree.listLabel", { name: folder?.name ?? "" })}>
      {#each items as n, i (n.id)}
        {@const pct = folder && folder.size > 0 ? n.size / folder.size : 0}
        <li>
          <div
            class="row"
            class:sel={scan.selected?.id === n.id}
            data-row
            role="button"
            tabindex={i === 0 ? 0 : -1}
            aria-label={`${n.name}, ${formatSize(n.size, i18n.lang)}`}
            onclick={() => (scan.selected = n)}
            ondblclick={() => drill(n)}
            onkeydown={(e) => onRowKey(e, n, i)}
            onfocus={() => (scan.selected = n)}
            oncontextmenu={(e) => openMenu(e, n)}
          >
            <span class="name">
              <span class="ficon" style:color="var(--cat-{n.category})">
                <Icon name={n.isDir ? "folder" : "file"} size={17} />
              </span>
              <span class="nm ltr" title={n.name}>{n.name}</span>
              {#if n.explanation}<SafetyBadge level={n.explanation.safety} compact />{/if}
              {#if n.flags.cloudOnly}<span class="flag" title={t("flags.cloudOnly")}><Icon name="cloud" size={14} /></span>{/if}
              {#if n.flags.accessDenied}<span class="flag" title={t("flags.accessDenied")}><Icon name="ban" size={14} /></span>{/if}
              {#if n.flags.reparse}<span class="flag" title={t("flags.reparse")}><Icon name="link" size={14} /></span>{/if}
            </span>
            <span class="size">
              <span class="pbar" aria-hidden="true"><span style:width="{pct * 100}%" style:background="var(--cat-{n.category})"></span></span>
              <span class="pct num">{formatPercent(pct, i18n.lang)}</span>
              <span class="bytes num">{formatSize(n.size, i18n.lang)}</span>
            </span>
            <span class="mod">{formatRelative(n.modified, i18n.lang)}</span>
            <span class="files num">{n.isDir ? formatCompact(n.fileCount, i18n.lang) : ""}</span>
            <button
              type="button"
              class="icon-btn more"
              tabindex="-1"
              aria-label={t("tree.actions")}
              onclick={(e) => {
                e.stopPropagation();
                openMenu(e, n);
              }}
            >
              <Icon name="more" size={16} />
            </button>
          </div>
        </li>
      {/each}
    </ul>
    <div bind:this={sentinel} class="more-row">
      {#if items.length < total}
        <button type="button" class="btn btn-sm" disabled={loadingMore} onclick={() => load(false)}>
          {t("tree.loadMore", { shown: items.length, total })}
        </button>
      {:else}
        <span class="faint">{t("tree.count", { count: total })}</span>
      {/if}
    </div>
  {/if}
</div>

{#if menu}
  <RowMenu node={menu.node} x={menu.x} y={menu.y} onclose={() => (menu = null)} />
{/if}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding-bottom: var(--sp-3);
  }

  .head,
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 260px 120px 70px 34px;
    align-items: center;
    gap: var(--sp-3);
  }

  .head {
    padding: 0 var(--sp-3) var(--sp-2);
    border-bottom: 1px solid var(--border);
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .head .sort.size {
    text-align: end;
  }

  .sort {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 0;
    background: none;
    padding: 2px 4px;
    color: inherit;
    font-size: inherit;
    cursor: pointer;
    border-radius: var(--r-xs);
  }

  .sort.size {
    justify-content: flex-end;
  }

  .sort.on {
    color: var(--text);
    font-weight: var(--fw-medium);
  }

  .files-h {
    text-align: end;
    grid-column: 4;
  }

  .rows {
    list-style: none;
    margin: 0;
    padding: var(--sp-1) 0;
  }

  .row {
    padding: 5px var(--sp-3);
    border-radius: var(--r-md);
    cursor: default;
    user-select: none;
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .row.sel {
    background: var(--accent-soft);
  }

  .row:focus-visible {
    outline-offset: -2px;
  }

  .name {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    min-width: 0;
  }

  .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .flag {
    color: var(--text-3);
    display: inline-flex;
  }

  .size {
    display: grid;
    grid-template-columns: 1fr 44px 80px;
    align-items: center;
    gap: var(--sp-2);
  }

  .pbar {
    height: 6px;
    border-radius: var(--r-full);
    background: var(--surface-3);
    overflow: hidden;
  }

  .pbar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    min-width: 2px;
  }

  .pct {
    font-size: var(--fs-xs);
    color: var(--text-3);
    text-align: end;
  }

  .bytes {
    text-align: end;
    font-weight: var(--fw-medium);
  }

  .mod {
    font-size: var(--fs-xs);
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .files {
    font-size: var(--fs-xs);
    color: var(--text-2);
    text-align: end;
  }

  .more {
    width: 28px;
    height: 28px;
    opacity: 0;
  }

  .row:hover .more,
  .row.sel .more {
    opacity: 1;
  }

  .sk {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: var(--sp-2);
  }

  .more-row {
    display: flex;
    justify-content: center;
    padding: var(--sp-3);
    font-size: var(--fs-xs);
  }
</style>
