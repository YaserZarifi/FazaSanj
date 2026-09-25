<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { ai } from "../../lib/stores/ai.svelte";
  import { settings } from "../../lib/stores/settings.svelte";
  import AiWarning from "../common/AiWarning.svelte";
  import Dialog from "../common/Dialog.svelte";
  import Toggle from "../common/Toggle.svelte";
  import ProviderCard from "./ProviderCard.svelte";

  let confirmEnable = $state(false);
  const s = $derived(settings.value);
</script>

{#if s}
  <div class="ai">
    <Toggle
      checked={s.aiEnabled}
      label={t("aiSettings.enable")}
      description={t("aiSettings.enableHint")}
      onchange={(v) => (v ? (confirmEnable = true) : settings.update({ aiEnabled: false }))}
    />

    {#if s.aiEnabled}
      <AiWarning />
      <Toggle
        checked={s.aiMaskNames}
        label={t("aiSettings.mask")}
        description={t("aiSettings.maskHint")}
        onchange={(v) => settings.update({ aiMaskNames: v })}
      />
      <p class="muted small">{t("aiSettings.metadataOnly")}</p>
      <div class="provs">
        {#each ai.providers as p (p.provider)}
          <ProviderCard status={p} />
        {/each}
      </div>
    {/if}
  </div>

  <Dialog open={confirmEnable} title={t("aiSettings.enableTitle")} onclose={() => (confirmEnable = false)}>
    <p class="warnbox">{t("ai.warning")}</p>
    <p class="muted small">{t("aiSettings.enableDetails")}</p>
    {#snippet footer()}
      <button type="button" class="btn" onclick={() => (confirmEnable = false)}>{t("common.cancel")}</button>
      <button
        type="button"
        class="btn btn-primary"
        onclick={() => {
          confirmEnable = false;
          void settings.update({ aiEnabled: true });
        }}
      >
        {t("aiSettings.enableConfirm")}
      </button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .ai {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  .small {
    font-size: var(--fs-sm);
  }

  .provs {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .warnbox {
    padding: var(--sp-3) var(--sp-4);
    border-radius: var(--r-md);
    background: var(--careful-soft);
    margin-bottom: var(--sp-3);
  }
</style>
