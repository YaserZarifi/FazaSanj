<script lang="ts">
  import type { Language } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";

  let step = $state(0);
  const steps = [
    { icon: "pie", title: "onboarding.s1.title", body: "onboarding.s1.body" },
    { icon: "shield-check", title: "onboarding.s2.title", body: "onboarding.s2.body" },
    { icon: "zap", title: "onboarding.s3.title", body: "onboarding.s3.body" },
  ];
  const current = $derived(steps[step]);

  function setLang(l: Language) {
    app.saveSettings({ language: l });
  }

  function finish() {
    app.saveSettings({ onboardingDone: true });
  }
</script>

<div class="onboarding">
  <div class="card box">
    <div class="langs" role="radiogroup" aria-label={t("settings.language")}>
      <button role="radio" aria-checked={app.settings.language === "fa"} class:on={app.settings.language === "fa"} onclick={() => setLang("fa")}>فارسی</button>
      <button role="radio" aria-checked={app.settings.language === "en"} class:on={app.settings.language === "en"} onclick={() => setLang("en")}>English</button>
    </div>

    <div class="icon"><Icon name={current.icon} size={56} /></div>
    <h1>{t(current.title)}</h1>
    <p class="muted body">{t(current.body)}</p>

    <div class="dots" aria-hidden="true">
      {#each steps as _, i (i)}<span class:on={i === step}></span>{/each}
    </div>

    <div class="row buttons">
      {#if step > 0}
        <button class="btn" onclick={() => step--}>{t("common.back")}</button>
      {:else}
        <button class="btn ghost" onclick={finish}>{t("onboarding.skip")}</button>
      {/if}
      <span class="spacer"></span>
      {#if step < steps.length - 1}
        <button class="btn primary" onclick={() => step++}>{t("common.next")}</button>
      {:else}
        <button class="btn primary" onclick={finish}>{t("onboarding.start")}</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .onboarding {
    height: 100%;
    display: grid;
    place-items: center;
    padding: 24px;
    background: radial-gradient(circle at 30% 20%, var(--accent-soft), var(--bg) 60%);
  }
  .box {
    width: 100%;
    max-width: 520px;
    padding: 28px 32px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 14px;
    box-shadow: var(--shadow-lg);
  }
  .langs {
    align-self: flex-end;
    display: flex;
    gap: 2px;
    background: var(--surface-2);
    padding: 3px;
    border-radius: var(--radius-sm);
  }
  .langs button {
    border: none;
    background: none;
    padding: 3px 12px;
    border-radius: 5px;
    cursor: pointer;
    color: var(--text-2);
  }
  .langs button.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: var(--shadow);
  }
  .icon {
    color: var(--accent);
    margin-top: 8px;
  }
  .body {
    font-size: 15px;
    min-height: 96px;
  }
  .dots {
    display: flex;
    gap: 6px;
  }
  .dots span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--surface-3);
  }
  .dots span.on {
    background: var(--accent);
    width: 22px;
    border-radius: 4px;
  }
  .buttons {
    width: 100%;
    margin-top: 8px;
  }
</style>
