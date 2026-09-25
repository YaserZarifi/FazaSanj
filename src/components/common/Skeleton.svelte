<script lang="ts">
  let { height = 16, width = "100%", radius = "var(--r-sm)" }: { height?: number; width?: string; radius?: string } = $props();
</script>

<span class="sk" style:height="{height}px" style:width style:border-radius={radius} aria-hidden="true"></span>

<style>
  .sk {
    display: block;
    background: linear-gradient(90deg, var(--surface-3) 0%, var(--surface-2) 50%, var(--surface-3) 100%);
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }
</style>
