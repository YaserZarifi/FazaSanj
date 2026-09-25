<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { ui, type ExpertTab } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";
  import type { IconName } from "../common/icons";
  import HierarchyChart from "../viz/HierarchyChart.svelte";
  import AccessDenied from "./AccessDenied.svelte";
  import ByType from "./ByType.svelte";
  import DetailsPanel from "./DetailsPanel.svelte";
  import LargestFiles from "./LargestFiles.svelte";
  import TreeList from "./TreeList.svelte";

  const TABS: { id: ExpertTab; icon: IconName }[] = [
    { id: "tree", icon: "list" },
    { id: "treemap", icon: "treemap" },
    { id: "sunburst", icon: "sunburst" },
    { id: "largest", icon: "file-stack" },
    { id: "types", icon: "shapes" },
    { id: "denied", icon: "ban" },
  ];

  function onTabKey(e: KeyboardEvent) {
    const i = TABS.findIndex((x) => x.id === ui.expertTab);
    const rtl = document.documentElement.dir === "rtl";
    let n = -1;
    if (e.key === "ArrowRight") n = rtl ? i - 1 : i + 1;
    else if (e.key === "ArrowLeft") n = rtl ? i + 1 : i - 1;
    else return;
    e.preventDefault();
    ui.expertTab = TABS[(n + TABS.length) % TABS.length].id;
    const list = e.currentTarget as HTMLElement;
    requestAnimationFrame(() => list.querySelector<HTMLElement>("[aria-selected=true]")?.focus());
  }
</script>

<div class="expert">
  <div class="left">
    <div class="tabs" role="tablist" aria-label={t("expert.tabs")} tabindex="-1" onkeydown={onTabKey}>
      {#each TABS as tab (tab.id)}
        <button
          type="button"
          role="tab"
          id="tab-{tab.id}"
          aria-selected={ui.expertTab === tab.id}
          aria-controls="panel-{tab.id}"
          tabindex={ui.expertTab === tab.id ? 0 : -1}
          onclick={() => (ui.expertTab = tab.id)}
        >
          <Icon name={tab.icon} size={16} />{t(`expert.tab.${tab.id}`)}
        </button>
      {/each}
    </div>
    <div class="panel" role="tabpanel" id="panel-{ui.expertTab}" aria-labelledby="tab-{ui.expertTab}">
      {#if ui.expertTab === "tree"}
        <TreeList />
      {:else if ui.expertTab === "treemap"}
        <HierarchyChart kind="treemap" />
      {:else if ui.expertTab === "sunburst"}
        <HierarchyChart kind="sunburst" />
      {:else if ui.expertTab === "largest"}
        <LargestFiles />
      {:else if ui.expertTab === "types"}
        <ByType />
      {:else}
        <AccessDenied />
      {/if}
    </div>
  </div>
  <DetailsPanel />
</div>

<style>
  .expert {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--details-w);
    min-height: calc(100vh - var(--topbar-h) - 60px);
    margin-top: var(--sp-3);
    border-top: 1px solid var(--border);
  }

  .left {
    min-width: 0;
    padding: var(--sp-4) var(--sp-6) var(--sp-12);
  }

  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
    margin-bottom: var(--sp-4);
    border-bottom: 1px solid var(--border);
  }

  .tabs:focus {
    outline: none;
  }

  [role="tab"] {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: var(--sp-2) var(--sp-3);
    border: 0;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    background: none;
    color: var(--text-2);
    font-weight: var(--fw-medium);
    cursor: pointer;
    border-radius: var(--r-sm) var(--r-sm) 0 0;
  }

  [role="tab"]:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  [role="tab"][aria-selected="true"] {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
  }

  @media (max-width: 1100px) {
    .expert {
      grid-template-columns: 1fr;
    }
  }
</style>
