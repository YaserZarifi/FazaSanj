import type { Language } from "../api/types";
import { formatNumber } from "../format";

export type Params = Record<string, string | number>;
export type Dict = { [key: string]: string | Dict };

const pluralRules: Record<Language, Intl.PluralRules> = {
  fa: new Intl.PluralRules("fa"),
  en: new Intl.PluralRules("en"),
};

function lookup(dict: Dict, key: string): string | Dict | undefined {
  let cur: string | Dict | undefined = dict;
  for (const part of key.split(".")) {
    if (cur == null || typeof cur === "string") return undefined;
    cur = cur[part];
  }
  return cur;
}

function pick(entry: string | Dict | undefined, lang: Language, params?: Params): string | undefined {
  if (entry == null || typeof entry === "string") return entry;
  // plural forms: { zero?, one, other }
  const count = params?.count;
  if (typeof count !== "number") return typeof entry.other === "string" ? entry.other : undefined;
  if (count === 0 && typeof entry.zero === "string") return entry.zero;
  const form = pluralRules[lang].select(count);
  const v = entry[form] ?? entry.other;
  return typeof v === "string" ? v : undefined;
}

function interpolate(s: string, lang: Language, params?: Params): string {
  if (!params) return s;
  return s.replace(/\{(\w+)\}/g, (m, name: string) => {
    const v = params[name];
    if (v == null) return m;
    return typeof v === "number" ? formatNumber(v, lang, 1) : v;
  });
}

/** Looks a key up in the active dictionary, falls back to the other language, then to the key. */
export function translate(dicts: Record<Language, Dict>, lang: Language, key: string, params?: Params): string {
  let s = pick(lookup(dicts[lang], key), lang, params);
  if (s == null) {
    const other: Language = lang === "fa" ? "en" : "fa";
    s = pick(lookup(dicts[other], key), other, params);
  }
  if (s == null) return key;
  return interpolate(s, lang, params);
}

export function hasKey(dict: Dict, key: string): boolean {
  return lookup(dict, key) !== undefined;
}

/** All leaf keys, with plural objects counted as one key. */
export function flattenKeys(dict: Dict, prefix = ""): string[] {
  const out: string[] = [];
  for (const [k, v] of Object.entries(dict)) {
    const key = prefix ? `${prefix}.${k}` : k;
    if (typeof v === "string") out.push(key);
    else if ("other" in v && typeof v.other === "string") out.push(key);
    else out.push(...flattenKeys(v, key));
  }
  return out;
}
