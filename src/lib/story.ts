import type { Language, Reason, SnapshotComparison, Story } from "./api/types";
import { formatSize, formatSizeDelta } from "./format";
import { bi, t } from "./i18n/index.svelte";

function list(items: string[], lang: Language): string {
  return new Intl.ListFormat(lang, { style: "long", type: "conjunction" }).format(items);
}

function driveLetter(root: string): string | null {
  return /^[A-Za-z]:\\?$/.test(root) ? root.slice(0, 2).toUpperCase() : null;
}

function reasonText(r: Reason, lang: Language): string {
  return t("story.item", { name: bi(r.explanation.title), size: formatSize(r.bytes, lang) });
}

/** Builds the plain-language paragraph for Simple mode, one sentence per entry. */
export function storySentences(s: Story, lang: Language): string[] {
  const out: string[] = [];
  const letter = driveLetter(s.rootPath);
  const used = s.driveTotal - s.driveFree;
  if (letter) {
    out.push(t("story.intro", { drive: letter, total: formatSize(s.driveTotal, lang), used: formatSize(used, lang), free: formatSize(s.driveFree, lang) }));
  } else {
    out.push(t("story.introFolder", { size: formatSize(s.countedBytes, lang) }));
  }

  const buckets = s.buckets.filter((b) => b.bytes > 0).slice(0, 3);
  if (buckets.length) {
    const parts = buckets.map((b) => t("story.item", { name: t(`storyCategories.${b.category}`), size: formatSize(b.bytes, lang) }));
    out.push(t("story.buckets", { list: list(parts, lang) }));
  }

  const top = s.reasons.slice(0, 3);
  if (top.length) out.push(t("story.biggest", { list: list(top.map((r) => reasonText(r, lang)), lang) }));

  if (s.safeBytes > 0) out.push(t("story.safe", { size: formatSize(s.safeBytes, lang) }));
  else out.push(t("story.noSafe"));

  if (s.needsDecision.length) out.push(t("story.decide", { count: s.needsDecision.length }));
  return out;
}

/** "Since last scan, C: grew 12 GB. Biggest growth: Telegram cache +8 GB, ..." */
export function growthSentence(c: SnapshotComparison, lang: Language): string {
  const name = driveLetter(c.to.rootPath) ?? c.to.rootPath;
  const size = formatSize(Math.abs(c.totalDelta), lang);
  const first =
    c.totalDelta > 0
      ? t("growth.grew", { name, size })
      : c.totalDelta < 0
        ? t("growth.shrank", { name, size })
        : t("growth.same", { name });
  const growing = c.items.filter((i) => i.delta > 0).slice(0, 3);
  if (!growing.length) return first;
  const parts = growing.map((i) => {
    const label = i.explanation ? bi(i.explanation.title) : lastSegment(i.path);
    return `${label} ${formatSizeDelta(i.delta, lang)}`;
  });
  return `${first} ${t("growth.biggest", { list: list(parts, lang) })}`;
}

export function lastSegment(path: string): string {
  const parts = path.split("\\").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}
