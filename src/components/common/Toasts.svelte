<script lang="ts">
  import { app } from "../../lib/stores/app.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as toast (toast.id)}
    <div class="toast {toast.kind}">{toast.text}</div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    bottom: 20px;
    inset-inline-end: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 60;
    max-width: 380px;
  }
  .toast {
    padding: 10px 14px;
    border-radius: var(--radius);
    background: var(--text);
    color: var(--bg);
    box-shadow: var(--shadow-lg);
  }
  .toast.ok {
    background: var(--safe);
    color: #fff;
  }
  .toast.error {
    background: var(--danger);
    color: #fff;
  }
</style>
