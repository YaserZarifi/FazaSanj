import type { Language } from "../api/types";

const LOCALE: Record<Language, string> = { fa: "fa-IR", en: "en-US" };

const SIZE_UNITS: Record<Language, string[]> = {
  en: ["B", "KB", "MB", "GB", "TB", "PB"],
  fa: ["بایت", "کیلوبایت", "مگابایت", "گیگابایت", "ترابایت", "پتابایت"],
};

const numberCache = new Map<string, Intl.NumberFormat>();

function nf(lang: Language, opts: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = lang + JSON.stringify(opts);
  let f = numberCache.get(key);
  if (!f) {
    f = new Intl.NumberFormat(LOCALE[lang], opts);
    numberCache.set(key, f);
  }
  return f;
}

const FA_DIGITS = "۰۱۲۳۴۵۶۷۸۹";

/** Replaces ASCII digits with Persian ones. Useful for strings we build ourselves. */
export function toPersianDigits(s: string): string {
  return s.replace(/[0-9]/g, (d) => FA_DIGITS[Number(d)]);
}

export function localizeDigits(s: string, lang: Language): string {
  return lang === "fa" ? toPersianDigits(s) : s;
}

export function formatNumber(n: number, lang: Language, maxFractionDigits = 0): string {
  return nf(lang, { maximumFractionDigits: maxFractionDigits }).format(n);
}

/** "1.2M" / "۱٫۲ میلیون" */
export function formatCompact(n: number, lang: Language): string {
  if (Math.abs(n) < 10_000) return formatNumber(n, lang);
  return nf(lang, { notation: "compact", maximumFractionDigits: 1 }).format(n);
}

/** Binary units (1024), shown with the familiar GB/MB labels the way Windows does. */
export function formatSize(bytes: number, lang: Language): string {
  const units = SIZE_UNITS[lang];
  const sign = bytes < 0 ? -1 : 1;
  let v = Math.abs(bytes);
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  let digits = i === 0 || v >= 100 ? 0 : 1;
  let rounded = Number(v.toFixed(digits));
  // 1023.96 KB should read as 1 MB, not 1024 KB
  if (rounded >= 1024 && i < units.length - 1) {
    i++;
    v = rounded / 1024;
    digits = v >= 100 ? 0 : 1;
    rounded = Number(v.toFixed(digits));
  }
  return `${formatNumber(sign * rounded, lang, digits)} ${units[i]}`;
}

/** Signed size for growth deltas: "+8.2 GB" / "‎+۸٫۲ گیگابایت". */
export function formatSizeDelta(bytes: number, lang: Language): string {
  if (bytes === 0) return formatSize(0, lang);
  const s = formatSize(Math.abs(bytes), lang);
  return bytes > 0 ? `+${s}` : `\u2212${s}`;
}

/** ratio 0..1 */
export function formatPercent(ratio: number, lang: Language): string {
  if (!Number.isFinite(ratio)) ratio = 0;
  const digits = ratio > 0 && ratio < 0.01 ? 1 : 0;
  return nf(lang, { style: "percent", maximumFractionDigits: digits }).format(ratio);
}

const dateCache = new Map<string, Intl.DateTimeFormat>();

function dtf(lang: Language, withTime: boolean): Intl.DateTimeFormat {
  const key = `${lang}${withTime}`;
  let f = dateCache.get(key);
  if (!f) {
    const locale = lang === "fa" ? "fa-IR-u-ca-persian-nu-arabext" : "en-US";
    f = new Intl.DateTimeFormat(locale, {
      year: "numeric",
      month: withTime ? "short" : "long",
      day: "numeric",
      ...(withTime ? { hour: "2-digit", minute: "2-digit", hour12: false } : {}),
    });
    dateCache.set(key, f);
  }
  return f;
}

/** fa: Jalali calendar with Persian digits. en: Gregorian. */
export function formatDate(ms: number | null, lang: Language, withTime = false): string {
  if (ms == null) return "";
  return dtf(lang, withTime).format(new Date(ms));
}

const RELATIVE_STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["second", 60],
  ["minute", 60],
  ["hour", 24],
  ["day", 30],
  ["month", 12],
  ["year", Number.POSITIVE_INFINITY],
];

/** "3 months ago" / "۳ ماه پیش" */
export function formatRelative(ms: number | null, lang: Language, now = Date.now()): string {
  if (ms == null) return "";
  const rtf = new Intl.RelativeTimeFormat(lang, { numeric: "auto" });
  let value = (ms - now) / 1000;
  if (Math.abs(value) < 45) return rtf.format(0, "second");
  for (const [unit, step] of RELATIVE_STEPS) {
    if (Math.abs(value) < step) return rtf.format(Math.round(value), unit);
    value /= step;
  }
  return "";
}

const DURATION_UNITS: Record<Language, { s: string; m: string; h: string; ms: string; and: string }> = {
  en: { s: "s", m: "m", h: "h", ms: "ms", and: " " },
  fa: { s: " ثانیه", m: " دقیقه", h: " ساعت", ms: " میلی‌ثانیه", and: " و " },
};

/** "4.1s" / "۴٫۱ ثانیه", "2m 5s" / "۲ دقیقه و ۵ ثانیه" */
export function formatDuration(ms: number, lang: Language): string {
  const u = DURATION_UNITS[lang];
  if (ms < 1000) return `${formatNumber(Math.round(ms), lang)}${u.ms}`;
  const secs = ms / 1000;
  if (secs < 60) return `${formatNumber(Math.round(secs * 10) / 10, lang, 1)}${u.s}`;
  const totalSec = Math.round(secs);
  const h = Math.floor(totalSec / 3600);
  const m = Math.floor((totalSec % 3600) / 60);
  const s = totalSec % 60;
  const parts: string[] = [];
  if (h) parts.push(`${formatNumber(h, lang)}${u.h}`);
  if (m) parts.push(`${formatNumber(m, lang)}${u.m}`);
  if (s && !h) parts.push(`${formatNumber(s, lang)}${u.s}`);
  return parts.join(u.and);
}

/** Elapsed clock for the scan screen: "0:07" / "۰:۰۷" */
export function formatClock(ms: number, lang: Language): string {
  const total = Math.floor(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return localizeDigits(`${m}:${String(s).padStart(2, "0")}`, lang);
}

/** Shortens long paths in the middle: "C:\Users\...\cache\file.bin" */
export function truncateMiddle(s: string, max: number): string {
  if (s.length <= max) return s;
  const keep = max - 1;
  const head = Math.ceil(keep * 0.4);
  const tail = keep - head;
  return `${s.slice(0, head)}…${s.slice(s.length - tail)}`;
}
