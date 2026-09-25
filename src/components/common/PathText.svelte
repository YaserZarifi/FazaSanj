<script lang="ts">
  import { truncateMiddle } from "../../lib/format";

  let { path, max = 60 }: { path: string; max?: number } = $props();
</script>

<span class="path pt" title={path}>{truncateMiddle(path, max)}</span>

<style>
  .pt {
    color: var(--text-2);
    font-size: var(--fs-xs);
  }
</style>
