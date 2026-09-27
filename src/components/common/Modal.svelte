<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  let {
    title,
    onclose,
    width = 640,
    closable = true,
    children,
    footer,
  }: {
    title: string;
    onclose: () => void;
    width?: number;
    closable?: boolean;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  let dialog: HTMLDivElement | undefined = $state();

  $effect(() => {
    const prev = document.activeElement as HTMLElement | null;
    dialog?.focus();
    return () => prev?.focus?.();
  });

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && closable) {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="overlay" onkeydown={onkey}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    bind:this={dialog}
    style:max-width="{width}px"
  >
    <header>
      <h2>{title}</h2>
      {#if closable}
        <button class="btn ghost small" onclick={onclose} aria-label={t("common.close")}>
          <Icon name="x" />
        </button>
      {/if}
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay);
    display: grid;
    place-items: center;
    z-index: 50;
    padding: 24px;
  }
  .dialog {
    width: 100%;
    max-height: calc(100vh - 48px);
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-lg);
    outline: none;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 16px 20px 12px;
    border-bottom: 1px solid var(--border);
  }
  .body {
    padding: 16px 20px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 20px 16px;
    border-top: 1px solid var(--border);
    flex-wrap: wrap;
  }
</style>
