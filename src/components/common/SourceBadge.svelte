<script lang="ts">
  import type { ExplanationSource } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let { source }: { source: ExplanationSource } = $props();
</script>

<span class="src {source}">
  {#if source === "ai"}
    <Icon name="sparkles" size={13} strokeWidth={2} />
  {:else if source === "heuristic"}
    <Icon name="search" size={13} strokeWidth={2} />
  {:else}
    <Icon name="shield-check" size={13} strokeWidth={2} />
  {/if}
  {t(`source.${source}`)}
</span>

<style>
  .src {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 1px 8px;
    border-radius: var(--r-full);
    font-size: var(--fs-xs);
    font-weight: var(--fw-medium);
    border: 1px solid var(--border);
    color: var(--text-2);
    background: var(--surface-2);
    white-space: nowrap;
  }

  .ai {
    color: var(--ai);
    background: var(--ai-soft);
    border-color: transparent;
  }

  .knowledge_base {
    color: var(--accent-text);
    background: var(--accent-soft);
    border-color: transparent;
  }
</style>
