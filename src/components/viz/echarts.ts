// Modular ECharts import keeps the bundle small: only the charts we draw.
import * as echarts from "echarts/core";
import { BarChart, LineChart, PieChart, SunburstChart, TreemapChart } from "echarts/charts";
import { GridComponent, TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";

echarts.use([TreemapChart, SunburstChart, PieChart, LineChart, BarChart, GridComponent, TooltipComponent, CanvasRenderer]);

export { echarts };
export type { EChartsCoreOption as ChartOption } from "echarts/core";
