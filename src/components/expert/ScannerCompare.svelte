<script lang="ts">
  // Runs both scanners on the same drive and shows where they disagree (P3 check).
  import * as api from "../../lib/api/commands";
  import type { ScannerComparison } from "../../lib/api/types";
  import { formatDelta, formatDuration, formatInt, formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { root }: { root: string } = $props();
  let result = $state<ScannerComparison | null>(null);
  let running = $state(false);
  let error = $state<string | null>(null);

  async function run() {
    running = true;
    error = null;
    try {
      result = await api.compareScanners(root);
    } catch (e) {
      error = errorText(e);
    } finally {
      running = false;
    }
  }
</script>

<div class="compare">
  <p class="muted">{t("compare.explain")}</p>
  <div>
    <button class="btn" onclick={run} disabled={running}>
      {#if running}<Spinner size={14} />{:else}<Icon name="play" size={14} />{/if}
      {running ? t("compare.running") : t("compare.run")}
    </button>
  </div>
  {#if error}<div class="notice error">{error}</div>{/if}
  {#if result}
    <table class="list">
      <thead>
        <tr><th></th><th class="num">{t("start.fast")}</th><th class="num">{t("start.normal")}</th></tr>
      </thead>
      <tbody>
        <tr>
          <td>{t("compare.total")}</td>
          <td class="num">{formatSize(result.fastBytes, lang())}</td>
          <td class="num">{formatSize(result.normalBytes, lang())}</td>
        </tr>
        <tr>
          <td>{t("compare.files")}</td>
          <td class="num">{formatInt(result.fastFiles, lang())}</td>
          <td class="num">{formatInt(result.normalFiles, lang())}</td>
        </tr>
        <tr>
          <td>{t("compare.time")}</td>
          <td class="num">{formatDuration(result.fastMs, lang())}</td>
          <td class="num">{formatDuration(result.normalMs, lang())}</td>
        </tr>
      </tbody>
    </table>
    {#if result.differences.length === 0}
      <div class="notice ok"><Icon name="check" />{t("compare.same")}</div>
    {:else}
      <h3>{t("compare.differences")}</h3>
      <table class="list">
        <tbody>
          {#each result.differences as d (d.path)}
            <tr>
              <td><bdi class="path">{d.path}</bdi></td>
              <td class="num">{formatDelta(d.fastBytes - d.normalBytes, lang())}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</div>

<style>
  .compare {
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-width: 820px;
  }
</style>
