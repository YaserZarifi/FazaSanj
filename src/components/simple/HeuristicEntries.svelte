<script lang="ts">
  import type { HeuristicKind } from "../../lib/api/types";
  import { formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { heuristics } from "../../lib/stores/heuristics.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { ui } from "../../lib/stores/ui.svelte";
  import Icon from "../common/Icon.svelte";
  import type { IconName } from "../common/icons";

  const ENTRIES: { kind: HeuristicKind; icon: IconName }[] = [
    { kind: "duplicates", icon: "copy" },
    { kind: "old_project", icon: "archive" },
    { kind: "orphan", icon: "package" },
    { kind: "stale", icon: "hourglass" },
  ];

  function open(kind: HeuristicKind) {
    if (scan.scanId != null) void heuristics.run(scan.scanId, kind);
    ui.openHeuristics(kind);
  }
</script>

<section aria-labelledby="he-title">
  <h3 id="he-title" class="section-title">{t("heur.entryTitle")}</h3>
  <p class="muted small">{t("heur.entrySubtitle")}</p>
  <div class="grid">
    {#each ENTRIES as e (e.kind)}
      {@const st = heuristics.state(e.kind, scan.scanId)}
      <button type="button" class="card entry" onclick={() => open(e.kind)}>
        <span class="ic"><Icon name={e.icon} size={20} /></span>
        <span class="t">{t(`heur.kinds.${e.kind}.title`)}</span>
        <span class="d">{t(`heur.kinds.${e.kind}.short`)}</span>
        <span class="r num">
          {#if st?.running}
            {t("heur.running")}
          {:else if st?.findings}
            {t("heur.found", { count: st.findings.length, size: formatSize(st.findings.reduce((s, f) => s + f.bytes, 0), i18n.lang) })}
          {:else}
            {t("heur.check")}
          {/if}
          <Icon name="chevron-right" size={15} flip />
        </span>
      </button>
    {/each}
  </div>
</section>

<style>
  .small {
    font-size: var(--fs-sm);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--sp-3);
    margin-top: var(--sp-3);
  }

  .entry {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: var(--sp-4);
    text-align: start;
    cursor: pointer;
    transition: border-color var(--dur-fast) var(--ease), transform var(--dur-fast) var(--ease), box-shadow var(--dur-fast) var(--ease);
  }

  .entry:hover {
    border-color: var(--border-strong);
    box-shadow: var(--shadow-md);
    transform: translateY(-1px);
  }

  .ic {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: var(--r-md);
    background: var(--surface-3);
    color: var(--text-2);
  }

  .t {
    font-weight: var(--fw-bold);
  }

  .d {
    font-size: var(--fs-xs);
    color: var(--text-2);
    flex: 1;
  }

  .r {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: var(--fs-xs);
    color: var(--accent-text);
    font-weight: var(--fw-medium);
  }
</style>
