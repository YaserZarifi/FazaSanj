<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { HistoryEntry } from "../../lib/api/types";
  import { formatDate, formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { methodKey } from "../../lib/labels";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import Spinner from "../common/Spinner.svelte";

  const PAGE = 20;
  let runs = $state<HistoryEntry[]>([]);
  let loading = $state(true);
  let more = $state(false);
  let error = $state<string | null>(null);

  async function load(offset: number) {
    loading = true;
    try {
      const page = await api.getCleanupHistory(PAGE, offset);
      runs = offset === 0 ? page : [...runs, ...page];
      more = page.length === PAGE;
    } catch (e) {
      error = errorText(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void app.cleanupRuns;
    load(0);
  });
</script>

<section class="page">
  <p class="muted">{t("history.intro")}</p>
  {#if error}<div class="notice error">{error}</div>{/if}
  {#if !loading && runs.length === 0 && !error}
    <div class="empty">
      <Icon name="history" size={40} />
      <p class="muted">{t("history.empty")}</p>
    </div>
  {/if}
  {#each runs as run (run.runId)}
    <article class="card run">
      <div class="row">
        <strong>{formatDate(run.startedAt, lang(), true)}</strong>
        {#if run.dryRun}<span class="pill">{t("history.dryRun")}</span>{/if}
        <span class="spacer"></span>
        <strong>{run.dryRun ? "" : t("cleanup.freed", { size: formatSize(run.bytesFreed, lang()) })}</strong>
      </div>
      <ul>
        {#each run.actions as a, i (i)}
          <li>
            <bdi class="path">{a.path}</bdi>
            <span class="faint">{t(methodKey(a.method))} · {t(`status.${a.status}`)}{a.bytes ? ` · ${formatSize(a.bytes, lang())}` : ""}</span>
            {#if a.restorable}
              <button class="btn small" onclick={() => api.openRecycleBin()}>
                <Icon name="refresh" size={13} />{t("history.restore")}
              </button>
            {/if}
          </li>
        {/each}
      </ul>
    </article>
  {/each}
  {#if loading}<Spinner />{/if}
  {#if more && !loading}
    <button class="btn" onclick={() => load(runs.length)}>{t("history.more")}</button>
  {/if}
</section>

<style>
  .page {
    max-width: 900px;
    margin: 0 auto;
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 48px 0;
    color: var(--text-3);
  }
  .run {
    padding: 14px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .pill {
    font-size: 12px;
    padding: 0 8px;
    border-radius: 99px;
    background: var(--surface-2);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: center;
  }
</style>
