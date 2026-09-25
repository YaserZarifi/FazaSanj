<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { GrowthPoint } from "../../lib/api/types";
  import { formatDate, formatSize } from "../../lib/format";
  import { i18n, t } from "../../lib/i18n/index.svelte";
  import { themeState } from "../../lib/stores/themeState.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import { cssVar } from "../../lib/theme";
  import EmptyState from "../common/EmptyState.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import { FONT, tooltipHtml, tooltipStyle } from "../viz/chartTheme";
  import EChart from "../viz/EChart.svelte";

  let { path }: { path: string } = $props();
  let points = $state<GrowthPoint[] | null>(null);

  $effect(() => {
    const p = path;
    points = null;
    backend
      .getGrowth(p)
      .then((x) => (points = x))
      .catch((e: unknown) => {
        points = [];
        toasts.error(e);
      });
  });

  const option = $derived.by(() => {
    void themeState.resolved;
    const lang = i18n.lang;
    const rtl = lang === "fa";
    const pts = points ?? [];
    const accent = cssVar("--accent");
    const axis = { color: cssVar("--text-3"), fontFamily: FONT, fontSize: 11 };
    return {
      textStyle: { fontFamily: FONT },
      grid: { left: rtl ? 16 : 70, right: rtl ? 70 : 16, top: 16, bottom: 30 },
      tooltip: {
        ...tooltipStyle(lang),
        trigger: "axis",
        formatter: (ps: { dataIndex: number }[]) => {
          const pt = pts[ps[0]?.dataIndex ?? 0];
          return pt ? tooltipHtml(formatDate(pt.takenAt, lang), [[t("chart.size"), formatSize(pt.bytes, lang)]], accent) : "";
        },
      },
      xAxis: {
        type: "category",
        inverse: rtl,
        data: pts.map((p) => formatDate(p.takenAt, lang)),
        axisLabel: axis,
        axisLine: { lineStyle: { color: cssVar("--border-strong") } },
        axisTick: { show: false },
      },
      yAxis: {
        type: "value",
        position: rtl ? "right" : "left",
        axisLabel: { ...axis, formatter: (v: number) => formatSize(v, lang) },
        splitLine: { lineStyle: { color: cssVar("--border") } },
      },
      series: [
        {
          type: "line",
          smooth: true,
          symbolSize: 7,
          data: pts.map((p) => p.bytes),
          lineStyle: { width: 3, color: accent },
          itemStyle: { color: accent },
          areaStyle: { color: cssVar("--accent-soft") },
        },
      ],
    };
  });
</script>

{#if points == null}
  <Skeleton height={260} radius="var(--r-lg)" />
{:else if points.length < 2}
  <EmptyState compact icon="growth" title={t("growth.notEnough")} />
{:else}
  <EChart {option} height="260px" label={t("growth.chartLabel", { path })} />
{/if}
