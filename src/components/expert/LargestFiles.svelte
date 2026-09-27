<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { FileEntry, NodeId, ScanId } from "../../lib/api/types";
  import { formatDate, formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { categoryColor, categoryKey } from "../../lib/labels";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";

  let { scanId, selectedId, onselect }: { scanId: ScanId; selectedId: NodeId | null; onselect: (id: NodeId) => void } =
    $props();

  let files = $state<FileEntry[] | null>(null);
  let error = $state<string | null>(null);
  let limit = $state(100);

  $effect(() => {
    const [s, l] = [scanId, limit];
    api.getLargestFiles(s, l).then((f) => (files = f)).catch((e) => (error = errorText(e)));
  });
</script>

{#if error}
  <div class="notice error">{error}</div>
{:else if !files}
  <div class="center"><Spinner /></div>
{:else}
  <table class="list">
    <thead>
      <tr>
        <th>{t("list.name")}</th>
        <th>{t("list.category")}</th>
        <th class="num">{t("list.size")}</th>
        <th class="num">{t("list.modified")}</th>
        <th></th>
      </tr>
    </thead>
    <tbody>
      {#each files as f (f.id)}
        <tr class="clickable" class:selected={selectedId === f.id} onclick={() => onselect(f.id)}>
          <td>
            <div class="name">
              <bdi>{f.name}</bdi>
              <bdi class="path faint">{f.path}</bdi>
            </div>
          </td>
          <td><span class="row"><span class="dot" style:background={categoryColor(f.category)}></span>{t(categoryKey(f.category))}</span></td>
          <td class="num"><strong>{formatSize(f.size, lang())}</strong></td>
          <td class="num faint">{formatDate(f.modified, lang())}</td>
          <td>
            <button
              class="btn ghost small"
              title={t("common.showInExplorer")}
              aria-label={t("common.showInExplorer")}
              onclick={(e) => { e.stopPropagation(); api.revealInExplorer(f.path); }}
            >
              <Icon name="external" size={14} />
            </button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if files.length === limit && limit < 1000}
    <div class="center"><button class="btn small" onclick={() => (limit += 200)}>{t("list.moreFiles")}</button></div>
  {/if}
{/if}

<style>
  .center {
    display: grid;
    place-items: center;
    padding: 16px;
  }
  .name {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
</style>
