<script lang="ts">
  import { update } from "../../lib/stores/update.svelte";
  import { openExternal } from "../../lib/api/platform";
  import { localizeDigits } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import Icon from "../common/Icon.svelte";
  import Logo from "../layout/Logo.svelte";

  const REPO = "https://github.com/YaserZarifi/FazaSanj";
  const RULE_ISSUE = "https://github.com/YaserZarifi/FazaSanj/issues/new?labels=rule&title=Wrong%20rule%3A%20";

  function open(url: string) {
    openExternal(url).catch((e: unknown) => toasts.error(e));
  }
</script>

<div class="page">
  <div class="card hero">
    <Logo size={64} />
    <h2>{t("app.name")}</h2>
    {#if settings.info}
      <p class="ver num">{t("about.version", { v: localizeDigits(settings.info.version, i18n.lang) })}</p>
    {/if}
    <p class="desc">{t("about.description")}</p>
    <div class="links">
      <button type="button" class="btn" onclick={() => open(REPO)}><Icon name="code" size={16} />{t("about.source")}</button>
      <button type="button" class="btn" onclick={() => open(RULE_ISSUE)}><Icon name="flag" size={16} />{t("about.reportRule")}</button>
      <button type="button" class="btn" disabled={update.checking} onclick={() => update.check()}>
        <Icon name="download" size={16} />{update.checking ? t("update.checking") : t("update.check")}
      </button>
    </div>
    {#if update.lastChecked && !update.checking}
      <p class="muted">
        {#if update.available}{t("update.available", { v: update.available.version })}
        {:else if update.failed}{t("update.checkFailed")}
        {:else}{t("update.upToDate")}{/if}
      </p>
    {/if}
  </div>

  <div class="card facts">
    <div><p class="eyebrow">{t("about.license")}</p><p>MIT</p></div>
    <div><p class="eyebrow">{t("about.privacy")}</p><p>{t("about.privacyText")}</p></div>
    <div><p class="eyebrow">{t("about.offline")}</p><p>{t("about.offlineText")}</p></div>
  </div>
</div>

<style>
  .page {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--sp-10) var(--sp-6);
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--sp-3);
    padding: var(--sp-10) var(--sp-8);
  }

  h2 {
    font-size: var(--fs-3xl);
  }

  .ver {
    color: var(--text-3);
    font-size: var(--fs-sm);
  }

  .desc {
    max-width: 480px;
    color: var(--text-2);
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--sp-2);
    margin-top: var(--sp-3);
  }

  .facts {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--sp-5);
    padding: var(--sp-5) var(--sp-6);
    font-size: var(--fs-sm);
  }
</style>
