<script lang="ts">
  import type { Snippet } from "svelte";

  let { label, hint, forId, children }: { label: string; hint?: string; forId?: string; children: Snippet } = $props();
</script>

<div class="row">
  <div class="text">
    {#if forId}<label for={forId}>{label}</label>{:else}<span class="lbl">{label}</span>{/if}
    {#if hint}<p class="muted hint">{hint}</p>{/if}
  </div>
  <div class="ctl">{@render children()}</div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-6);
    padding: var(--sp-3) 0;
  }

  .text {
    min-width: 0;
  }

  label,
  .lbl {
    font-weight: var(--fw-medium);
  }

  .hint {
    font-size: var(--fs-sm);
    margin-top: 2px;
  }

  .ctl {
    flex-shrink: 0;
  }
</style>
