<script lang="ts">
  import { onMount } from "svelte";
  import { echarts, type ChartOption } from "./echarts";

  export interface ChartClick {
    data: unknown;
    name: string;
    treePathInfo?: { name: string; value: unknown }[];
  }

  interface Props {
    option: ChartOption;
    height?: string;
    label: string;
    onclick?: (p: ChartClick) => void;
  }

  let { option, height = "420px", label, onclick }: Props = $props();
  let el = $state<HTMLDivElement>();
  let chart: ReturnType<typeof echarts.init> | null = null;

  onMount(() => {
    if (!el) return;
    chart = echarts.init(el, undefined, { renderer: "canvas" });
    chart.on("click", (p) => onclick?.(p as unknown as ChartClick));
    const ro = new ResizeObserver(() => chart?.resize());
    ro.observe(el);
    return () => {
      ro.disconnect();
      chart?.dispose();
      chart = null;
    };
  });

  // declared after onMount, so the chart exists by the first run
  $effect(() => {
    const o = option;
    chart?.setOption(o, { notMerge: true });
  });
</script>

<div class="chart" bind:this={el} style:height role="img" aria-label={label}></div>

<style>
  .chart {
    width: 100%;
  }
</style>
