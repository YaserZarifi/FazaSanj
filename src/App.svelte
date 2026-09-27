<script lang="ts">
  import Toasts from "./components/common/Toasts.svelte";
  import Spinner from "./components/common/Spinner.svelte";
  import CleanupDialog from "./components/cleanup/CleanupDialog.svelte";
  import History from "./components/cleanup/History.svelte";
  import Growth from "./components/growth/Growth.svelte";
  import Home from "./components/layout/Home.svelte";
  import Sidebar from "./components/layout/Sidebar.svelte";
  import TopBar from "./components/layout/TopBar.svelte";
  import Onboarding from "./components/onboarding/Onboarding.svelte";
  import TargetView from "./components/scan/TargetView.svelte";
  import About from "./components/settings/About.svelte";
  import Settings from "./components/settings/Settings.svelte";
  import { ltr } from "./lib/format";
  import { t } from "./lib/i18n/index.svelte";
  import { app } from "./lib/stores/app.svelte";
  import { checkForUpdate } from "./lib/updater.svelte";

  $effect(() => {
    app.init().then(() => {
      // Quietly look for a new version a little after start.
      setTimeout(() => checkForUpdate(true), 8000);
    });
  });

  const title = $derived.by(() => {
    const v = app.view;
    switch (v.kind) {
      case "target":
        return ltr(v.root);
      case "home":
        return t("app.name");
      default:
        return t(`nav.${v.kind}`);
    }
  });
</script>

{#if !app.ready}
  <div class="boot"><Spinner size={28} /></div>
{:else if !app.settings.onboardingDone}
  <Onboarding />
{:else}
  <div class="shell">
    <Sidebar />
    <div class="main">
      <TopBar {title} />
      <main class="content">
        {#if app.view.kind === "target"}
          {#key app.view.root.toLowerCase()}
            <TargetView root={app.view.root} />
          {/key}
        {:else if app.view.kind === "history"}
          <History />
        {:else if app.view.kind === "growth"}
          <Growth />
        {:else if app.view.kind === "settings"}
          <Settings />
        {:else if app.view.kind === "about"}
          <About />
        {:else}
          <Home />
        {/if}
      </main>
    </div>
  </div>
  {#if app.cleanup}
    <CleanupDialog />
  {/if}
{/if}
<Toasts />

<style>
  .boot {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .shell {
    display: flex;
    height: 100%;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
