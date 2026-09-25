<script lang="ts">
  // Treemap or sunburst of one folder. Clicking a folder drills in, anything else selects it.
  import type { ECElementEvent } from "echarts/core";
  import * as api from "../../lib/api/commands";
  import type { NodeId, ScanId, TreemapNode } from "../../lib/api/types";
  import { cssVar, toVizData, type VizDatum } from "../../lib/charts";
  import { formatDate, formatInt, formatPercent, formatSize } from "../../lib/format";
  import { errorText, lang, t } from "../../lib/i18n/index.svelte";
  import { categoryKey } from "../../lib/labels";
  import Spinner from "../common/Spinner.svelte";
  import Chart from "./Chart.svelte";

  let {
    scanId,
    nodeId,
    kind,
    onopen,
    onselect,
  }: {
    scanId: ScanId;
    nodeId: NodeId;
    kind: "treemap" | "sunburst";
    onopen: (id: NodeId) => void;
    onselect: (id: NodeId) => void;
  } = $props();

  let slice = $state<TreemapNode | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    const [s, n, k] = [scanId, nodeId, kind];
    error = null;
    api
      .getTreemap(s, n, k === "treemap" ? 2 : 3, k === "treemap" ? 80 : 30)
      .then((r) => (slice = r))
      .catch((e) => (error = errorText(e)));
  });

  function tooltip(d: VizDatum): string {
    const l = lang();
    const total = slice?.size || 1;
    const esc = (s: string) => s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c] ?? c);
    const lines = [
      `<b>${esc(d.name)}</b>`,
      `${formatSize(d.value, l)} · ${formatPercent(d.value / total, l)}`,
      t("viz.files", { count: formatInt(d.fileCount, l) }),
      t(categoryKey(d.category)),
    ];
    if (d.modified) lines.push(t("viz.modified", { date: formatDate(d.modified, l) }));
    return `<div dir="${l === "fa" ? "rtl" : "ltr"}" style="font-family: Vazirmatn, sans-serif">${lines.join("<br>")}</div>`;
  }

  const option = $derived.by(() => {
    if (!slice) return {};
    const data = toVizData(slice, t("viz.other"));
    const text = cssVar("--text");
    const surface = cssVar("--surface");
    const l = lang();
    const common = {
      tooltip: {
        formatter: (p: { data: VizDatum }) => (p.data ? tooltip(p.data) : ""),
        backgroundColor: surface,
        borderColor: cssVar("--border"),
        textStyle: { color: text },
      },
      textStyle: { fontFamily: "Vazirmatn, Segoe UI, sans-serif" },
      animationDurationUpdate: 300,
    };
    if (kind === "treemap") {
      return {
        ...common,
        series: [
          {
            type: "treemap",
            data,
            roam: false,
            nodeClick: false,
            breadcrumb: { show: false },
            width: "100%",
            height: "100%",
            top: 0,
            left: 0,
            leafDepth: 2,
            visibleMin: 300,
            label: {
              show: true,
              formatter: (p: { data: VizDatum }) => `${p.data.name}\n${formatSize(p.data.value, l)}`,
              color: "#fff",
              fontSize: 12,
              overflow: "truncate",
            },
            upperLabel: {
              show: true,
              height: 22,
              color: "#fff",
              formatter: (p: { data: VizDatum }) => `${p.data.name}  ${formatSize(p.data.value, l)}`,
            },
            itemStyle: { borderColor: surface, borderWidth: 1, gapWidth: 1 },
            levels: [
              { itemStyle: { borderWidth: 3, gapWidth: 3, borderColor: surface } },
              { itemStyle: { borderWidth: 1, gapWidth: 1, borderColorSaturation: 0.5 }, colorSaturation: [0.35, 0.6] },
            ],
          },
        ],
      };
    }
    return {
      ...common,
      series: [
        {
          type: "sunburst",
          data,
          radius: ["12%", "95%"],
          nodeClick: false,
          sort: undefined,
          itemStyle: { borderColor: surface, borderWidth: 1 },
          label: { rotate: "radial", color: "#fff", fontSize: 11, minAngle: 8, overflow: "truncate", width: 90 },
          levels: [{}, { r0: "12%", r: "45%" }, { r0: "45%", r: "72%" }, { r0: "72%", r: "95%", label: { show: false } }],
        },
      ],
    };
  });

  function onclick(e: ECElementEvent) {
    const d = e.data as VizDatum | undefined;
    if (!d || d.isOther) return;
    if (d.isDir) onopen(d.id);
    else onselect(d.id);
  }
</script>

<div class="viz">
  {#if error}
    <div class="notice error">{error}</div>
  {:else if !slice}
    <div class="center"><Spinner size={24} /></div>
  {:else if slice.children.length === 0}
    <div class="center faint">{t("viz.empty")}</div>
  {:else}
    <Chart {option} {onclick} label={t(kind === "treemap" ? "viz.treemap" : "viz.sunburst")} />
  {/if}
</div>

<style>
  .viz {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .center {
    flex: 1;
    display: grid;
    place-items: center;
  }
</style>
