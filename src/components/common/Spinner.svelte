<script lang="ts">
  let { size = 18 }: { size?: number } = $props();
</script>

<span class="spinner" style:width="{size}px" style:height="{size}px" role="presentation"></span>

<style>
  .spinner {
    display: inline-block;
    border-radius: 50%;
    border: 2px solid var(--surface-3);
    border-top-color: var(--accent);
    animation: spin 0.8s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
