<script lang="ts">
  import type { Category } from "../../lib/api/types";
  import { CATEGORY_ICON } from "./categoryIcons";
  import Icon from "./Icon.svelte";

  let { category, size = 36 }: { category: Category; size?: number } = $props();
</script>

<span class="cat" style:--c="var(--cat-{category})" style:width="{size}px" style:height="{size}px">
  <Icon name={CATEGORY_ICON[category]} size={Math.round(size * 0.5)} />
</span>

<style>
  .cat {
    display: inline-grid;
    place-items: center;
    flex-shrink: 0;
    border-radius: var(--r-md);
    color: var(--c);
    background: color-mix(in srgb, var(--c) 14%, transparent);
  }
</style>
