import type { Language } from "../../lib/api/types";
import { cssVar } from "../../lib/theme";

export const FONT = "Vazirmatn, Segoe UI, sans-serif";

export function esc(s: string): string {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c] ?? c);
}

/** Shared tooltip look. Direction follows the UI language so Persian lines read right to left. */
export function tooltipStyle(lang: Language) {
  return {
    backgroundColor: cssVar("--surface"),
    borderColor: cssVar("--border"),
    borderWidth: 1,
    padding: [10, 12],
    textStyle: { color: cssVar("--text"), fontFamily: FONT, fontSize: 13 },
    extraCssText: `direction:${lang === "fa" ? "rtl" : "ltr"};text-align:start;border-radius:10px;box-shadow:0 8px 24px rgba(0,0,0,.14);`,
  };
}

/** Simple HTML block for tooltips: a bold title and label/value rows. */
export function tooltipHtml(title: string, rows: [string, string][], color?: string): string {
  const dot = color ? `<span style="display:inline-block;width:9px;height:9px;border-radius:3px;background:${color};margin-inline-end:6px"></span>` : "";
  const body = rows
    .map(([k, v]) => `<div style="display:flex;justify-content:space-between;gap:18px"><span style="opacity:.7">${esc(k)}</span><b style="font-weight:500">${esc(v)}</b></div>`)
    .join("");
  return `<div style="min-width:180px"><div style="font-weight:700;margin-bottom:6px;max-width:320px;overflow:hidden;text-overflow:ellipsis">${dot}${esc(title)}</div>${body}</div>`;
}
