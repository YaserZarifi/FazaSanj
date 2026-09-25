<script lang="ts">
  import type { DriveInfo, ScanMode } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  let { root, drive }: { root: string; drive: DriveInfo | null } = $props();

  const fastPossible = $derived(drive?.fastScanAvailable ?? false);
  let mode = $state<ScanMode>("normal");
  let starting = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    mode = fastPossible ? "fast" : "normal";
  });

  async function start() {
    starting = true;
    error = null;
    try {
      await app.startScan(root, mode);
    } catch (e) {
      error = errorText(e);
    } finally {
      starting = false;
    }
  }
</script>

<section class="start">
  <div class="card head">
    <Icon name={drive ? "drive" : "folder"} size={36} />
    <div class="info">
      {#if drive}
        <h2>{drive.label || t("sidebar.localDisk")} <span class="mono faint">{drive.letter}</span></h2>
        <UsageBar total={drive.total} free={drive.free} height={8} />
        <p class="muted">
          {t("start.driveLine", {
            used: formatSize(drive.total - drive.free, lang()),
            total: formatSize(drive.total, lang()),
            free: formatSize(drive.free, lang()),
          })}
          · {drive.filesystem}
        </p>
      {:else}
        <h2>{t("start.folder")}</h2>
        <bdi class="path">{root}</bdi>
      {/if}
    </div>
  </div>

  <fieldset class="modes">
    <legend>{t("start.howTitle")}</legend>
    <label class="card mode" class:on={mode === "fast"} class:disabled={!fastPossible}>
      <input type="radio" name="mode" value="fast" bind:group={mode} disabled={!fastPossible} />
      <Icon name="zap" size={22} />
      <span>
        <strong>{t("start.fast")}</strong>
        <span class="muted">{fastPossible ? t("start.fastHint") : t("start.fastUnavailable")}</span>
      </span>
    </label>
    <label class="card mode" class:on={mode === "normal"}>
      <input type="radio" name="mode" value="normal" bind:group={mode} />
      <Icon name="folder" size={22} />
      <span>
        <strong>{t("start.normal")}</strong>
        <span class="muted">{t("start.normalHint")}</span>
      </span>
    </label>
  </fieldset>

  {#if error}<div class="notice error">{error}</div>{/if}

  <div>
    <button class="btn primary big" onclick={start} disabled={starting}>
      <Icon name="play" size={18} />
      {t("start.button")}
    </button>
  </div>
  <p class="faint">{t("start.readOnly")}</p>
</section>

<style>
  .start {
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
    padding: 32px 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .head {
    display: flex;
    gap: 16px;
    padding: 20px;
    align-items: flex-start;
    color: var(--accent);
  }
  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    color: var(--text);
  }
  .modes {
    border: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  legend {
    font-weight: 700;
    margin-bottom: 10px;
    padding: 0;
  }
  .mode {
    display: flex;
    gap: 12px;
    padding: 14px;
    cursor: pointer;
    align-items: flex-start;
  }
  .mode span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .mode.on {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .mode.disabled {
    opacity: 0.6;
    cursor: default;
  }
  .mode input {
    margin-top: 4px;
  }
</style>
