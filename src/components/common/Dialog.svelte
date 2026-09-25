<script lang="ts">
  import type { Snippet } from "svelte";
  import { t } from "../../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    open: boolean;
    title: string;
    width?: number;
    tone?: "default" | "danger";
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let { open, title, width = 520, tone = "default", onclose, children, footer }: Props = $props();
  let el = $state<HTMLDialogElement>();
  const titleId = `dlg-${Math.random().toString(36).slice(2, 8)}`;

  $effect(() => {
    if (!el) return;
    if (open && !el.open) el.showModal();
    else if (!open && el.open) el.close();
  });

  function onBackdrop(e: MouseEvent) {
    if (e.target === el) onclose();
  }
</script>

<!-- the backdrop click is a mouse shortcut; Escape and the close button cover keyboard users -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
  bind:this={el}
  class={tone}
  style:--w="{width}px"
  aria-labelledby={titleId}
  oncancel={(e) => {
    e.preventDefault();
    onclose();
  }}
  onclick={onBackdrop}
>
  {#if open}
    <div class="inner">
      <header>
        <h2 id={titleId}>{title}</h2>
        <button type="button" class="icon-btn" aria-label={t("common.close")} onclick={onclose}>
          <Icon name="x" />
        </button>
      </header>
      <div class="body">{@render children()}</div>
      {#if footer}
        <footer>{@render footer()}</footer>
      {/if}
    </div>
  {/if}
</dialog>

<style>
  dialog {
    width: min(var(--w), calc(100vw - 48px));
    max-height: calc(100vh - 64px);
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--r-xl);
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }

  dialog[open] {
    animation: pop var(--dur-med) var(--ease);
  }

  dialog::backdrop {
    background: var(--overlay);
    backdrop-filter: blur(2px);
  }

  @keyframes pop {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.98);
    }
  }

  .inner {
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 64px);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    padding: var(--sp-5) var(--sp-6) var(--sp-2);
  }

  h2 {
    font-size: var(--fs-xl);
  }

  .danger h2 {
    color: var(--danger);
  }

  .body {
    padding: var(--sp-2) var(--sp-6) var(--sp-5);
    overflow: auto;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sp-2);
    padding: var(--sp-4) var(--sp-6);
    border-top: 1px solid var(--border);
    background: var(--surface-2);
  }
</style>
