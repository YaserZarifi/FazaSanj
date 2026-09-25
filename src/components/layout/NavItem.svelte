<script lang="ts">
  import Icon from "../common/Icon.svelte";
  import type { IconName } from "../common/icons";

  interface Props {
    icon: IconName;
    label: string;
    active: boolean;
    badge?: string;
    onclick: () => void;
  }

  let { icon, label, active, badge, onclick }: Props = $props();
</script>

<button type="button" class="nav" class:active aria-current={active ? "page" : undefined} {onclick}>
  <Icon name={icon} size={18} />
  <span class="label">{label}</span>
  {#if badge}<span class="badge num">{badge}</span>{/if}
</button>

<style>
  .nav {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    width: 100%;
    min-height: 38px;
    padding-inline: var(--sp-3);
    border: 0;
    border-radius: var(--r-md);
    background: transparent;
    color: var(--text-2);
    font-weight: var(--fw-medium);
    text-align: start;
    cursor: pointer;
    position: relative;
    transition: background var(--dur-fast) var(--ease), color var(--dur-fast) var(--ease);
  }

  .nav:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .active {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }

  .active::before {
    content: "";
    position: absolute;
    inset-inline-start: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: var(--r-full);
    background: var(--accent);
  }

  .label {
    flex: 1;
  }

  .badge {
    font-size: var(--fs-xs);
    padding: 0 7px;
    border-radius: var(--r-full);
    background: var(--accent-soft);
    color: var(--accent-text);
  }
</style>
