import type { ApiError, Bilingual, Category, Language, SafetyLevel } from "../api/types";
import { type Dict, type Params, hasKey, translate } from "./core";
import en from "./en.json";
import fa from "./fa.json";

const dicts: Record<Language, Dict> = { fa: fa as Dict, en: en as Dict };

class I18nState {
  lang = $state<Language>("fa");
  get dir(): "rtl" | "ltr" {
    return this.lang === "fa" ? "rtl" : "ltr";
  }
}

export const i18n = new I18nState();

export function setLanguage(lang: Language): void {
  i18n.lang = lang;
  const html = document.documentElement;
  html.lang = lang;
  html.dir = lang === "fa" ? "rtl" : "ltr";
}

/** Reactive: templates that call t() re-render when the language changes. */
export function t(key: string, params?: Params): string {
  return translate(dicts, i18n.lang, key, params);
}

/** Both languages of a key, for building Bilingual values on the UI side. */
export function both(key: string, params?: Params): Bilingual {
  return { fa: translate(dicts, "fa", key, params), en: translate(dicts, "en", key, params) };
}

/** Picks the current language from a backend Bilingual text. */
export function bi(text: Bilingual | null | undefined): string {
  if (!text) return "";
  return text[i18n.lang] || text.en || text.fa;
}

export function errorText(err: ApiError | null | undefined): string {
  if (!err) return t("errors.generic");
  const key = `errors.${err.code}`;
  return hasKey(dicts[i18n.lang], key) ? t(key) : t("errors.generic");
}

export const categoryName = (c: Category): string => t(`categories.${c}`);
export const safetyName = (s: SafetyLevel): string => t(`safety.${s}`);
