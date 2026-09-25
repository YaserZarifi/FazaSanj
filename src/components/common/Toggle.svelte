<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    description?: string;
    disabled?: boolean;
    onchange: (v: boolean) => void;
  }

  let { checked, label, description, disabled = false, onchange }: Props = $props();
  const id = `tg-${Math.random().toString(36).slice(2, 8)}`;
</script>

<div class="row">
  <div class="text">
    <label for={id}>{label}</label>
    {#if description}<p class="muted desc" id="{id}-d">{description}</p>{/if}
  </div>
  <button
    {id}
    type="button"
    role="switch"
    class="switch"
    aria-checked={checked}
    aria-label={label}
    aria-describedby={description ? `${id}-d` : undefined}
    {disabled}
    onclick={() => onchange(!checked)}
  >
    <span class="knob"></span>
  </button>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-4);
  }

  label {
    font-weight: var(--fw-medium);
  }

  .desc {
    font-size: var(--fs-sm);
    margin-top: 2px;
  }

  .switch {
    position: relative;
    flex-shrink: 0;
    width: 40px;
    height: 22px;
    padding: 0;
    border-radius: var(--r-full);
    border: 1px solid var(--border-strong);
    background: var(--surface-3);
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), border-color var(--dur-fast) var(--ease);
  }

  .switch[aria-checked="true"] {
    background: var(--accent);
    border-color: var(--accent);
  }

  .switch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .knob {
    position: absolute;
    top: 2px;
    inset-inline-start: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    box-shadow: var(--shadow-sm);
    transition: inset-inline-start var(--dur-med) var(--ease);
  }

  .switch[aria-checked="true"] .knob {
    inset-inline-start: 20px;
  }
</style>
