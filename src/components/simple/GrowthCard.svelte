<script lang="ts">
  import type { SnapshotComparison } from "../../lib/api/types";
  import { formatRelative, formatSizeDelta } from "../../lib/format";
  import { bi, i18n, t } from "../../lib/i18n/index.svelte";
  import { growthSentence, lastSegment } from "../../lib/story";
  import { ui } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";

  let { comparison }: { comparison: SnapshotComparison } = $props();
  const top = $derived(comparison.items.slice(0, 4));
</script>

<section class="card growth" aria-labelledby="gc-title">
  <header>
    <span class="ic"><Icon name="growth" size={18} /></span>
    <div>
      <h3 id="gc-title" class="section-title">{t("growth.cardTitle")}</h3>
      <p class="faint small">{t("growth.since", { when: formatRelative(comparison.from.takenAt, i18n.lang) })}</p>
    </div>
  </header>
  <p class="sentence">{growthSentence(comparison, i18n.lang)}</p>
  <ul>
    {#each top as g (g.path)}
      <li>
        <span class="name">{g.explanation ? bi(g.explanation.title) : lastSegment(g.path)}</span>
        <span class="delta num" class:up={g.delta > 0} class:down={g.delta < 0}>{formatSizeDelta(g.delta, i18n.lang)}</span>
      </li>
    {/each}
  </ul>
  <button type="button" class="btn btn-sm btn-ghost link" onclick={() => ui.go("growth")}>
    {t("growth.seeAll")}<Icon name="arrow-right" size={15} flip />
  </button>
</section>

<style>
  .growth {
    padding: var(--sp-5);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  header {
    display: flex;
    gap: var(--sp-3);
    align-items: center;
  }

  .ic {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: var(--r-md);
    background: var(--careful-soft);
    color: var(--careful);
  }

  .small {
    font-size: var(--fs-xs);
  }

  .sentence {
    font-size: var(--fs-sm);
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  li {
    display: flex;
    justify-content: space-between;
    gap: var(--sp-3);
    font-size: var(--fs-sm);
  }

  .name {
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .delta {
    font-weight: var(--fw-medium);
    white-space: nowrap;
  }

  .up {
    color: var(--careful);
  }

  .down {
    color: var(--safe);
  }

  .link {
    align-self: flex-start;
    color: var(--accent-text);
    padding-inline: 0;
  }
</style>
