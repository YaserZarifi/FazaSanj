<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { ScanId, TypeGroup } from "../../lib/api/types";
  import { formatInt, formatPercent, formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import ShareBar from "../common/ShareBar.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { scanId }: { scanId: ScanId } = $props();
  let groups = $state<TypeGroup[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const s = scanId;
    api.getByType(s).then((g) => (groups = g)).catch((e) => (error = errorText(e)));
  });

  const total = $derived(groups ? groups.reduce((a, g) => a + g.bytes, 0) : 0);
</script>

{#if error}
  <div class="notice error">{error}</div>
{:else if !groups}
  <div class="center"><Spinner /></div>
{:else}
  <div class="groups">
    {#each groups as g (g.group)}
      <div class="card group">
        <div class="row">
          <strong>{t(`types.${g.group}`)}</strong>
          <span class="spacer"></span>
          <strong>{formatSize(g.bytes, lang())}</strong>
        </div>
        <div class="row">
          <ShareBar share={total ? g.bytes / total : 0} />
          <span class="faint pct">{formatPercent(total ? g.bytes / total : 0, lang())}</span>
        </div>
        <span class="faint">{t("types.files", { count: formatInt(g.files, lang()) })}</span>
        {#if g.topExtensions.length}
          <div class="exts">
            {#each g.topExtensions.slice(0, 6) as x (x.ext)}
              <span class="ext"><bdi class="mono">.{x.ext}</bdi> {formatSize(x.bytes, lang())}</span>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .center {
    display: grid;
    place-items: center;
    padding: 16px;
  }
  .groups {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 12px;
  }
  .group {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .pct {
    min-width: 44px;
  }
  .exts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .ext {
    font-size: 12px;
    padding: 1px 8px;
    border-radius: 99px;
    background: var(--surface-2);
  }
</style>
