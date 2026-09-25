<script lang="ts">
  // Generic ECharts host: resizes with its box and re-renders when the option changes.
  import type { EChartsCoreOption, ECElementEvent } from "echarts/core";
  import { echarts } from "../../lib/charts";

  let {
    option,
    onclick,
    height = "100%",
    label,
  }: {
    option: EChartsCoreOption;
    onclick?: (e: ECElementEvent) => void;
    height?: string;
    label: string;
  } = $props();

  let el: HTMLDivElement | undefined = $state();
  let chart: ReturnType<typeof echarts.init> | null = null;

  $effect(() => {
    if (!el) return;
    chart = echarts.init(el, undefined, { renderer: "canvas" });
    const ro = new ResizeObserver(() => chart?.resize());
    ro.observe(el);
    return () => {
      ro.disconnect();
      chart?.dispose();
      chart = null;
    };
  });

  $effect(() => {
    const opt = option;
    chart?.setOption(opt, { notMerge: true, lazyUpdate: true });
  });

  $effect(() => {
    const handler = onclick;
    if (!chart || !handler) return;
    const c = chart;
    c.on("click", handler as (e: unknown) => void);
    return () => c.off("click");
  });
</script>

<div class="chart" bind:this={el} style:height role="img" aria-label={label}></div>

<style>
  .chart {
    width: 100%;
    min-height: 240px;
  }
</style>
