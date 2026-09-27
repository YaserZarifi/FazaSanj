<script lang="ts">
  // One drive or folder: start a scan, watch it, then show the result.
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import ExpertView from "../expert/ExpertView.svelte";
  import StoryView from "../simple/StoryView.svelte";
  import ScanProgressView from "./ScanProgressView.svelte";
  import ScanStart from "./ScanStart.svelte";
  import ScanSummaryBar from "./ScanSummaryBar.svelte";

  let { root }: { root: string } = $props();

  const session = $derived(app.session(root));
  const drive = $derived(app.drives.find((d) => d.root.toLowerCase() === root.toLowerCase()) ?? null);
</script>

<div class="target">
  {#if !session || session.status === "cancelled" || session.status === "error"}
    {#if session?.status === "cancelled"}
      <div class="notice info banner"><Icon name="info" />{t("scan.cancelled")}</div>
    {:else if session?.status === "error"}
      <div class="notice error banner"><Icon name="alert" />{errorText(session.error)}</div>
    {/if}
    <ScanStart {root} {drive} />
  {:else if session.status === "scanning"}
    <ScanProgressView {session} />
  {:else if session.summary}
    <ScanSummaryBar {session} {drive} />
    <div class="result">
      {#if app.mode === "simple"}
        <StoryView summary={session.summary} />
      {:else}
        <ExpertView summary={session.summary} />
      {/if}
    </div>
  {/if}
</div>

<style>
  .target {
    display: flex;
    flex-direction: column;
    min-height: 100%;
  }
  .banner {
    margin: 16px 24px 0;
  }
  .result {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
