<script lang="ts">
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import AboutScreen from "../about/AboutScreen.svelte";
  import BasketBar from "../cleanup/BasketBar.svelte";
  import CleanupScreen from "../cleanup/CleanupScreen.svelte";
  import GrowthScreen from "../growth/GrowthScreen.svelte";
  import HistoryScreen from "../history/HistoryScreen.svelte";
  import HomeView from "../scan/HomeView.svelte";
  import SettingsScreen from "../settings/SettingsScreen.svelte";
  import HeuristicsScreen from "../simple/HeuristicsScreen.svelte";
  import Sidebar from "./Sidebar.svelte";
  import TopBar from "./TopBar.svelte";
  import UpdateBanner from "./UpdateBanner.svelte";
</script>

<a class="skip" href="#main">{t("common.skipToContent")}</a>
<div class="shell" dir={i18n.dir}>
  <Sidebar />
  <div class="column">
    <TopBar />
    <UpdateBanner />
    <main id="main" class="main" tabindex="-1">
      {#key ui.view}
        <div class="view fade-in">
          {#if ui.view === "home"}
            <HomeView />
          {:else if ui.view === "heuristics"}
            <HeuristicsScreen />
          {:else if ui.view === "cleanup"}
            <CleanupScreen />
          {:else if ui.view === "history"}
            <HistoryScreen />
          {:else if ui.view === "growth"}
            <GrowthScreen />
          {:else if ui.view === "settings"}
            <SettingsScreen />
          {:else if ui.view === "about"}
            <AboutScreen />
          {/if}
        </div>
      {/key}
    </main>
    {#if cleanup.basket.length > 0 && ui.view !== "cleanup"}
      <BasketBar />
    {/if}
  </div>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
    height: 100%;
  }

  .column {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    position: relative;
  }

  .main {
    flex: 1;
    min-height: 0;
    overflow: auto;
    outline: none;
  }

  .view {
    min-height: 100%;
  }

  .skip {
    position: absolute;
    inset-inline-start: var(--sp-3);
    top: -60px;
    z-index: 200;
    padding: var(--sp-2) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--accent);
    color: var(--text-on-accent);
  }

  .skip:focus {
    top: var(--sp-3);
  }
</style>
