<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { NodeId, NodeInfo, ScanSummary } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import VizView from "../viz/VizView.svelte";
  import AccessDenied from "./AccessDenied.svelte";
  import ByType from "./ByType.svelte";
  import DetailPanel from "./DetailPanel.svelte";
  import Findings from "./Findings.svelte";
  import FolderList from "./FolderList.svelte";
  import LargestFiles from "./LargestFiles.svelte";
  import ScannerCompare from "./ScannerCompare.svelte";

  let { summary }: { summary: ScanSummary } = $props();

  type Tab = "folders" | "largest" | "types" | "findings" | "denied" | "compare";
  type Look = "list" | "treemap" | "sunburst";

  let tab = $state<Tab>("folders");
  let look = $state<Look>("list");
  let currentId = $state<NodeId>(0);
  let selectedId = $state<NodeId | null>(null);
  let crumbs = $state<NodeInfo[]>([]);

  const drive = $derived(app.drives.find((d) => d.root.toLowerCase() === summary.rootPath.toLowerCase()) ?? null);
  const tabs = $derived.by(() => {
    const list: Tab[] = ["folders", "largest", "types", "findings"];
    if (summary.accessDenied > 0) list.push("denied");
    if (drive?.fastScanAvailable) list.push("compare");
    return list;
  });

  $effect(() => {
    currentId = summary.rootNode;
    selectedId = null;
  });

  // Breadcrumb from the current folder up to the root.
  $effect(() => {
    const [scanId, id] = [summary.scanId, currentId];
    (async () => {
      const chain: NodeInfo[] = [];
      let cur: NodeId | null = id;
      while (cur !== null && chain.length < 64) {
        const n: NodeInfo = await api.getNode(scanId, cur);
        chain.unshift(n);
        cur = n.parent;
      }
      if (currentId === id) crumbs = chain;
    })().catch(() => {});
  });

  function open(id: NodeId) {
    currentId = id;
    selectedId = id;
    tab = "folders";
  }

  function select(id: NodeId) {
    selectedId = id;
  }

  const looks: { id: Look; icon: string }[] = [
    { id: "list", icon: "list" },
    { id: "treemap", icon: "grid" },
    { id: "sunburst", icon: "pie" },
  ];
</script>

<div class="expert">
  <div class="main">
    <div class="tabs" role="tablist">
      {#each tabs as id (id)}
        <button class="tab" role="tab" aria-selected={tab === id} onclick={() => (tab = id)}>{t(`expert.tab.${id}`)}</button>
      {/each}
    </div>

    {#if tab === "folders"}
      <div class="toolbar">
        <nav class="crumbs" aria-label={t("expert.breadcrumb")}>
          {#each crumbs as c, i (c.id)}
            {#if i > 0}<Icon name="chevron-end" size={14} class="sep" />{/if}
            <button class="crumb" class:current={i === crumbs.length - 1} onclick={() => open(c.id)}>
              <bdi>{i === 0 ? c.path : c.name}</bdi>
            </button>
          {/each}
        </nav>
        <span class="spacer"></span>
        {#if crumbs.length > 1}
          <button class="btn small" onclick={() => open(crumbs[crumbs.length - 2].id)}>
            <Icon name="arrow-start" size={14} />
            {t("expert.up")}
          </button>
        {/if}
        <div class="looks" role="radiogroup" aria-label={t("expert.view")}>
          {#each looks as l (l.id)}
            <button
              role="radio"
              aria-checked={look === l.id}
              class:on={look === l.id}
              onclick={() => (look = l.id)}
              title={t(`expert.look.${l.id}`)}
              aria-label={t(`expert.look.${l.id}`)}
            >
              <Icon name={l.icon} size={16} />
            </button>
          {/each}
        </div>
      </div>
      <div class="pane">
        {#if look === "list"}
          <FolderList scanId={summary.scanId} nodeId={currentId} {selectedId} onopen={open} onselect={select} />
        {:else}
          <VizView scanId={summary.scanId} nodeId={currentId} kind={look} onopen={open} onselect={select} />
        {/if}
      </div>
    {:else if tab === "largest"}
      <div class="pane scroll"><LargestFiles scanId={summary.scanId} {selectedId} onselect={select} /></div>
    {:else if tab === "types"}
      <div class="pane scroll"><ByType scanId={summary.scanId} /></div>
    {:else if tab === "findings"}
      <div class="pane scroll"><Findings scanId={summary.scanId} onselect={select} /></div>
    {:else if tab === "denied"}
      <div class="pane scroll"><AccessDenied scanId={summary.scanId} /></div>
    {:else if tab === "compare"}
      <div class="pane scroll"><ScannerCompare root={summary.rootPath} /></div>
    {/if}
  </div>

  <DetailPanel scanId={summary.scanId} nodeId={selectedId ?? currentId} {currentId} onopen={open} />
</div>

<style>
  .expert {
    flex: 1;
    min-height: 0;
    display: flex;
    height: calc(100vh - var(--topbar-h) - 60px);
    min-height: 480px;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
  }
  .tabs {
    padding: 0 16px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    border-bottom: 1px solid var(--border);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    white-space: nowrap;
  }
  .crumbs :global(.sep) {
    color: var(--text-3);
  }
  .crumb {
    border: none;
    background: none;
    padding: 3px 6px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-2);
  }
  .crumb:hover {
    background: var(--surface-2);
  }
  .crumb.current {
    color: var(--text);
    font-weight: 700;
  }
  .looks {
    display: flex;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    padding: 3px;
    gap: 2px;
  }
  .looks button {
    border: none;
    background: none;
    padding: 4px 8px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-2);
    display: grid;
    place-items: center;
  }
  .looks button.on {
    background: var(--surface);
    color: var(--accent);
    box-shadow: var(--shadow);
  }
  .pane {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 8px;
  }
  .pane.scroll {
    overflow: auto;
    padding: 16px;
  }
</style>
