<script lang="ts">
  import type { ScanMode } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import Icon from "../common/Icon.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  const target = $derived(scan.target);
  const drive = $derived(target?.drive ?? null);
  const fastPossible = $derived(!!drive && drive.fastScanAvailable && !target?.isFolder);
  let chosen = $state<ScanMode>("fast");
  const mode = $derived<ScanMode>(fastPossible ? chosen : "normal");

  const fastReason = $derived.by(() => {
    if (target?.isFolder) return t("prescan.fastFolderOnly");
    if (drive && drive.filesystem.toUpperCase() !== "NTFS") return t("prescan.fastNotNtfs", { fs: drive.filesystem });
    return t("prescan.fastUnavailable");
  });

  const used = $derived(drive ? drive.total - drive.free : 0);
</script>

{#if target}
  <section class="wrap">
    <div class="card head">
      <span class="ic"><Icon name={target.isFolder ? "folder" : drive?.kind === "removable" ? "usb" : "drive"} size={26} /></span>
      <div class="info">
        {#if target.isFolder}
          <p class="eyebrow">{t("prescan.folder")}</p>
          <h2 class="path">{target.path}</h2>
        {:else if drive}
          <p class="eyebrow">{t("prescan.drive")}</p>
          <h2><bdi class="ltr">{drive.letter}</bdi> {drive.label || t("drives.localDisk")}</h2>
          <div class="facts">
            <span>{drive.filesystem}</span>
            <span class="num">{t("prescan.totalSize", { size: formatSize(drive.total, i18n.lang) })}</span>
            <span class="num">{t("prescan.usedSize", { size: formatSize(used, i18n.lang) })}</span>
            <span class="num">{t("prescan.freeSize", { size: formatSize(drive.free, i18n.lang) })}</span>
          </div>
          <UsageBar value={drive.total ? used / drive.total : 0} height={8} label={t("drives.usedAria", { used: formatSize(used, i18n.lang), total: formatSize(drive.total, i18n.lang) })} />
        {/if}
      </div>
    </div>

    {#if scan.wasCancelled}
      <p class="note" role="status"><Icon name="info" size={16} /> {t("prescan.cancelledNote")}</p>
    {/if}

    <h3 class="section-title">{t("prescan.howTitle")}</h3>
    <div class="choices" role="radiogroup" aria-label={t("prescan.howTitle")}>
      <button type="button" role="radio" aria-checked={mode === "fast"} class="choice" disabled={!fastPossible} onclick={() => (chosen = "fast")}>
        <span class="cic fast"><Icon name="zap" size={20} /></span>
        <span class="ctext">
          <span class="ctitle">{t("prescan.fastTitle")} <span class="pill">{t("prescan.recommended")}</span></span>
          {#if fastPossible}
            <span class="cdesc">{t("prescan.fastDesc")}</span>
            <span class="cnote"><Icon name="shield" size={14} /> {t("prescan.fastAdmin")}</span>
          {:else}
            <span class="cdesc">{fastReason}</span>
          {/if}
        </span>
        <span class="radio" aria-hidden="true"></span>
      </button>

      <button type="button" role="radio" aria-checked={mode === "normal"} class="choice" onclick={() => (chosen = "normal")}>
        <span class="cic"><Icon name="gauge" size={20} /></span>
        <span class="ctext">
          <span class="ctitle">{t("prescan.normalTitle")}</span>
          <span class="cdesc">{t("prescan.normalDesc")}</span>
        </span>
        <span class="radio" aria-hidden="true"></span>
      </button>
    </div>

    <div class="actions">
      <button type="button" class="btn btn-primary btn-lg" onclick={() => scan.start(mode)}>
        <Icon name="play" size={18} />
        {t("prescan.start")}
      </button>
      <p class="faint small">{t("prescan.readOnly")}</p>
    </div>
  </section>
{/if}

<style>
  .wrap {
    max-width: 760px;
    margin: 0 auto;
    padding: var(--sp-8) var(--sp-6);
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
  }

  .head {
    display: flex;
    gap: var(--sp-4);
    padding: var(--sp-6);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    flex-shrink: 0;
    border-radius: var(--r-lg);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  h2.path {
    font-size: var(--fs-xl);
    font-family: var(--font-ui);
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sp-1) var(--sp-4);
    color: var(--text-2);
    font-size: var(--fs-sm);
  }

  .note {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: var(--fs-sm);
  }

  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sp-3);
  }

  .choice {
    display: flex;
    align-items: flex-start;
    gap: var(--sp-3);
    padding: var(--sp-4);
    border-radius: var(--r-lg);
    border: 1.5px solid var(--border);
    background: var(--surface);
    text-align: start;
    cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
  }

  .choice:hover:not(:disabled) {
    border-color: var(--border-strong);
  }

  .choice[aria-checked="true"] {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .choice:disabled {
    cursor: not-allowed;
    background: var(--surface-2);
  }

  .choice:disabled .cic,
  .choice:disabled .ctitle {
    opacity: 0.55;
  }

  .cic {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    flex-shrink: 0;
    border-radius: var(--r-md);
    background: var(--surface-3);
    color: var(--text-2);
  }

  .cic.fast {
    background: var(--probably-safe-soft);
    color: var(--probably-safe);
  }

  .ctext {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
  }

  .ctitle {
    font-weight: var(--fw-bold);
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  .pill {
    font-size: 11px;
    font-weight: var(--fw-medium);
    padding: 0 7px;
    border-radius: var(--r-full);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .cdesc {
    font-size: var(--fs-sm);
    color: var(--text-2);
  }

  .cnote {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .radio {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
    border-radius: 50%;
    border: 2px solid var(--border-strong);
    margin-top: 2px;
  }

  .choice[aria-checked="true"] .radio {
    border: 5px solid var(--accent);
  }

  .actions {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--sp-2);
    margin-top: var(--sp-2);
  }

  .small {
    font-size: var(--fs-xs);
  }

  @media (max-width: 900px) {
    .choices {
      grid-template-columns: 1fr;
    }
  }
</style>
