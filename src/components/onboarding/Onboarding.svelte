<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import Dialog from "../common/Dialog.svelte";
  import Icon from "../common/Icon.svelte";
  import type { IconName } from "../common/icons";

  const STEPS: { key: string; icon: IconName; points: number }[] = [
    { key: "what", icon: "search", points: 3 },
    { key: "safety", icon: "shield-check", points: 3 },
    { key: "ai", icon: "sparkles", points: 3 },
  ];

  let step = $state(0);
  const cur = $derived(STEPS[step]);

  async function finish() {
    ui.showOnboarding = false;
    await settings.update({ onboardingDone: true });
  }
</script>

<Dialog open={true} title={t(`onboarding.${cur.key}.title`)} width={560} onclose={finish}>
  <div class="step fade-in">
    {#key step}
      <span class="ic ic-{cur.key}"><Icon name={cur.icon} size={30} /></span>
      <p class="lead">{t(`onboarding.${cur.key}.lead`)}</p>
      <ul>
        {#each Array(cur.points) as _, i (i)}
          <li><Icon name="check" size={16} /> {t(`onboarding.${cur.key}.p${i + 1}`)}</li>
        {/each}
      </ul>
    {/key}
    <div class="dots" aria-label={t("onboarding.stepOf", { n: step + 1, total: STEPS.length })} role="img">
      {#each STEPS as s, i (s.key)}<span class:on={i === step}></span>{/each}
    </div>
  </div>
  {#snippet footer()}
    {#if step > 0}
      <button type="button" class="btn" onclick={() => step--}>{t("common.back")}</button>
    {:else}
      <button type="button" class="btn btn-ghost" onclick={finish}>{t("onboarding.skip")}</button>
    {/if}
    {#if step < STEPS.length - 1}
      <button type="button" class="btn btn-primary" onclick={() => step++}>{t("common.next")}<Icon name="arrow-right" size={15} flip /></button>
    {:else}
      <button type="button" class="btn btn-primary" onclick={finish}>{t("onboarding.start")}</button>
    {/if}
  {/snippet}
</Dialog>

<style>
  .step {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
    padding-top: var(--sp-2);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 60px;
    height: 60px;
    border-radius: var(--r-lg);
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .ic-safety {
    background: var(--safe-soft);
    color: var(--safe);
  }

  .ic-ai {
    background: var(--ai-soft);
    color: var(--ai);
  }

  .lead {
    font-size: var(--fs-lg);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }

  li {
    display: flex;
    gap: var(--sp-2);
    color: var(--text-2);
  }

  li :global(svg) {
    color: var(--safe);
    margin-top: 5px;
  }

  .dots {
    display: flex;
    gap: 6px;
    justify-content: center;
    margin-top: var(--sp-2);
  }

  .dots span {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--surface-3);
    transition: width var(--dur-med) var(--ease);
  }

  .dots .on {
    width: 22px;
    border-radius: 4px;
    background: var(--accent);
  }
</style>
