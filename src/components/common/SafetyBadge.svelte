<script lang="ts">
  import type { SafetyLevel } from "../../lib/api/types";
  import { safetyName } from "../../lib/i18n/index.svelte";
  import { SAFETY_META } from "../../lib/theme";
  import Icon from "./Icon.svelte";
  import type { IconName } from "./icons";

  interface Props {
    level: SafetyLevel;
    compact?: boolean;
  }

  let { level, compact = false }: Props = $props();
  const meta = $derived(SAFETY_META[level]);
</script>

<span
  class="badge"
  class:compact
  style:--c={meta.color}
  style:--bg={meta.soft}
  title={compact ? safetyName(level) : undefined}
>
  <Icon name={meta.icon as IconName} size={compact ? 14 : 15} strokeWidth={2} />
  {#if compact}
    <span class="sr-only">{safetyName(level)}</span>
  {:else}
    <span>{safetyName(level)}</span>
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 9px 2px 8px;
    border-radius: var(--r-full);
    background: var(--bg);
    color: var(--c);
    font-size: var(--fs-xs);
    font-weight: var(--fw-medium);
    line-height: 1.6;
    white-space: nowrap;
  }

  .compact {
    padding: 3px;
  }
</style>
