<script lang="ts">
  import { truncateMiddle } from "../../lib/format";

  let { path, max = 60 }: { path: string; max?: number } = $props();
</script>

<span class="path pt" title={path}>{truncateMiddle(path, max)}</span>

<style>
  /* One line, never broken letter by letter; the full path is in the tooltip. */
  .pt {
    display: block;
    max-width: 100%;
    color: var(--text-2);
    font-size: var(--fs-xs);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    overflow-wrap: normal;
    text-align: start;
  }

  :global([dir="rtl"]) .pt {
    text-align: right;
  }
</style>
