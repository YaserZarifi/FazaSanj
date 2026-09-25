<script lang="ts">
  import type { Reason } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { bi, i18n, t } from "../../lib/i18n/index.svelte";
  import { cleanup } from "../../lib/stores/cleanup.svelte";
  import CategoryIcon from "../common/CategoryIcon.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import SafetyBadge from "../common/SafetyBadge.svelte";

  let { items }: { items: Reason[] } = $props();
  let picked = $state<string[]>([]);
  let openPath = $state<string | null>(null);

  const pickable = $derived(items.filter((r) => r.explanation.method !== "manual_only"));
  const pickedItems = $derived(pickable.filter((r) => picked.includes(r.path) && !cleanup.has(r.path)));
  const pickedBytes = $derived(pickedItems.reduce((s, r) => s + r.bytes, 0));

  function toggle(path: string) {
    picked = picked.includes(path) ? picked.filter((p) => p !== path) : [...picked, path];
  }

  function addPicked() {
    for (const r of pickedItems) cleanup.add({ path: r.path, bytes: r.bytes, explanation: r.explanation });
    picked = [];
  }
</script>

<section class="card box" aria-labelledby="nd-title">
  <header>
    <div>
      <h3 id="nd-title" class="section-title">{t("decision.title")}</h3>
      <p class="muted small">{t("decision.subtitle")}</p>
    </div>
    <button type="button" class="btn btn-sm" disabled={pickedItems.length === 0} onclick={addPicked}>
      <Icon name="plus" size={15} />
      {t("decision.addSelected", { count: pickedItems.length, size: formatSize(pickedBytes, i18n.lang) })}
    </button>
  </header>

  {#if items.length === 0}
    <EmptyState compact icon="check-circle" title={t("decision.emptyTitle")} text={t("decision.emptyText")} />
  {:else}
    <ul>
      {#each items as r (r.path)}
        {@const inBasket = cleanup.has(r.path)}
        {@const manual = r.explanation.method === "manual_only"}
        <li>
          <div class="row">
            <input
              type="checkbox"
              id="nd-{r.nodeId}"
              checked={inBasket || picked.includes(r.path)}
              disabled={inBasket || manual}
              onchange={() => toggle(r.path)}
            />
            <CategoryIcon category={r.category} size={32} />
            <label class="text" for="nd-{r.nodeId}">
              <span class="title">{bi(r.explanation.title)}</span>
              <PathText path={r.path} max={56} />
            </label>
            <SafetyBadge level={r.explanation.safety} />
            <span class="size num">{formatSize(r.bytes, i18n.lang)}</span>
            <button
              type="button"
              class="icon-btn"
              aria-expanded={openPath === r.path}
              aria-label={t("reason.whatIf")}
              title={t("reason.whatIf")}
              onclick={() => (openPath = openPath === r.path ? null : r.path)}
            >
              <Icon name="help" size={17} />
            </button>
          </div>
          {#if inBasket}<p class="tag">{t("reason.inCleanup")}</p>{/if}
          {#if manual}<p class="tag manual">{t("decision.manual")}</p>{/if}
          {#if openPath === r.path}
            <div class="more fade-in">
              <p><b>{t("reason.why")}</b> {bi(r.explanation.whyBig)}</p>
              <p><b>{t("reason.whatIf")}</b> {bi(r.explanation.ifDeleted)}</p>
              {#if r.explanation.instructions}<p class="muted">{bi(r.explanation.instructions)}</p>{/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .box {
    padding: var(--sp-5);
  }

  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--sp-3);
    margin-bottom: var(--sp-3);
  }

  .small {
    font-size: var(--fs-sm);
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  li {
    padding: var(--sp-3) 0;
    border-top: 1px solid var(--border);
  }

  li:first-child {
    border-top: 0;
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--sp-3);
  }

  input[type="checkbox"] {
    width: 18px;
    height: 18px;
    accent-color: var(--accent);
    flex-shrink: 0;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    cursor: pointer;
  }

  .title {
    font-weight: var(--fw-medium);
  }

  .size {
    min-width: 88px;
    text-align: end;
    font-weight: var(--fw-bold);
  }

  .tag {
    margin-inline-start: 78px;
    font-size: var(--fs-xs);
    color: var(--safe);
  }

  .tag.manual {
    color: var(--text-3);
  }

  .more {
    margin: var(--sp-2) 0 0;
    margin-inline-start: 78px;
    padding: var(--sp-3);
    border-radius: var(--r-md);
    background: var(--surface-2);
    font-size: var(--fs-sm);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
</style>
