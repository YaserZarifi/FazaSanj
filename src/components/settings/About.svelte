<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { digits, formatRelative } from "../../lib/format";
  import { lang, t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { checkForUpdate, installUpdate, updater } from "../../lib/updater.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";

  const REPO = "https://github.com/YaserZarifi/FazaSanj";

  function reportRule() {
    const title = encodeURIComponent("Wrong rule: ");
    const body = encodeURIComponent(
      [
        "Path (you can hide your user name):",
        "",
        "What the app said:",
        "",
        "What it really is:",
        "",
        `App version: ${app.info?.version ?? "?"}`,
      ].join("\n"),
    );
    openUrl(`${REPO}/issues/new?labels=rules&title=${title}&body=${body}`);
  }
</script>

<section class="page">
  <div class="card hero">
    <img src="/icon.svg" alt="" width="64" height="64" />
    <div>
      <h1>{t("app.name")}</h1>
      <p class="muted">{t("about.tagline")}</p>
      <p class="faint">{t("about.version", { version: digits(app.info?.version ?? "?", lang()) })}</p>
    </div>
  </div>

  <div class="card group">
    <h2>{t("about.updates")}</h2>
    {#if updater.available}
      <div class="notice info">
        <Icon name="download" />
        <div class="col">
          <strong>{t("about.updateReady", { version: updater.available.version })}</strong>
          {#if updater.available.notes}<p class="notes">{updater.available.notes}</p>{/if}
        </div>
      </div>
      <div>
        <button class="btn primary" onclick={installUpdate} disabled={updater.installing}>
          {#if updater.installing}<Spinner size={14} />{/if}
          {updater.installing ? t("about.installing", { pct: Math.round(updater.progress * 100) }) : t("about.install")}
        </button>
      </div>
    {:else}
      <div class="row">
        <button class="btn" onclick={() => checkForUpdate(false)} disabled={updater.checking}>
          {#if updater.checking}<Spinner size={14} />{:else}<Icon name="refresh" size={15} />{/if}
          {t("about.check")}
        </button>
        {#if updater.lastChecked && !updater.checking && !updater.error}
          <span class="faint">{t("about.upToDate")} · {formatRelative(updater.lastChecked, lang())}</span>
        {/if}
      </div>
    {/if}
    {#if updater.error}<div class="notice error">{t("about.updateFailed")}</div>{/if}
  </div>

  <div class="card group">
    <h2>{t("about.help")}</h2>
    <p class="muted">{t("about.reportHint")}</p>
    <div class="row wrap">
      <button class="btn" onclick={reportRule}><Icon name="alert" size={15} />{t("about.reportRule")}</button>
      <button class="btn" onclick={() => openUrl(REPO)}><Icon name="external" size={15} />{t("about.source")}</button>
    </div>
  </div>

  <div class="card group">
    <h2>{t("about.privacy")}</h2>
    <p>{t("about.privacyText")}</p>
    {#if app.info}
      <p class="faint">{t("about.dataDir")} <bdi class="path">{app.info.dataDir}</bdi></p>
    {/if}
    <p class="faint">{t("about.license")}</p>
  </div>
</section>

<style>
  .page {
    max-width: 760px;
    margin: 0 auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .hero {
    display: flex;
    gap: 18px;
    align-items: center;
    padding: 20px;
  }
  .group {
    padding: 18px 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .notes {
    white-space: pre-line;
    font-size: 13px;
  }
  .wrap {
    flex-wrap: wrap;
  }
</style>
