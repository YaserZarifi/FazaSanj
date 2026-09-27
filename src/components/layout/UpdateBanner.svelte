<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { update } from "../../lib/stores/update.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";
</script>

{#if update.available && !update.dismissed}
  <div class="banner fade-in" role="status">
    <Icon name="download" size={18} />
    <p>
      {t("update.available", { v: update.available.version })}
      {#if update.failed}<span class="err">{t("update.failed")}</span>{/if}
    </p>
    <button type="button" class="btn btn-sm btn-primary" disabled={update.installing} onclick={() => update.install()}>
      {#if update.installing}<Spinner size={14} />{t("update.installing")}{:else}{t("update.install")}{/if}
    </button>
    <button
      type="button"
      class="icon-btn"
      aria-label={t("update.later")}
      title={t("update.later")}
      disabled={update.installing}
      onclick={() => (update.dismissed = true)}
    >
      <Icon name="x" size={16} />
    </button>
  </div>
{/if}

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-5);
    background: var(--accent-soft);
    border-bottom: 1px solid var(--border);
    color: var(--text);
  }

  p {
    flex: 1;
    margin: 0;
    font-size: var(--fs-sm);
  }

  .err {
    margin-inline-start: var(--sp-2);
    color: var(--danger, #d9534f);
  }
</style>
