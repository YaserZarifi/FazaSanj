<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { TypeGroup } from "../../lib/api/types";
  import { formatCompact, formatPercent, formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Skeleton from "../common/Skeleton.svelte";

  let groups = $state<TypeGroup[] | null>(null);
  const total = $derived(groups?.reduce((s, g) => s + g.bytes, 0) ?? 0);
  const max = $derived(groups?.[0]?.bytes ?? 1);

  $effect(() => {
    void scan.version;
    const id = scan.scanId;
    if (id == null) return;
    groups = null;
    backend
      .getByType(id)
      .then((g) => (groups = [...g].sort((a, b) => b.bytes - a.bytes)))
      .catch((e: unknown) => {
        groups = [];
        toasts.error(e);
      });
  });
</script>

{#if groups == null}
  <div class="sk">{#each Array(8) as _, i (i)}<Skeleton height={56} />{/each}</div>
{:else if groups.length === 0}
  <EmptyState compact icon="shapes" title={t("types.empty")} />
{:else}
  <ul class="groups">
    {#each groups as g (g.group)}
      <li class="card g">
        <div class="line">
          <span class="name">{t(`types.groups.${g.group}`)}</span>
          <span class="faint small num">{t("types.files", { n: formatCompact(g.files, i18n.lang) })}</span>
          <span class="size num">{formatSize(g.bytes, i18n.lang)}</span>
          <span class="pct num">{formatPercent(total ? g.bytes / total : 0, i18n.lang)}</span>
        </div>
        <div class="bar" aria-hidden="true"><span style:width="{(g.bytes / max) * 100}%"></span></div>
        {#if g.topExtensions.length}
          <div class="exts">
            {#each g.topExtensions as e (e.ext)}
              <span class="ext"><b class="ltr">.{e.ext}</b> <span class="num">{formatSize(e.bytes, i18n.lang)}</span></span>
            {/each}
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .sk {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .groups {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
  }

  .g {
    padding: var(--sp-3) var(--sp-4);
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-shadow: none;
  }

  .line {
    display: flex;
    align-items: baseline;
    gap: var(--sp-3);
  }

  .name {
    font-weight: var(--fw-bold);
    flex: 1;
  }

  .small {
    font-size: var(--fs-xs);
  }

  .size {
    font-weight: var(--fw-bold);
    min-width: 80px;
    text-align: end;
  }

  .pct {
    min-width: 44px;
    text-align: end;
    color: var(--text-3);
    font-size: var(--fs-xs);
  }

  .bar {
    height: 6px;
    border-radius: var(--r-full);
    background: var(--surface-3);
    overflow: hidden;
  }

  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: inherit;
  }

  .exts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .ext {
    font-size: var(--fs-xs);
    padding: 1px 8px;
    border-radius: var(--r-full);
    background: var(--surface-2);
    color: var(--text-2);
  }

  .ext b {
    font-weight: var(--fw-medium);
    color: var(--text);
  }
</style>
