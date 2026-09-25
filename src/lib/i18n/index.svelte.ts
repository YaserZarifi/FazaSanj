// Tiny i18n: flat keys in fa.json / en.json, {name} placeholders, instant switching.

import type { Bilingual } from "../api/types";
import { digits, type Lang } from "../format";
import en from "./en.json";
import fa from "./fa.json";

type Dict = Record<string, string>;
const dicts: Record<Lang, Dict> = { fa: fa as Dict, en: en as Dict };

export const i18n = $state<{ lang: Lang }>({ lang: "fa" });

/** Current language (reactive when read inside components). */
export function lang(): Lang {
  return i18n.lang;
}

/** Sets the language and flips the document direction. */
export function setLang(l: Lang) {
  i18n.lang = l;
  if (typeof document !== "undefined") {
    document.documentElement.lang = l;
    document.documentElement.dir = l === "fa" ? "rtl" : "ltr";
  }
}

/**
 * Translates a key. Numbers in params get the right digits; strings are used as given
 * (format sizes and dates before passing them). Missing keys fall back to English, then the key.
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const l = i18n.lang;
  let s = dicts[l][key] ?? dicts.en[key] ?? key;
  if (params) {
    s = s.replace(/\{(\w+)\}/g, (m, name: string) => {
      const v = params[name];
      if (v === undefined) return m;
      return typeof v === "number" ? digits(v, l) : v;
    });
  }
  return s;
}

/** Picks the current language from a bilingual rule text. */
export function bi(b: Bilingual | null | undefined): string {
  if (!b) return "";
  return (i18n.lang === "fa" ? b.fa : b.en) || b.en || b.fa;
}

/** Translated error text for an ApiError-like value. */
export function errorText(e: unknown): string {
  const code = typeof e === "object" && e !== null && "code" in e ? String((e as { code: unknown }).code) : "";
  if (code && (dicts.en[`errors.${code}`] !== undefined || dicts.fa[`errors.${code}`] !== undefined)) {
    return t(`errors.${code}`);
  }
  if (code.startsWith("blocked_")) return t("errors.blocked");
  return t("errors.unknown");
}

export const allKeys = { fa: Object.keys(fa), en: Object.keys(en) };
