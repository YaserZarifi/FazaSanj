<script lang="ts">
  import * as api from "../../lib/api/commands";
  import type { DriveInfo, SnapshotComparison } from "../../lib/api/types";
  import { formatDelta, formatDuration, formatInt, formatRelative, formatSize } from "../../lib/format";
  import { lang, t } from "../../lib/i18n/index.svelte";
  import { app, type ScanSession } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";

  let { session, drive }: { session: ScanSession; drive: DriveInfo | null } = $props();
  const s = $derived(session.summary!);
  let growth = $state<SnapshotComparison | null>(null);
  const cleanedSince = $state({ base: app.cleanupRuns });

  $effect(() => {
    const id = s.scanId;
    api.compareWithLast(id).then((c) => (growth = c)).catch(() => (growth = null));
  });

  /** Space Windows says is used but no file accounts for (other users' files, system areas). */
  const uncounted = $derived(s.driveTotal > 0 && drive ? Math.max(0, s.driveTotal - s.driveFree - s.totalBytes) : 0);
</script>

<div class="bar">
  <div class="line">
    <Icon name="check" size={16} />
    <span>
      {t("summary.scanned", {
        files: formatInt(s.files, lang()),
        time: formatDuration(s.durationMs, lang()),
        size: formatSize(s.totalBytes, lang()),
      })}
    </span>
    <span class="faint">· {formatRelative(s.finishedAt, lang())}</span>
    <span class="spacer"></span>
    <button class="btn small" onclick={() => app.startScan(session.root, session.mode)}>
      <Icon name="refresh" size={14} />
      {t("summary.rescan")}
    </button>
  </div>

  {#if s.fallbackReason}
    <div class="notice warn"><Icon name="info" />{t(`fallback.${s.fallbackReason}`)}</div>
  {/if}
  {#if app.cleanupRuns > cleanedSince.base}
    <div class="notice info"><Icon name="refresh" />{t("summary.staleAfterCleanup")}</div>
  {/if}
  {#if growth && Math.abs(growth.totalDelta) >= 1024 * 1024 * 100}
    <button class="notice info growth" onclick={() => app.go({ kind: "growth" })}>
      <Icon name="growth" />
      {growth.totalDelta > 0
        ? t("summary.grew", { delta: formatDelta(growth.totalDelta, lang()), when: formatRelative(growth.from.takenAt, lang()) })
        : t("summary.shrank", { delta: formatDelta(growth.totalDelta, lang()), when: formatRelative(growth.from.takenAt, lang()) })}
    </button>
  {/if}
  {#if s.accessDenied > 0 || uncounted > 1024 ** 3}
    <p class="faint">
      {#if s.accessDenied > 0}{t("summary.denied", { count: s.accessDenied })}{/if}
      {#if uncounted > 1024 ** 3}{t("summary.uncounted", { size: formatSize(uncounted, lang()) })}{/if}
    </p>
  {/if}
</div>

<style>
  .bar {
    padding: 12px 24px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--safe);
    flex-wrap: wrap;
  }
  .line > span:first-of-type {
    color: var(--text);
  }
  .growth {
    cursor: pointer;
    text-align: start;
    color: var(--text);
  }
</style>
