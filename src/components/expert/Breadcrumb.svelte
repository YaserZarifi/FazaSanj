<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import Icon from "../common/Icon.svelte";

  interface Crumb {
    id: number;
    name: string;
  }

  let { items, onpick }: { items: Crumb[]; onpick: (id: number, index: number) => void } = $props();
</script>

<nav class="crumbs" aria-label={t("tree.breadcrumb")}>
  <ol>
    {#each items as c, i (c.id)}
      <li>
        {#if i > 0}<Icon name="chevron-right" size={14} flip class="sep" />{/if}
        {#if i === items.length - 1}
          <span class="cur ltr" aria-current="location">{c.name}</span>
        {:else}
          <button type="button" class="ltr" onclick={() => onpick(c.id, i)}>{c.name}</button>
        {/if}
      </li>
    {/each}
  </ol>
</nav>

<style>
  ol {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: var(--fs-sm);
  }

  li {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }

  li :global(.sep) {
    color: var(--text-3);
  }

  button {
    border: 0;
    background: none;
    padding: 2px 6px;
    border-radius: var(--r-sm);
    color: var(--accent-text);
    cursor: pointer;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  button:hover {
    background: var(--surface-hover);
  }

  .cur {
    padding: 2px 6px;
    font-weight: var(--fw-bold);
  }
</style>
