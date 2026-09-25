<script lang="ts">
  import type { ActionStatus } from "../../lib/api/types";
  import { t } from "../../lib/i18n/index.svelte";

  let { status }: { status: ActionStatus } = $props();

  const TONE: Record<ActionStatus, string> = {
    done: "good",
    dry_run: "info",
    partial: "warn",
    skipped_in_use: "warn",
    blocked: "bad",
    failed: "bad",
    needs_manual: "info",
    opened_setting: "info",
  };
</script>

<span class="st {TONE[status]}">{t(`status.${status}`)}</span>

<style>
  .st {
    font-size: 11px;
    font-weight: var(--fw-medium);
    padding: 1px 9px;
    border-radius: var(--r-full);
    white-space: nowrap;
  }

  .good {
    background: var(--safe-soft);
    color: var(--safe);
  }

  .info {
    background: var(--accent-soft);
    color: var(--accent-text);
  }

  .warn {
    background: var(--careful-soft);
    color: var(--careful);
  }

  .bad {
    background: var(--do-not-touch-soft);
    color: var(--do-not-touch);
  }
</style>
