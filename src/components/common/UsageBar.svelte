<script lang="ts">
  import { usage } from "../../lib/labels";

  let { total, free, height = 6 }: { total: number; free: number; height?: number } = $props();
  const u = $derived(usage(total, free));
</script>

<div class="track" style:height="{height}px" role="presentation">
  <div class="fill {u.level}" style:width="{u.share * 100}%"></div>
</div>

<style>
  .track {
    width: 100%;
    background: var(--bar-track);
    border-radius: 99px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 99px;
    background: var(--bar-used);
    transition: width 0.3s;
  }
  .fill.warn {
    background: var(--bar-warn);
  }
  .fill.full {
    background: var(--bar-full);
  }
</style>
