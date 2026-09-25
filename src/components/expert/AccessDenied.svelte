<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { AccessDeniedEntry, ScanId } from "../../lib/api/types";
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { scanId }: { scanId: ScanId } = $props();
  let list = $state<AccessDeniedEntry[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const s = scanId;
    api.getAccessDenied(s).then((l) => (list = l)).catch((e) => (error = errorText(e)));
  });
</script>

<div class="denied">
  <div class="notice info">{t("denied.explain")}</div>
  {#if error}
    <div class="notice error">{error}</div>
  {:else if !list}
    <Spinner />
  {:else}
    <ul>
      {#each list as d (d.path)}
        <li><bdi class="path">{d.path}</bdi></li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .denied {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  ul {
    margin: 0;
    padding-inline-start: 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
