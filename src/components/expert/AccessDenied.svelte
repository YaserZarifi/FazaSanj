<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { AccessDeniedEntry } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import Skeleton from "../common/Skeleton.svelte";

  let list = $state<AccessDeniedEntry[] | null>(null);

  $effect(() => {
    const id = scan.scanId;
    if (id == null) return;
    list = null;
    backend
      .getAccessDenied(id)
      .then((l) => (list = l))
      .catch((e: unknown) => {
        list = [];
        toasts.error(e);
      });
  });
</script>

<p class="note"><Icon name="info" size={16} /> {t("denied.explain")}</p>
{#if list == null}
  <div class="sk">{#each Array(4) as _, i (i)}<Skeleton height={32} />{/each}</div>
{:else if list.length === 0}
  <EmptyState compact icon="check-circle" title={t("denied.none")} />
{:else}
  <ul>
    {#each list as d (d.path)}
      <li><Icon name="ban" size={15} /><span class="path">{d.path}</span></li>
    {/each}
  </ul>
{/if}

<style>
  .note {
    display: flex;
    gap: var(--sp-2);
    align-items: center;
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: var(--fs-sm);
    margin-bottom: var(--sp-3);
  }

  .sk {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: 8px var(--sp-3);
    border-bottom: 1px solid var(--border);
    color: var(--text-2);
  }

  li :global(svg) {
    color: var(--text-3);
  }

  .path {
    color: var(--text);
  }
</style>
