// Only the ECharts parts we use, to keep the bundle small.

import { LineChart, SunburstChart, TreemapChart } from "echarts/charts";
import { GridComponent, TooltipComponent } from "echarts/components";
import * as echarts from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import type { TreemapNode } from "./api/types";
import { categoryColor } from "./labels";

echarts.use([TreemapChart, SunburstChart, LineChart, TooltipComponent, GridComponent, CanvasRenderer]);

export { echarts };

export interface VizDatum {
  name: string;
  value: number;
  id: number;
  isDir: boolean;
  isOther: boolean;
  fileCount: number;
  modified: number | null;
  category: TreemapNode["category"];
  itemStyle: { color: string };
  children?: VizDatum[];
}

/** Turns a backend treemap slice into ECharts data. `otherLabel` names the grouped leftovers. */
export function toVizData(node: TreemapNode, otherLabel: string): VizDatum[] {
  return node.children.map((c) => toDatum(c, otherLabel));
}

function toDatum(n: TreemapNode, otherLabel: string): VizDatum {
  const d: VizDatum = {
    name: n.isOther ? otherLabel : n.name,
    value: n.size,
    id: n.id,
    isDir: n.isDir,
    isOther: n.isOther,
    fileCount: n.fileCount,
    modified: n.modified,
    category: n.category,
    itemStyle: { color: n.isOther ? "#9aa3ae" : categoryColor(n.category) },
  };
  if (n.children.length) d.children = n.children.map((c) => toDatum(c, otherLabel));
  return d;
}

/** Reads a CSS variable from the document, for chart text colors. */
export function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}
