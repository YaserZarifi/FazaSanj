<script lang="ts">
  import type { NodeInfo } from "../../lib/api/types";
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import { ai } from "../../lib/stores/ai.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";
  import AiAnswerCard from "./AiAnswerCard.svelte";

  let { node }: { node: NodeInfo } = $props();
  const answer = $derived(ai.answers[node.path]);
  const loading = $derived(ai.loading[node.path] ?? false);
  const error = $derived(ai.errors[node.path]);
</script>

{#if answer}
  <AiAnswerCard {answer} {node} />
{:else if ai.enabled}
  <div class="ask">
    <p class="muted small">{t("ai.noRule")}</p>
    {#if !ai.ready}
      <button type="button" class="btn btn-sm" onclick={() => ui.go("settings")}>
        <Icon name="key" size={15} />{t("ai.addKeyFirst")}
      </button>
    {:else}
      <button type="button" class="btn btn-sm ai-btn" disabled={loading} onclick={() => ai.ask(node)}>
        {#if loading}<Spinner size={14} />{t("ai.thinking")}{:else}<Icon name="sparkles" size={15} />{t("ai.ask")}{/if}
      </button>
    {/if}
    {#if error}
      <p class="err" role="alert"><Icon name="alert" size={14} /> {errorText(error)}</p>
    {/if}
  </div>
{/if}

<style>
  .ask {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
  }

  .small {
    font-size: var(--fs-sm);
  }

  .ai-btn {
    color: var(--ai);
    border-color: color-mix(in srgb, var(--ai) 40%, transparent);
  }

  .err {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--danger);
    font-size: var(--fs-sm);
  }
</style>
