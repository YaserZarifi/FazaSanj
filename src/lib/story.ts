// Builds the plain-language story sentence for Simple mode.

import type { Reason, Story } from "./api/types";
import type { Lang } from "./format";
import { formatSize } from "./format";
import { categoryKey } from "./labels";

type T = (key: string, params?: Record<string, string | number>) => string;

/** Joins "a", "b" and "c" the way each language does. */
export function joinList(items: string[], lang: Lang): string {
  if (items.length <= 1) return items.join("");
  const last = items[items.length - 1];
  const head = items.slice(0, -1).join(lang === "fa" ? "، " : ", ");
  return lang === "fa" ? `${head} و ${last}` : `${head} and ${last}`;
}

/** Up to three biggest known buckets as "Windows and system (95 GB)". */
export function topBuckets(story: Story, t: T, lang: Lang, count = 3): string[] {
  return story.buckets
    .filter((b) => b.category !== "unknown" && b.bytes > 0)
    .slice(0, count)
    .map((b) => t("story.bucket", { name: t(categoryKey(b.category)), size: formatSize(b.bytes, lang) }));
}

export function storySentences(story: Story, t: T, lang: Lang, isDrive: boolean): string[] {
  const out: string[] = [];
  const used = story.driveTotal > 0 && isDrive ? story.driveTotal - story.driveFree : story.countedBytes;
  out.push(isDrive ? t("story.driveHolds", { size: formatSize(used, lang) }) : t("story.folderHolds", { size: formatSize(story.countedBytes, lang) }));
  const buckets = topBuckets(story, t, lang);
  if (buckets.length) out.push(t("story.mostly", { list: joinList(buckets, lang) }));
  if (story.safeBytes >= 50 * 1024 * 1024) {
    out.push(t("story.canFree", { size: formatSize(story.safeBytes, lang) }));
  } else {
    out.push(t("story.littleToFree"));
  }
  if (story.needsDecision.length) {
    const bytes = story.needsDecision.reduce((a, r) => a + r.bytes, 0);
    out.push(t("story.decide", { size: formatSize(bytes, lang) }));
  }
  return out;
}

/** The five biggest explained items, one per rule. */
export function topReasons(story: Story, count = 5): Reason[] {
  const seen = new Set<string>();
  const out: Reason[] = [];
  for (const r of story.reasons) {
    if (seen.has(r.explanation.ruleId)) continue;
    seen.add(r.explanation.ruleId);
    out.push(r);
    if (out.length === count) break;
  }
  return out;
}
