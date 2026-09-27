<script lang="ts">
  // Size of one folder across snapshots.
  import * as api from "../../lib/api/commands";
  import type { GrowthPoint } from "../../lib/api/types";
  import { cssVar } from "../../lib/charts";
  import { formatDate, formatSize } from "../../lib/format";
  import { lang, t } from "../../lib/i18n/index.svelte";
  import Chart from "../viz/Chart.svelte";

  let { path, compact = false }: { path: string; compact?: boolean } = $props();
  let points = $state<GrowthPoint[]>([]);

  $effect(() => {
    const p = path;
    api.getGrowth(p).then((r) => (points = r)).catch(() => (points = []));
  });

  const option = $derived.by(() => {
    const l = lang();
    const text = cssVar("--text-3");
    const accent = cssVar("--accent");
    return {
      grid: { left: 8, right: 8, top: 10, bottom: 4, containLabel: true },
      tooltip: {
        trigger: "axis",
        backgroundColor: cssVar("--surface"),
        borderColor: cssVar("--border"),
        textStyle: { color: cssVar("--text"), fontFamily: "Vazirmatn" },
        valueFormatter: (v: number) => formatSize(v, l),
      },
      xAxis: {
        type: "category",
        boundaryGap: false,
        inverse: l === "fa",
        data: points.map((p) => formatDate(p.takenAt, l)),
        axisLabel: { color: text, fontFamily: "Vazirmatn", fontSize: 11, show: !compact },
        axisLine: { lineStyle: { color: cssVar("--border") } },
      },
      yAxis: {
        type: "value",
        position: l === "fa" ? "right" : "left",
        axisLabel: { color: text, fontFamily: "Vazirmatn", fontSize: 11, formatter: (v: number) => formatSize(v, l) },
        splitLine: { lineStyle: { color: cssVar("--border") } },
      },
      series: [
        {
          type: "line",
          data: points.map((p) => p.bytes),
          smooth: true,
          symbolSize: 6,
          lineStyle: { color: accent, width: 2 },
          itemStyle: { color: accent },
          areaStyle: { color: accent, opacity: 0.12 },
        },
      ],
    };
  });
</script>

{#if points.length >= 2}
  <section class="growth">
    <h4>{t("growth.chartTitle")}</h4>
    <Chart {option} height={compact ? "150px" : "280px"} label={t("growth.chartTitle")} />
  </section>
{/if}

<style>
  .growth {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  h4 {
    font-size: 13px;
    color: var(--text-2);
  }
</style>
