<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import CleanupProgressView from "./CleanupProgressView.svelte";
  import CleanupResult from "./CleanupResult.svelte";
  import ReviewScreen from "./ReviewScreen.svelte";
</script>

{#if cleanup.stage === "running"}
  <CleanupProgressView />
{:else if cleanup.stage === "result"}
  <CleanupResult />
{:else if cleanup.plan || cleanup.planLoading || cleanup.planError}
  <ReviewScreen />
{:else}
  <EmptyState icon="basket" title={t("cleanup.emptyTitle")} text={t("cleanup.emptyText")}>
    <button type="button" class="btn" onclick={() => ui.go("home")}>{t("common.back")}</button>
  </EmptyState>
{/if}
