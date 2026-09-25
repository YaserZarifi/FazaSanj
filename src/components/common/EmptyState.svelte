<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";

  interface Props {
    icon?: IconName;
    title: string;
    text?: string;
    compact?: boolean;
    children?: Snippet;
  }

  let { icon = "folder", title, text, compact = false, children }: Props = $props();
</script>

<div class="empty" class:compact>
  <span class="ic"><Icon name={icon} size={compact ? 22 : 28} /></span>
  <p class="title">{title}</p>
  {#if text}<p class="muted text">{text}</p>{/if}
  {#if children}<div class="actions">{@render children()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--sp-2);
    padding: var(--sp-10) var(--sp-6);
  }

  .compact {
    padding: var(--sp-6) var(--sp-4);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--r-lg);
    background: var(--surface-3);
    color: var(--text-3);
    margin-bottom: var(--sp-2);
  }

  .compact .ic {
    width: 44px;
    height: 44px;
  }

  .title {
    font-weight: var(--fw-bold);
  }

  .text {
    max-width: 420px;
    font-size: var(--fs-sm);
  }

  .actions {
    margin-top: var(--sp-3);
  }
</style>
