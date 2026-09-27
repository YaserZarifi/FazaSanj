<script lang="ts">
  import type { SafetyLevel } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { safetyKey, safetyVar } from "../../lib/labels";

  let { level, small = false }: { level: SafetyLevel; small?: boolean } = $props();
  const v = $derived(safetyVar(level));
</script>

<span class="badge" class:small style:--c="var({v})" style:--bg="var({v}-soft)">
  <span class="dot"></span>{t(safetyKey(level))}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 10px;
    border-radius: 99px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--c);
    background: var(--bg);
    white-space: nowrap;
  }
  .badge.small {
    padding: 0 8px;
    font-size: 12px;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--c);
  }
</style>
