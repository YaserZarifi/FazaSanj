// Number, size, date and duration formatting for both languages.
// Persian uses Persian digits, the Persian decimal separator and the Jalali calendar.

export type Lang = "fa" | "en";

const FA_DIGITS = "۰۱۲۳۴۵۶۷۸۹";

/** Turns ASCII digits into Persian digits. Everything else is kept. */
export function toPersianDigits(s: string): string {
  return s.replace(/[0-9]/g, (d) => FA_DIGITS[Number(d)]);
}

/** Digits for the given language. */
export function digits(s: string | number, lang: Lang): string {
  const str = String(s);
  return lang === "fa" ? toPersianDigits(str) : str;
}

const intCache = new Map<Lang, Intl.NumberFormat>();

/** 1234567 -> "1,234,567" or "۱٬۲۳۴٬۵۶۷". */
export function formatInt(n: number, lang: Lang): string {
  let f = intCache.get(lang);
  if (!f) {
    f = new Intl.NumberFormat(lang === "fa" ? "fa-IR" : "en-US", { maximumFractionDigits: 0 });
    intCache.set(lang, f);
  }
  return f.format(Math.round(n));
}

/** A number with a fixed count of decimals, "23.4" or "۲۳٫۴". */
export function formatDecimal(n: number, decimals: number, lang: Lang): string {
  const s = n.toFixed(decimals);
  return lang === "fa" ? toPersianDigits(s).replace(".", "٫") : s;
}

const UNITS: Record<Lang, string[]> = {
  en: ["bytes", "KB", "MB", "GB", "TB", "PB"],
  fa: ["بایت", "کیلوبایت", "مگابایت", "گیگابایت", "ترابایت", "پتابایت"],
};

/** Splits a size into a number and a unit, for layouts that style them apart. */
export function sizeParts(bytes: number, lang: Lang): { value: string; unit: string } {
  const units = UNITS[lang];
  const b = Math.max(0, bytes || 0);
  if (b < 1024) {
    return { value: formatInt(b, lang), unit: units[0] };
  }
  let v = b;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const decimals = v >= 100 ? 0 : 1;
  return { value: formatDecimal(v, decimals, lang), unit: units[i] };
}

/** 25125558681 -> "23.4 GB" or "۲۳٫۴ گیگابایت". */
export function formatSize(bytes: number, lang: Lang): string {
  const { value, unit } = sizeParts(bytes, lang);
  return `${value} ${unit}`;
}

/** Signed size for growth views: "+1.2 GB", "−300 MB". */
export function formatDelta(bytes: number, lang: Lang): string {
  if (bytes === 0) return formatSize(0, lang);
  const sign = bytes > 0 ? "+" : "−";
  return `${sign}${formatSize(Math.abs(bytes), lang)}`;
}

/** 0.234 -> "23%" or "٪۲۳". Small but non zero shares show one decimal. */
export function formatPercent(share: number, lang: Lang): string {
  const p = Math.max(0, share) * 100;
  const s = p > 0 && p < 10 ? formatDecimal(p, 1, lang) : formatInt(p, lang);
  return lang === "fa" ? `٪${s}` : `${s}%`;
}

const dateCache = new Map<string, Intl.DateTimeFormat>();

function dateFormat(lang: Lang, withTime: boolean): Intl.DateTimeFormat {
  const key = `${lang}-${withTime}`;
  let f = dateCache.get(key);
  if (!f) {
    const locale = lang === "fa" ? "fa-IR-u-ca-persian" : "en-GB";
    f = new Intl.DateTimeFormat(locale, {
      year: "numeric",
      month: "long",
      day: "numeric",
      ...(withTime ? { hour: "2-digit", minute: "2-digit" } : {}),
    });
    dateCache.set(key, f);
  }
  return f;
}

/** Jalali date in Persian ("۴ مهر ۱۴۰۵"), Gregorian in English ("25 September 2026"). */
export function formatDate(ms: number | null | undefined, lang: Lang, withTime = false): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return "";
  return dateFormat(lang, withTime).format(new Date(ms));
}

const relCache = new Map<Lang, Intl.RelativeTimeFormat>();

/** "3 days ago" / "۳ روز پیش". */
export function formatRelative(ms: number | null | undefined, lang: Lang, now = Date.now()): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return "";
  let f = relCache.get(lang);
  if (!f) {
    f = new Intl.RelativeTimeFormat(lang === "fa" ? "fa-IR" : "en", { numeric: "auto" });
    relCache.set(lang, f);
  }
  const diff = (ms - now) / 1000;
  const abs = Math.abs(diff);
  const steps: [number, Intl.RelativeTimeFormatUnit][] = [
    [60, "second"],
    [3600, "minute"],
    [86400, "hour"],
    [86400 * 30, "day"],
    [86400 * 365, "month"],
  ];
  let unit: Intl.RelativeTimeFormatUnit = "year";
  let div = 86400 * 365;
  let prev = 1;
  for (const [limit, u] of steps) {
    if (abs < limit) {
      unit = u;
      div = prev;
      break;
    }
    prev = limit;
  }
  return f.format(Math.round(diff / div), unit);
}

/** Scan durations: "850 ms", "4.1 s", "2 min 5 s" (and Persian equivalents). */
export function formatDuration(ms: number, lang: Lang): string {
  const fa = lang === "fa";
  if (ms < 1000) return `${formatInt(ms, lang)} ${fa ? "میلی‌ثانیه" : "ms"}`;
  const s = ms / 1000;
  if (s < 60) return `${formatDecimal(s, 1, lang)} ${fa ? "ثانیه" : "s"}`;
  const min = Math.floor(s / 60);
  const rest = Math.round(s - min * 60);
  return fa ? `${formatInt(min, lang)} دقیقه و ${formatInt(rest, lang)} ثانیه` : `${min} min ${rest} s`;
}
