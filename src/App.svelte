<script lang="ts">
  import { onMount } from "svelte";
  import AppShell from "./components/layout/AppShell.svelte";
  import Toasts from "./components/common/Toasts.svelte";
  import Onboarding from "./components/onboarding/Onboarding.svelte";
  import AiPreviewDialog from "./components/ai/AiPreviewDialog.svelte";
  import Splash from "./components/layout/Splash.svelte";
  import { settings } from "./lib/stores/settings.svelte";
  import { scan } from "./lib/stores/scan.svelte";
  import { cleanup } from "./lib/stores/cleanup.svelte";
  import { heuristics } from "./lib/stores/heuristics.svelte";
  import { drives } from "./lib/stores/drives.svelte";
  import { ai } from "./lib/stores/ai.svelte";
  import { ui } from "./lib/stores/ui.svelte";
  import { toasts } from "./lib/stores/toasts.svelte";
  import { update } from "./lib/stores/update.svelte";

  let ready = $state(false);

  onMount(async () => {
    await Promise.all([scan.init(), cleanup.init(), heuristics.init()]);
    try {
      await settings.load();
    } catch (e) {
      settings.useDefaults();
      toasts.error(e);
    }
    ready = true;
    void drives.load();
    void ai.loadProviders();
    // Quietly look for a new release; offline or no release just means no banner.
    setTimeout(() => void update.check(true), 4000);
  });
</script>

{#if ready}
  <AppShell />
  {#if ui.showOnboarding}
    <Onboarding />
  {/if}
  <AiPreviewDialog />
{:else}
  <Splash />
{/if}
<Toasts />
