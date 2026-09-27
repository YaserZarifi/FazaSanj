<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { CleanupTarget, Reason, ScanSummary, Story } from "../../lib/api/types";
  import { formatPercent, formatSize } from "../../lib/format";
  import { bi, errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { categoryColor, categoryKey } from "../../lib/labels";
  import { app } from "../../lib/stores/app.svelte";
  import { storySentences, topReasons } from "../../lib/story";
  import Icon from "../common/Icon.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";
  import Spinner from "../common/Spinner.svelte";
  import ReasonCard from "./ReasonCard.svelte";

  let { summary }: { summary: ScanSummary } = $props();

  let story = $state<Story | null>(null);
  let error = $state<string | null>(null);
  let chosen = $state<Record<number, boolean>>({});

  $effect(() => {
    const id = summary.scanId;
    story = null;
    error = null;
    api.getStory(id).then((s) => (story = s)).catch((e) => (error = errorText(e)));
  });

  const isDrive = $derived(/^[a-z]:\\?$/i.test(summary.rootPath));
  const sentences = $derived(story ? storySentences(story, t, lang(), isDrive) : []);
  const top = $derived(story ? topReasons(story) : []);
  const bucketTotal = $derived(story ? story.buckets.reduce((a, b) => a + b.bytes, 0) : 0);
  const chosenList = $derived(story ? story.needsDecision.filter((r) => chosen[r.nodeId]) : []);
  const chosenBytes = $derived(chosenList.reduce((a, r) => a + r.bytes, 0));

  const toTarget = (r: Reason): CleanupTarget => ({ path: r.path, bytes: r.bytes, explanation: r.explanation });
</script>

{#if error}
  <div class="pad"><div class="notice error">{error}</div></div>
{:else if !story}
  <div class="pad center"><Spinner size={28} /></div>
{:else}
  <section class="story">
    <div class="card intro">
      <div class="sentences">
        {#each sentences as s, i (i)}
          <p class:lead={i === 0}>{s}</p>
        {/each}
      </div>

      {#if bucketTotal > 0}
        <div class="stack" role="img" aria-label={t("story.bucketsLabel")}>
          {#each story.buckets as b (b.category)}
            <span
              style:width="{(b.bytes / bucketTotal) * 100}%"
              style:background={categoryColor(b.category)}
              title="{t(categoryKey(b.category))}: {formatSize(b.bytes, lang())}"
            ></span>
          {/each}
        </div>
        <ul class="legend">
          {#each story.buckets.slice(0, 8) as b (b.category)}
            <li>
              <span class="swatch" style:background={categoryColor(b.category)}></span>
              {t(categoryKey(b.category))}
              <span class="faint">{formatSize(b.bytes, lang())} · {formatPercent(b.bytes / bucketTotal, lang())}</span>
            </li>
          {/each}
        </ul>
      {/if}

      <div class="actions">
        <button
          class="btn primary big"
          disabled={story.safeItems.length === 0}
          onclick={() => app.openCleanup(story!.safeItems.map(toTarget))}
        >
          <Icon name="shield-check" size={20} />
          {story.safeItems.length ? t("story.freeButton", { size: formatSize(story.safeBytes, lang()) }) : t("story.nothingSafe")}
        </button>
        <span class="faint">{t("story.freeHint")}</span>
      </div>
    </div>

    {#if top.length}
      <h2>{t("story.topReasons")}</h2>
      <div class="cards">
        {#each top as r (r.nodeId)}
          <ReasonCard reason={r} />
        {/each}
      </div>
    {/if}

    {#if story.needsDecision.length}
      <div class="decide-head">
        <div>
          <h2>{t("story.needsDecision")}</h2>
          <p class="muted">{t("story.needsDecisionHint")}</p>
        </div>
        <span class="spacer"></span>
        <button class="btn" disabled={chosenList.length === 0} onclick={() => app.openCleanup(chosenList.map(toTarget))}>
          {t("story.reviewChosen", { count: chosenList.length, size: formatSize(chosenBytes, lang()) })}
        </button>
      </div>
      <div class="card">
        <table class="list">
          <tbody>
            {#each story.needsDecision as r (r.nodeId)}
              <tr>
                <td style="width: 36px">
                  <input
                    type="checkbox"
                    aria-label={bi(r.explanation.title)}
                    checked={!!chosen[r.nodeId]}
                    onchange={(e) => (chosen[r.nodeId] = e.currentTarget.checked)}
                  />
                </td>
                <td>
                  <div class="decision">
                    <strong>{bi(r.explanation.title)}</strong>
                    <span class="muted small">{bi(r.explanation.ifDeleted)}</span>
                    <bdi class="path faint">{r.path}</bdi>
                  </div>
                </td>
                <td><SafetyBadge level={r.explanation.safety} small /></td>
                <td class="num"><strong>{formatSize(r.bytes, lang())}</strong></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}

    {#if story.reasons.length === 0}
      <div class="notice info">{t("story.nothingKnown")}</div>
    {/if}
  </section>
{/if}

<style>
  .pad {
    padding: 24px;
  }
  .center {
    display: grid;
    place-items: center;
    min-height: 200px;
  }
  .story {
    max-width: 960px;
    width: 100%;
    margin: 0 auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .intro {
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .sentences {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 15px;
  }
  .lead {
    font-size: 19px;
    font-weight: 700;
  }
  .stack {
    display: flex;
    height: 14px;
    border-radius: 99px;
    overflow: hidden;
    background: var(--bar-track);
  }
  .stack span {
    height: 100%;
    min-width: 2px;
  }
  .legend {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
  }
  .decide-head {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    margin-top: 8px;
  }
  .decision {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .small {
    font-size: 13px;
  }
</style>
