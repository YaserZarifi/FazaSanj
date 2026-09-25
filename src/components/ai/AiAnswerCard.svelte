<script lang="ts">
  import type { AiAnswer, Explanation, NodeInfo } from "../../lib/api/types";
  import { formatRelative } from "../../lib/format";
  import { cappedSafety } from "../../lib/heuristicTarget";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import AiWarning from "../common/AiWarning.svelte";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import SourceBadge from "../common/SourceBadge.svelte";

  let { answer, node }: { answer: AiAnswer; node: NodeInfo } = $props();

  // AI answers are capped at "probably safe", whatever the model says
  const safety = $derived(cappedSafety(answer.safety));
  const inBasket = $derived(cleanup.has(node.path));

  function asExplanation(): Explanation {
    const same = (s: string) => ({ fa: s, en: s });
    return {
      ruleId: `ai-${answer.provider}`,
      source: "ai",
      title: same(node.name),
      whyBig: same(answer.whyBig),
      ifDeleted: same(answer.consequence),
      safety,
      method: "recycle",
      needsAdmin: false,
      instructions: null,
      confidence: null,
    };
  }
</script>

<div class="ai">
  <div class="head">
    <SourceBadge source="ai" />
    <span class="faint small">
      {answer.model}{answer.cached ? ` · ${t("ai.cached")}` : ""} · {formatRelative(answer.createdAt, i18n.lang)}
    </span>
  </div>
  <p><b>{t("ai.what")}</b> {answer.what}</p>
  <p><b>{t("reason.why")}</b> {answer.whyBig}</p>
  <p><b>{t("reason.whatIf")}</b> {answer.consequence}</p>
  <div class="row">
    <SafetyBadge level={safety} />
    {#if safety !== "do_not_touch" && safety !== "careful"}
      <button type="button" class="btn btn-sm" aria-pressed={inBasket} onclick={() => cleanup.toggle({ path: node.path, bytes: node.size, explanation: asExplanation() })}>
        <Icon name={inBasket ? "check" : "plus"} size={14} />{inBasket ? t("reason.inCleanup") : t("reason.addToCleanup")}
      </button>
    {/if}
  </div>
  <AiWarning />
</div>

<style>
  .ai {
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    padding: var(--sp-4);
    border-radius: var(--r-lg);
    border: 1px solid color-mix(in srgb, var(--ai) 35%, transparent);
    background: color-mix(in srgb, var(--ai-soft) 60%, var(--surface));
    font-size: var(--fs-sm);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    flex-wrap: wrap;
  }

  .small {
    font-size: var(--fs-xs);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-2);
  }
</style>
