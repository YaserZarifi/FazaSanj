<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="stack" aria-live="polite" aria-relevant="additions">
  {#each toasts.list as toast (toast.id)}
    <div class="toast {toast.kind}" role={toast.kind === "error" ? "alert" : "status"}>
      <Icon name={toast.kind === "error" ? "alert" : toast.kind === "success" ? "check-circle" : "info"} size={18} />
      <p>{toast.text}</p>
      <button type="button" class="icon-btn" aria-label={t("common.dismiss")} onclick={() => toasts.dismiss(toast.id)}>
        <Icon name="x" size={16} />
      </button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    inset-block-end: var(--sp-5);
    inset-inline-end: var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    z-index: 100;
    max-width: 420px;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding-block: var(--sp-2);
    padding-inline: var(--sp-4) var(--sp-2);
    border-radius: var(--r-lg);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    animation: fade-in var(--dur-med) var(--ease);
  }

  p {
    flex: 1;
    font-size: var(--fs-sm);
    color: var(--text);
  }

  .error {
    color: var(--danger);
  }

  .success {
    color: var(--safe);
  }

  .info {
    color: var(--accent);
  }
</style>
