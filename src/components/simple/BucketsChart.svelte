<script lang="ts">
  import type { StoryBucket } from "../../lib/api/types";
  import { formatPercent, formatSize } from "../../lib/format";
  import { categoryName, i18n, t } from "../../lib/i18n/index.svelte";
  import { themeState } from "../../lib/stores/themeState.svelte";
  import { categoryColor } from "../../lib/theme";
  import { FONT, tooltipHtml, tooltipStyle } from "../viz/chartTheme";
  import EChart from "../viz/EChart.svelte";

  let { buckets, total }: { buckets: StoryBucket[]; total: number } = $props();

  const shown = $derived(buckets.filter((b) => b.bytes > 0));

  const option = $derived.by(() => {
    void themeState.resolved;
    const lang = i18n.lang;
    return {
      textStyle: { fontFamily: FONT },
      tooltip: {
        ...tooltipStyle(lang),
        trigger: "item",
        formatter: (p: { name: string; value: number; color: string }) =>
          tooltipHtml(p.name, [
            [t("chart.size"), formatSize(p.value, lang)],
            [t("chart.share"), formatPercent(total ? p.value / total : 0, lang)],
          ], p.color),
      },
      series: [
        {
          type: "pie",
          radius: ["62%", "92%"],
          padAngle: 1.5,
          itemStyle: { borderRadius: 5 },
          label: { show: false },
          emphasis: { scale: true, scaleSize: 4 },
          data: shown.map((b) => ({ name: categoryName(b.category), value: b.bytes, itemStyle: { color: categoryColor(b.category) } })),
        },
      ],
    };
  });
</script>

<div class="wrap">
  <div class="donut">
    <EChart {option} height="210px" label={t("story.chartLabel")} />
    <div class="center" aria-hidden="true">
      <span class="v num">{formatSize(total, i18n.lang)}</span>
      <span class="l">{t("story.counted")}</span>
    </div>
  </div>
  <ul class="legend">
    {#each shown.slice(0, 7) as b (b.category)}
      <li>
        <span class="dot" style:background="var(--cat-{b.category})"></span>
        <span class="name">{categoryName(b.category)}</span>
        <span class="size num">{formatSize(b.bytes, i18n.lang)}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .wrap {
    display: grid;
    grid-template-columns: 210px 1fr;
    gap: var(--sp-5);
    align-items: center;
  }

  .donut {
    position: relative;
  }

  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }

  .v {
    font-weight: var(--fw-bold);
    font-size: var(--fs-lg);
  }

  .l {
    font-size: var(--fs-xs);
    color: var(--text-3);
  }

  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    font-size: var(--fs-sm);
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .name {
    flex: 1;
    color: var(--text-2);
  }

  .size {
    font-weight: var(--fw-medium);
  }
</style>
