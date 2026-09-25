<script lang="ts">
  // A thin bar for "this item is X% of its parent".
  let { share, color = "var(--accent)" }: { share: number; color?: string } = $props();
</script>

<div class="track" role="presentation">
  <div class="fill" style:width="{Math.min(100, Math.max(0, share * 100))}%" style:background={color}></div>
</div>

<style>
  .track {
    height: 6px;
    width: 100%;
    min-width: 60px;
    background: var(--bar-track);
    border-radius: 99px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    border-radius: 99px;
  }
</style>
