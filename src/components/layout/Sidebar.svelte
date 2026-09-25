<script lang="ts">
  import { pickFolder } from "../../lib/api/platform";
  import { t } from "../../lib/i18n/index.svelte";
  import { drives } from "../../lib/stores/drives.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import { ui, type View } from "../../lib/stores/ui.svelte";
  import type { DriveInfo } from "../../lib/api/types";
  import DriveCard from "../drives/DriveCard.svelte";
  import Icon from "../common/Icon.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import Logo from "./Logo.svelte";
  import NavItem from "./NavItem.svelte";
  import type { IconName } from "../common/icons";

  const busy = $derived(scan.phase === "scanning" || scan.phase === "starting");

  function isActive(d: DriveInfo): boolean {
    return ui.view === "home" && !scan.target?.isFolder && scan.target?.path === d.root;
  }

  function openDrive(d: DriveInfo) {
    // clicking the drive that already has results just goes back to them
    if (scan.target?.path === d.root && scan.phase !== "idle") {
      ui.go("home");
      return;
    }
    if (busy) {
      toasts.push("info", t("scan.busy"));
      return;
    }
    scan.chooseDrive(d);
  }

  async function scanFolder() {
    if (busy) {
      toasts.push("info", t("scan.busy"));
      return;
    }
    try {
      const path = await pickFolder();
      if (path) scan.chooseFolder(path);
    } catch (e) {
      toasts.error(e);
    }
  }

  const nav: { view: View; icon: IconName; key: string }[] = [
    { view: "history", icon: "history", key: "nav.history" },
    { view: "growth", icon: "growth", key: "nav.growth" },
    { view: "settings", icon: "settings", key: "nav.settings" },
    { view: "about", icon: "info", key: "nav.about" },
  ];
</script>

<aside class="sidebar" aria-label={t("nav.sidebar")}>
  <div class="brand">
    <Logo />
    <div>
      <p class="name">{t("app.name")}</p>
      <p class="tag">{t("app.tagline")}</p>
    </div>
  </div>

  <nav class="scroll" aria-label={t("nav.main")}>
    {#if scan.phase !== "idle"}
      <div class="group">
        <NavItem icon="monitor" label={t("nav.results")} active={ui.view === "home" || ui.view === "heuristics"} onclick={() => ui.go("home")} />
      </div>
    {/if}

    <p class="eyebrow section">{t("drives.title")}</p>
    <div class="drives">
      {#if drives.loading && drives.list.length === 0}
        {#each [0, 1] as i (i)}
          <div class="sk"><Skeleton height={54} radius="var(--r-lg)" /></div>
        {/each}
      {:else if drives.failed && drives.list.length === 0}
        <div class="fail">
          <p class="muted">{t("drives.loadFailed")}</p>
          <button class="btn btn-sm" type="button" onclick={() => drives.load()}>
            <Icon name="refresh" size={15} />{t("common.retry")}
          </button>
        </div>
      {:else if drives.list.length === 0}
        <p class="muted none">{t("drives.none")}</p>
      {:else}
        {#each drives.list as d (d.letter)}
          <DriveCard drive={d} active={isActive(d)} onclick={() => openDrive(d)} />
        {/each}
      {/if}
    </div>

    <button type="button" class="btn folder" onclick={scanFolder}>
      <Icon name="folder-search" size={17} />
      {t("drives.scanFolder")}
    </button>

    <div class="group bottom">
      {#each nav as n (n.view)}
        <NavItem icon={n.icon} label={t(n.key)} active={ui.view === n.view} onclick={() => ui.go(n.view)} />
      {/each}
    </div>
  </nav>
</aside>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-sidebar);
    border-inline-end: 1px solid var(--border);
    min-height: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-5) var(--sp-5) var(--sp-4);
  }

  .name {
    font-weight: var(--fw-bold);
    font-size: var(--fs-lg);
    line-height: 1.2;
  }

  .tag {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .scroll {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 0 var(--sp-3) var(--sp-4);
    min-height: 0;
  }

  .section {
    padding: var(--sp-4) var(--sp-3) var(--sp-2);
  }

  .drives {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .sk {
    padding: 2px 0;
  }

  .fail,
  .none {
    padding: var(--sp-3);
    font-size: var(--fs-sm);
  }

  .fail {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
  }

  .folder {
    margin: var(--sp-3) var(--sp-1) 0;
    border-style: dashed;
    background: transparent;
    color: var(--text-2);
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .bottom {
    margin-top: auto;
    padding-top: var(--sp-5);
  }
</style>
