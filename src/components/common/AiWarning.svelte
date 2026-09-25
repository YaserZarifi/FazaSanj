<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import Icon from "./Icon.svelte";
</script>

<p class="warn"><Icon name="alert" size={14} strokeWidth={2} /> <span>{t("ai.warning")}</span></p>

<style>
  .warn {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .warn :global(svg) {
    margin-top: 3px;
    color: var(--careful);
  }
</style>
