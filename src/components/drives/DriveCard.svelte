<script lang="ts">
  import type { DriveInfo } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import Icon from "../common/Icon.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  interface Props {
    drive: DriveInfo;
    active: boolean;
    onclick: () => void;
  }

  let { drive, active, onclick }: Props = $props();

  const used = $derived(drive.total - drive.free);
  const ratio = $derived(drive.total > 0 ? used / drive.total : 0);
  const low = $derived(drive.free < (settings.value?.lowSpaceThresholdGb ?? 10) * 1024 ** 3);
  const name = $derived(drive.label || t(drive.kind === "removable" ? "drives.removable" : "drives.localDisk"));
</script>

<button type="button" class="drive" class:active aria-current={active ? "true" : undefined} {onclick}>
  <span class="ic" class:low>
    <Icon name={drive.kind === "removable" ? "usb" : "drive"} size={20} />
  </span>
  <span class="body">
    <span class="top">
      <span class="name"><bdi class="ltr">{drive.letter}</bdi> {name}</span>
      <span class="fs">{drive.filesystem}</span>
    </span>
    <UsageBar
      value={ratio}
      tone={low ? "danger" : ratio > 0.85 ? "warn" : "accent"}
      label={t("drives.usedAria", { used: formatSize(used, i18n.lang), total: formatSize(drive.total, i18n.lang) })}
    />
    <span class="free num" class:lowtext={low}>
      {t("drives.freeOf", { free: formatSize(drive.free, i18n.lang), total: formatSize(drive.total, i18n.lang) })}
    </span>
  </span>
</button>

<style>
  .drive {
    display: flex;
    gap: var(--sp-3);
    width: 100%;
    padding: var(--sp-3);
    border-radius: var(--r-lg);
    border: 1px solid transparent;
    background: transparent;
    text-align: start;
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }

  .drive:hover {
    background: var(--surface-hover);
  }

  .active {
    background: var(--surface);
    border-color: var(--border);
    box-shadow: var(--shadow-sm);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: var(--r-md);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .ic.low {
    background: var(--danger-soft);
    color: var(--danger);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }

  .top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--sp-2);
  }

  .name {
    font-weight: var(--fw-bold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .fs {
    font-size: 11px;
    color: var(--text-3);
    padding: 0 6px;
    border-radius: var(--r-xs);
    background: var(--surface-3);
  }

  .free {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .lowtext {
    color: var(--danger);
  }
</style>
