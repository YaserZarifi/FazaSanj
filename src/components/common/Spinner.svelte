<script lang="ts">
  let { size = 18, label }: { size?: number; label?: string } = $props();
</script>

<span class="spin" style:width="{size}px" style:height="{size}px" role={label ? "status" : undefined} aria-label={label}></span>

<style>
  .spin {
    display: inline-block;
    flex-shrink: 0;
    border-radius: 50%;
    border: 2px solid color-mix(in srgb, currentColor 25%, transparent);
    border-top-color: currentColor;
    animation: rot 0.8s linear infinite;
  }

  @keyframes rot {
    to {
      transform: rotate(360deg);
    }
  }
</style>
