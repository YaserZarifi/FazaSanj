<script lang="ts" generics="T extends string">
  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";

  interface Option {
    value: T;
    label: string;
    icon?: IconName;
  }

  interface Props {
    options: Option[];
    value: T;
    label: string;
    iconOnly?: boolean;
    onchange: (v: T) => void;
  }

  let { options, value, label, iconOnly = false, onchange }: Props = $props();

  function onkeydown(e: KeyboardEvent) {
    const i = options.findIndex((o) => o.value === value);
    const rtl = document.documentElement.dir === "rtl";
    let next: number;
    if (e.key === "ArrowRight") next = rtl ? i - 1 : i + 1;
    else if (e.key === "ArrowLeft") next = rtl ? i + 1 : i - 1;
    else return;
    e.preventDefault();
    next = (next + options.length) % options.length;
    onchange(options[next].value);
    const group = e.currentTarget as HTMLElement;
    requestAnimationFrame(() => group.querySelector<HTMLElement>("[aria-checked=true]")?.focus());
  }
</script>

<div class="seg" role="radiogroup" aria-label={label} tabindex="-1" {onkeydown}>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={o.value === value}
      tabindex={o.value === value ? 0 : -1}
      title={iconOnly ? o.label : undefined}
      aria-label={iconOnly ? o.label : undefined}
      onclick={() => onchange(o.value)}
    >
      {#if o.icon}<Icon name={o.icon} size={16} />{/if}
      {#if !iconOnly}<span>{o.label}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    border-radius: var(--r-md);
    background: var(--surface-3);
    border: 1px solid var(--border);
  }

  .seg:focus {
    outline: none;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    padding-inline: var(--sp-3);
    border: 0;
    border-radius: 7px;
    background: transparent;
    color: var(--text-2);
    font-size: var(--fs-sm);
    font-weight: var(--fw-medium);
    cursor: pointer;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }

  button:hover {
    color: var(--text);
  }

  button[aria-checked="true"] {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
</style>
