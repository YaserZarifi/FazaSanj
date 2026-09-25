<script lang="ts">
  import { formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { storySentences } from "../../lib/story";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import BucketsChart from "./BucketsChart.svelte";
  import GrowthCard from "./GrowthCard.svelte";
  import HeuristicEntries from "./HeuristicEntries.svelte";
  import NeedsDecision from "./NeedsDecision.svelte";
  import ReasonCard from "./ReasonCard.svelte";

  const story = $derived(scan.story);
  const sentences = $derived(story ? storySentences(story, i18n.lang) : []);

  function freeSafe() {
    if (!story) return;
    void cleanup.review(story.safeItems.map((r) => ({ path: r.path, bytes: r.bytes, explanation: r.explanation })));
  }
</script>

<div class="page">
  {#if scan.storyLoading && !story}
    <div class="card hero">
      <div class="sk-text">
        <Skeleton height={28} width="70%" />
        <Skeleton height={16} />
        <Skeleton height={16} width="85%" />
        <Skeleton height={16} width="60%" />
      </div>
      <Skeleton height={200} width="200px" radius="50%" />
    </div>
  {:else if !story}
    <EmptyState icon="alert" title={t("story.failed")}>
      <button type="button" class="btn" onclick={() => scan.loadStory()}>{t("common.retry")}</button>
    </EmptyState>
  {:else}
    <section class="card hero" aria-labelledby="story-title">
      <div class="text">
        <h2 id="story-title">{t("story.title")}</h2>
        <p class="story">{sentences.join(" ")}</p>
        {#if story.safeBytes > 0}
          <div class="cta">
            <button type="button" class="btn btn-primary btn-lg" onclick={freeSafe}>
              <Icon name="broom" size={20} />
              {t("story.freeSafe", { size: formatSize(story.safeBytes, i18n.lang) })}
            </button>
            <p class="faint small">{t("story.freeSafeNote", { count: story.safeItems.length })}</p>
          </div>
        {/if}
      </div>
      <BucketsChart buckets={story.buckets} total={story.countedBytes} />
    </section>

    <div class="cols">
      <section class="reasons" aria-labelledby="reasons-title">
        <h3 id="reasons-title" class="section-title">{t("story.topReasons")}</h3>
        {#if story.reasons.length === 0}
          <EmptyState compact icon="search" title={t("story.noReasons")} />
        {:else}
          <div class="list">
            {#each story.reasons as r, i (r.path)}
              <ReasonCard reason={r} rank={i} />
            {/each}
          </div>
        {/if}
      </section>

      <aside class="side">
        {#if scan.sinceLast}
          <GrowthCard comparison={scan.sinceLast} />
        {/if}
        <NeedsDecision items={story.needsDecision} />
      </aside>
    </div>

    <HeuristicEntries />
  {/if}
</div>

<style>
  .page {
    padding: var(--sp-5) var(--sp-6) var(--sp-12);
    display: flex;
    flex-direction: column;
    gap: var(--sp-6);
    max-width: 1280px;
  }

  .hero {
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: var(--sp-8);
    padding: var(--sp-8);
    align-items: center;
    background:
      radial-gradient(1200px 300px at 0% 0%, var(--accent-soft), transparent 60%),
      var(--surface);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: var(--sp-4);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .story {
    font-size: var(--fs-lg);
    line-height: 1.9;
    color: var(--text);
  }

  .cta {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sp-2);
    margin-top: var(--sp-2);
  }

  .small {
    font-size: var(--fs-xs);
  }

  .sk-text {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1.4fr) minmax(0, 1fr);
    gap: var(--sp-6);
    align-items: start;
  }

  .reasons {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .side {
    display: flex;
    flex-direction: column;
    gap: var(--sp-5);
  }

  @media (max-width: 1100px) {
    .hero,
    .cols {
      grid-template-columns: 1fr;
    }
  }
</style>
