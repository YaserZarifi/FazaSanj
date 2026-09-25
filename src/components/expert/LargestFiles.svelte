<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { FileEntry } from "../../lib/api/types";
  import { formatRelative, formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Icon from "../common/Icon.svelte";
  import PathText from "../common/PathText.svelte";
  import Skeleton from "../common/Skeleton.svelte";

  let files = $state<FileEntry[] | null>(null);

  $effect(() => {
    void scan.version;
    const id = scan.scanId;
    if (id == null) return;
    files = null;
    backend
      .getLargestFiles(id, 100)
      .then((f) => (files = f))
      .catch((e: unknown) => {
        files = [];
        toasts.error(e);
      });
  });
</script>

{#if files == null}
  <div class="sk">
    {#each Array(10) as _, i (i)}<Skeleton height={40} />{/each}
  </div>
{:else if files.length === 0}
  <EmptyState compact icon="file-stack" title={t("largest.empty")} />
{:else}
  <ol class="list">
    {#each files as f, i (f.id)}
      <li>
        <button type="button" class="row" class:sel={scan.selected?.id === f.id} onclick={() => scan.select(f.id)}>
          <span class="rank num">{i + 1}</span>
          <span class="ic" style:color="var(--cat-{f.category})"><Icon name="file" size={17} /></span>
          <span class="nm">
            <span class="ltr name">{f.name}</span>
            <PathText path={f.path} max={80} />
          </span>
          <span class="mod">{formatRelative(f.modified, i18n.lang)}</span>
          <span class="size num">{formatSize(f.size, i18n.lang)}</span>
        </button>
      </li>
    {/each}
  </ol>
{/if}

<style>
  .sk {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row {
    display: grid;
    grid-template-columns: 32px 24px minmax(0, 1fr) 120px 90px;
    align-items: center;
    gap: var(--sp-3);
    width: 100%;
    padding: 6px var(--sp-3);
    border: 0;
    border-radius: var(--r-md);
    background: none;
    text-align: start;
    cursor: pointer;
  }

  .row:hover {
    background: var(--surface-hover);
  }

  .sel {
    background: var(--accent-soft);
  }

  .rank {
    color: var(--text-3);
    font-size: var(--fs-xs);
    text-align: center;
  }

  .nm {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: var(--fw-medium);
  }

  .mod {
    font-size: var(--fs-xs);
    color: var(--text-2);
  }

  .size {
    text-align: end;
    font-weight: var(--fw-bold);
  }
</style>
