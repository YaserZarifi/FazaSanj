<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { Reason } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { bi, lang, t } from "../../lib/i18n/index.svelte";
  import { CATEGORY_ICONS, categoryColor, methodKey, safetyVar } from "../../lib/labels";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";

  let { reason }: { reason: Reason } = $props();
  let open = $state(false);
  const e = $derived(reason.explanation);
</script>

<article class="card reason" style:--safety="var({safetyVar(e.safety)})">
  <div class="top">
    <span class="icon" style:background={categoryColor(reason.category)}>
      <Icon name={CATEGORY_ICONS[reason.category]} size={18} />
    </span>
    <div class="head">
      <h3>{bi(e.title)}</h3>
      <SafetyBadge level={e.safety} small />
    </div>
    <strong class="size">{formatSize(reason.bytes, lang())}</strong>
  </div>
  <p class="why muted">{bi(e.whyBig)}</p>

  <button class="btn ghost small toggle" aria-expanded={open} onclick={() => (open = !open)}>
    <Icon name="chevron-down" size={14} class={open ? "rot" : ""} />
    {t("reason.whatHappens")}
  </button>
  {#if open}
    <div class="more">
      <p>{bi(e.ifDeleted)}</p>
      {#if e.instructions}
        <p class="instr"><strong>{t("reason.howTo")}</strong> {bi(e.instructions)}</p>
      {/if}
      <p class="faint">{t("reason.method")}: {t(methodKey(e.method))}</p>
      <div class="row">
        <bdi class="path faint">{reason.path}</bdi>
        <span class="spacer"></span>
        <button class="btn small" onclick={() => api.revealInExplorer(reason.path)}>
          <Icon name="external" size={14} />
          {t("common.showInExplorer")}
        </button>
      </div>
    </div>
  {/if}
</article>

<style>
  .reason {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-inline-start: 4px solid var(--safety);
  }
  .top {
    display: flex;
    gap: 12px;
    align-items: flex-start;
  }
  .icon {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: #fff;
    flex-shrink: 0;
  }
  .head {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-start;
  }
  .size {
    font-size: 17px;
    white-space: nowrap;
  }
  .toggle {
    align-self: flex-start;
  }
  .toggle :global(.rot) {
    transform: rotate(180deg);
  }
  .more {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    background: var(--surface-2);
    border-radius: var(--radius-sm);
  }
  .instr {
    white-space: pre-line;
  }
</style>
