<script lang="ts">
  import { backend } from "../../lib/api/client";
  import type { NodeId, TreemapNode } from "../../lib/api/types";
  import { formatCompact, formatDate, formatPercent, formatSize } from "../../lib/format";
  import { categoryName, i18n, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import { themeState } from "../../lib/stores/themeState.svelte";
  import { toasts } from "../../lib/stores/toasts.svelte";
  import { categoryColor, cssVar } from "../../lib/theme";
  import Breadcrumb from "../expert/Breadcrumb.svelte";
  import EmptyState from "../common/EmptyState.svelte";
  import Skeleton from "../common/Skeleton.svelte";
  import { FONT, tooltipHtml, tooltipStyle } from "./chartTheme";
  import EChart, { type ChartClick } from "./EChart.svelte";

  let { kind }: { kind: "treemap" | "sunburst" } = $props();

  // Small slices, fetched again on every drill-down, keep this smooth on huge scans.
  const DEPTH = 2;
  const MAX_ITEMS = $derived(kind === "treemap" ? 90 : 60);

  interface Crumb {
    id: NodeId;
    name: string;
  }

  let trail = $state<Crumb[]>([]);
  let data = $state<TreemapNode | null>(null);
  let loading = $state(false);
  let req = 0;

  // start from the folder open in the tree
  $effect(() => {
    void scan.version;
    const start = scan.trail;
    if (start.length) trail = start.map((n) => ({ id: n.id, name: n.name }));
  });

  $effect(() => {
    const cur = trail[trail.length - 1];
    if (!cur || scan.scanId == null) return;
    const id = scan.scanId;
    const my = ++req;
    loading = true;
    backend
      .getTreemap(id, cur.id, DEPTH, MAX_ITEMS)
      .then((d) => {
        if (my === req) data = d;
      })
      .catch((e: unknown) => toasts.error(e))
      .finally(() => {
        if (my === req) loading = false;
      });
  });

  interface Item {
    name: string;
    value: number;
    nid: NodeId;
    other: boolean;
    dir: boolean;
    files: number;
    modified: number | null;
    category: TreemapNode["category"];
    drillable: boolean;
    itemStyle: { color: string };
    children?: Item[];
  }

  function toItem(n: TreemapNode, otherLabel: string): Item {
    const color = n.isOther ? cssVar("--surface-3") : categoryColor(n.category);
    return {
      name: n.isOther ? otherLabel : n.name,
      value: n.size,
      nid: n.id,
      other: n.isOther,
      dir: n.isDir,
      files: n.fileCount,
      modified: n.modified,
      category: n.category,
      drillable: n.isDir && !n.isOther && n.id > 0,
      itemStyle: { color },
      children: n.children.length ? n.children.map((c) => toItem(c, otherLabel)) : undefined,
    };
  }

  const option = $derived.by(() => {
    void themeState.resolved;
    const lang = i18n.lang;
    if (!data) return {};
    const total = data.size || 1;
    const items = data.children.map((c) => toItem(c, t("chart.other")));
    const border = cssVar("--surface");
    const tooltip = {
      ...tooltipStyle(lang),
      formatter: (p: { data: Item }) => {
        const d = p.data;
        if (!d) return "";
        const rows: [string, string][] = [
          [t("chart.size"), formatSize(d.value, lang)],
          [t("chart.share"), formatPercent(d.value / total, lang)],
          [t("chart.files"), formatCompact(d.files, lang)],
        ];
        if (d.modified) rows.push([t("chart.modified"), formatDate(d.modified, lang)]);
        if (!d.other) rows.push([t("chart.category"), categoryName(d.category)]);
        return tooltipHtml(d.name, rows, d.itemStyle.color);
      },
    };
    if (kind === "treemap") {
      return {
        textStyle: { fontFamily: FONT },
        tooltip,
        series: [
          {
            type: "treemap",
            data: items,
            roam: false,
            nodeClick: false,
            breadcrumb: { show: false },
            width: "100%",
            height: "100%",
            top: 0,
            left: 0,
            animationDurationUpdate: 450,
            label: {
              show: true,
              fontFamily: FONT,
              fontSize: 12,
              color: "#fff",
              overflow: "truncate",
              formatter: (p: { data: Item }) => `${p.data.name}\n${formatSize(p.data.value, lang)}`,
            },
            upperLabel: {
              show: true,
              height: 24,
              fontFamily: FONT,
              fontSize: 12,
              fontWeight: 500,
              color: "#fff",
              formatter: (p: { data: Item }) => `${p.data.name}  ${formatSize(p.data.value, lang)}`,
            },
            levels: [
              { itemStyle: { borderColor: border, borderWidth: 3, gapWidth: 3, borderRadius: 6 } },
              { itemStyle: { borderColor: "rgba(0,0,0,0.12)", borderWidth: 1, gapWidth: 1, borderRadius: 3 }, colorSaturation: [0.35, 0.6] },
            ],
          },
        ],
      };
    }
    return {
      textStyle: { fontFamily: FONT },
      tooltip,
      series: [
        {
          type: "sunburst",
          data: items,
          radius: ["14%", "95%"],
          nodeClick: false,
          sort: undefined,
          itemStyle: { borderColor: border, borderWidth: 2, borderRadius: 4 },
          label: {
            fontFamily: FONT,
            fontSize: 11,
            color: "#fff",
            minAngle: 12,
            rotate: "radial",
            overflow: "truncate",
            width: 90,
          },
          levels: [{}, { r0: "14%", r: "62%" }, { r0: "62%", r: "95%", label: { show: false } }],
        },
      ],
    };
  });

  function onclick(p: ChartClick) {
    const d = p.data as Item | undefined;
    if (!d || d.other) return;
    void scan.select(d.nid);
    if (!d.drillable) return;
    // clicking something two levels down drills into its group first, then the item
    const top = data?.children.find((c) => c.id === d.nid || c.children.some((g) => g.id === d.nid));
    const crumbs: Crumb[] = [];
    if (top && top.id !== d.nid) crumbs.push({ id: top.id, name: top.name });
    crumbs.push({ id: d.nid, name: d.name });
    trail = [...trail, ...crumbs];
  }

  function pickCrumb(_id: NodeId, index: number) {
    trail = trail.slice(0, index + 1);
  }
</script>

<div class="wrap">
  <div class="bar">
    <Breadcrumb items={trail} onpick={pickCrumb} />
    <p class="faint hint">{t("chart.hint")}</p>
  </div>
  {#if !data && loading}
    <Skeleton height={480} radius="var(--r-lg)" />
  {:else if data && data.children.length === 0}
    <EmptyState compact icon={kind === "treemap" ? "treemap" : "sunburst"} title={t("chart.empty")} />
  {:else if data}
    <div class="chart" class:busy={loading}>
      <EChart {option} {onclick} height="max(440px, calc(100vh - 300px))" label={t(kind === "treemap" ? "chart.treemapLabel" : "chart.sunburstLabel", { name: trail[trail.length - 1]?.name ?? "" })} />
    </div>
  {/if}
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sp-3);
    flex-wrap: wrap;
  }

  .hint {
    font-size: var(--fs-xs);
  }

  .chart {
    min-height: 420px;
    transition: opacity var(--dur-med) var(--ease);
  }

  .busy {
    opacity: 0.6;
  }
</style>
