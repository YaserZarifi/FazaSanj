<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type { DriveInfo } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { app, type View } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  let showNetwork = $state(false);

  const visible = $derived(app.drives.filter((d) => showNetwork || d.kind !== "network"));
  const hiddenCount = $derived(app.drives.length - app.drives.filter((d) => d.kind !== "network").length);

  const isActiveRoot = (root: string) =>
    app.view.kind === "target" && app.view.root.toLowerCase() === root.toLowerCase();

  const icon = (d: DriveInfo) => (d.kind === "removable" ? "usb" : d.kind === "network" ? "network" : "drive");

  async function pickFolder() {
    const dir = await open({ directory: true, multiple: false, title: t("sidebar.pickFolder") });
    if (typeof dir === "string" && dir) app.go({ kind: "target", root: dir });
  }

  const nav: { view: View; icon: string; key: string }[] = [
    { view: { kind: "history" }, icon: "history", key: "nav.history" },
    { view: { kind: "growth" }, icon: "growth", key: "nav.growth" },
    { view: { kind: "settings" }, icon: "settings", key: "nav.settings" },
    { view: { kind: "about" }, icon: "info", key: "nav.about" },
  ];

  const folderSessions = $derived(
    Object.values(app.sessions).filter((s) => !app.drives.some((d) => d.root.toLowerCase() === s.root.toLowerCase())),
  );
</script>

<aside class="sidebar">
  <div class="brand">
    <img src="/icon.svg" alt="" width="28" height="28" />
    <span>{t("app.name")}</span>
  </div>

  <div class="section-title row">
    <span>{t("sidebar.drives")}</span>
    <span class="spacer"></span>
    <button class="btn ghost small" onclick={() => app.refreshDrives()} aria-label={t("sidebar.refresh")} title={t("sidebar.refresh")}>
      <Icon name="refresh" size={15} />
    </button>
  </div>

  <nav class="drives" aria-label={t("sidebar.drives")}>
    {#if app.drivesLoading && app.drives.length === 0}
      <div class="row faint pad"><Spinner size={14} /> {t("common.loading")}</div>
    {:else if app.drivesError}
      <div class="notice error pad">{errorText(app.drivesError)}</div>
    {/if}
    {#each visible as d (d.root)}
      {@const s = app.session(d.root)}
      <button class="drive" class:active={isActiveRoot(d.root)} onclick={() => app.go({ kind: "target", root: d.root })}>
        <Icon name={icon(d)} size={22} />
        <span class="drive-body">
          <span class="row">
            <strong>{d.label || t("sidebar.localDisk")}</strong>
            <span class="letter mono">{d.letter}</span>
            <span class="spacer"></span>
            {#if s?.status === "scanning"}<Spinner size={12} />{/if}
          </span>
          <UsageBar total={d.total} free={d.free} />
          <span class="faint">
            {t("sidebar.freeOf", { free: formatSize(d.free, lang()), total: formatSize(d.total, lang()) })} · {d.filesystem || "?"}
          </span>
        </span>
      </button>
    {/each}
    {#if hiddenCount > 0}
      <button class="btn ghost small more" onclick={() => (showNetwork = !showNetwork)}>
        {showNetwork ? t("sidebar.hideNetwork") : t("sidebar.showNetwork", { count: hiddenCount })}
      </button>
    {/if}
    {#each folderSessions as s (s.root)}
      <button class="drive folder" class:active={isActiveRoot(s.root)} onclick={() => app.go({ kind: "target", root: s.root })}>
        <Icon name="folder" size={20} />
        <span class="drive-body"><bdi class="path">{s.root}</bdi></span>
        {#if s.status === "scanning"}<Spinner size={12} />{/if}
      </button>
    {/each}
    <button class="btn ghost pick" onclick={pickFolder}>
      <Icon name="folder-open" size={16} />
      {t("sidebar.scanFolder")}
    </button>
  </nav>

  <nav class="nav" aria-label={t("sidebar.menu")}>
    {#each nav as item (item.key)}
      <button class="nav-item" class:active={app.view.kind === item.view.kind} onclick={() => app.go(item.view)}>
        <Icon name={item.icon} size={17} />
        {t(item.key)}
      </button>
    {/each}
  </nav>
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-inline-end: 1px solid var(--border);
    height: 100%;
    overflow: hidden;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--topbar-h);
    padding: 0 16px;
    font-weight: 700;
    font-size: 17px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .section-title {
    padding: 14px 16px 4px;
    font-size: 12.5px;
    color: var(--text-3);
    font-weight: 500;
  }
  .drives {
    flex: 1;
    overflow-y: auto;
    padding: 4px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pad {
    padding: 8px;
  }
  .drive {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    background: none;
    cursor: pointer;
    text-align: start;
    color: var(--text-2);
  }
  .drive:hover {
    background: var(--surface-2);
  }
  .drive.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .drive.folder {
    align-items: center;
  }
  .drive-body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--text);
  }
  .letter {
    color: var(--text-3);
  }
  .more {
    align-self: flex-start;
  }
  .pick {
    justify-content: flex-start;
    margin-top: 6px;
  }
  .nav {
    border-top: 1px solid var(--border);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex-shrink: 0;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    border: none;
    background: none;
    cursor: pointer;
    color: var(--text-2);
    text-align: start;
  }
  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
</style>
