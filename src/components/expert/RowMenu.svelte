<script lang="ts">
  import { onMount } from "svelte";
  import type { NodeInfo } from "../../lib/api/types";
  import { backend } from "../../lib/api/client";
  import { t } from "../../lib/i18n/index.svelte";
  import { ai } from "../../lib/stores/ai.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import Icon from "../common/Icon.svelte";
  import type { IconName } from "../common/icons";

  interface Props {
    node: NodeInfo;
    x: number;
    y: number;
    onclose: () => void;
  }

  let { node, x, y, onclose }: Props = $props();
  let el = $state<HTMLDivElement>();

  interface Item {
    icon: IconName;
    label: string;
    run: () => void;
  }

  const items = $derived.by(() => {
    const out: Item[] = [];
    if (node.isDir && node.childCount > 0) out.push({ icon: "folder-open", label: t("tree.open"), run: () => scan.enter(node) });
    out.push({
      icon: "external",
      label: t("tree.reveal"),
      run: () => void backend.revealInExplorer(node.path).catch((e: unknown) => toasts.error(e)),
    });
    const e = node.explanation;
    if (e && e.safety !== "do_not_touch" && e.method !== "manual_only") {
      const inBasket = cleanup.has(node.path);
      out.push({
        icon: inBasket ? "minus" : "plus",
        label: inBasket ? t("tree.removeFromCleanup") : t("reason.addToCleanup"),
        run: () => cleanup.toggle({ path: node.path, bytes: node.size, explanation: e }),
      });
    }
    if (!e && ai.enabled) out.push({ icon: "sparkles", label: t("ai.ask"), run: () => void ai.ask(node) });
    out.push({ icon: "info", label: t("tree.details"), run: () => (scan.selected = node) });
    return out;
  });

  onMount(() => {
    el?.querySelector<HTMLButtonElement>("button")?.focus();
    const away = (ev: MouseEvent) => {
      if (el && !el.contains(ev.target as Node)) onclose();
    };
    setTimeout(() => window.addEventListener("mousedown", away));
    return () => window.removeEventListener("mousedown", away);
  });

  function onkeydown(e: KeyboardEvent) {
    const buttons = [...(el?.querySelectorAll<HTMLButtonElement>("button") ?? [])];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape") {
      e.preventDefault();
      onclose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      buttons[(i + 1) % buttons.length]?.focus();
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      buttons[(i - 1 + buttons.length) % buttons.length]?.focus();
    } else if (e.key === "Tab") {
      onclose();
    }
  }

  const left = $derived(Math.min(x, window.innerWidth - 240));
  const top = $derived(Math.min(y, window.innerHeight - items.length * 38 - 20));
</script>

<div class="menu" role="menu" tabindex="-1" bind:this={el} style:left="{left}px" style:top="{top}px" {onkeydown}>
  {#each items as it (it.label)}
    <button
      type="button"
      role="menuitem"
      onclick={() => {
        it.run();
        onclose();
      }}
    >
      <Icon name={it.icon} size={16} />{it.label}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 50;
    min-width: 220px;
    padding: var(--sp-1);
    border-radius: var(--r-md);
    border: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow-lg);
    animation: fade-in var(--dur-fast) var(--ease);
  }

  button {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    width: 100%;
    padding: 7px var(--sp-3);
    border: 0;
    border-radius: var(--r-sm);
    background: none;
    text-align: start;
    font-size: var(--fs-sm);
    cursor: pointer;
  }

  button:hover,
  button:focus-visible {
    background: var(--surface-hover);
    outline: none;
  }
</style>
