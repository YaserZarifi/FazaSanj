<script lang="ts">
  import { formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import Icon from "../common/Icon.svelte";
</script>

<div class="basket" role="region" aria-label={t("basket.label")}>
  <span class="ic"><Icon name="basket" size={18} /></span>
  <p>
    <b>{t("basket.count", { count: cleanup.basket.length })}</b>
    <span class="num muted">{formatSize(cleanup.basketBytes, i18n.lang)}</span>
  </p>
  <button type="button" class="btn btn-sm btn-ghost" onclick={() => cleanup.clear()}>{t("basket.clear")}</button>
  <button type="button" class="btn btn-sm btn-primary" onclick={() => cleanup.review()}>
    {t("basket.review")}<Icon name="arrow-right" size={15} flip />
  </button>
</div>

<style>
  .basket {
    position: absolute;
    inset-block-end: var(--sp-5);
    inset-inline-start: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: var(--sp-3);
    padding: var(--sp-2) var(--sp-2) var(--sp-2) var(--sp-3);
    padding-inline: var(--sp-3) var(--sp-2);
    border-radius: var(--r-full);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    animation: fade-in var(--dur-med) var(--ease);
    z-index: 20;
  }

  :global([dir="rtl"]) .basket {
    transform: translateX(50%);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  p {
    display: flex;
    gap: var(--sp-2);
    align-items: baseline;
    white-space: nowrap;
  }
</style>
