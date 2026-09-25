<script lang="ts">
  import { scan } from "../../lib/stores/scan.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import ExpertView from "../expert/ExpertView.svelte";
  import StoryView from "../simple/StoryView.svelte";
  import EmptyHome from "./EmptyHome.svelte";
  import PreScanPanel from "./PreScanPanel.svelte";
  import ScanDoneBanner from "./ScanDoneBanner.svelte";
  import ScanError from "./ScanError.svelte";
  import ScanningView from "./ScanningView.svelte";
</script>

{#if scan.phase === "idle"}
  <EmptyHome />
{:else if scan.phase === "prescan"}
  <PreScanPanel />
{:else if scan.phase === "starting" || scan.phase === "scanning"}
  <ScanningView />
{:else if scan.phase === "error"}
  <ScanError />
{:else if scan.phase === "done"}
  <ScanDoneBanner />
  {#if ui.mode === "simple"}
    <StoryView />
  {:else}
    <ExpertView />
  {/if}
{/if}
