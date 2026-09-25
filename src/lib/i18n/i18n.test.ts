import { describe, expect, it } from "vitest";
import { type Dict, flattenKeys, hasKey, translate } from "./core";
import en from "./en.json";
import fa from "./fa.json";

const dicts = { fa: fa as Dict, en: en as Dict };

// every source file as text, loaded by vite so the test needs no node types
const sources = import.meta.glob<string>(["/src/**/*.svelte", "/src/**/*.ts", "!/src/**/*.test.ts"], {
  query: "?raw",
  import: "default",
  eager: true,
});

describe("dictionaries", () => {
  it("have the same keys in both languages", () => {
    const a = flattenKeys(dicts.fa).sort();
    const b = flattenKeys(dicts.en).sort();
    expect(a.filter((k) => !b.includes(k))).toEqual([]);
    expect(b.filter((k) => !a.includes(k))).toEqual([]);
  });

  it("contain every static key used in the source", () => {
    const missing: string[] = [];
    expect(Object.keys(sources).length).toBeGreaterThan(20);
    for (const [file, text] of Object.entries(sources)) {
      for (const m of text.matchAll(/\bt\("([a-zA-Z0-9_.]+)"/g)) {
        if (!hasKey(dicts.en, m[1]) || !hasKey(dicts.fa, m[1])) missing.push(`${file}: ${m[1]}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("translate every backend error code the ui expects", () => {
    const codes = [
      "drives_failed", "scan_failed", "cancelled", "not_found", "uac_refused", "not_ntfs", "helper_failed", "timeout",
      "blocked_invalid_path", "blocked_drive_root", "blocked_protected", "blocked_contains_protected", "blocked_profile_root",
      "blocked_registry_hive", "blocked_system_managed", "outside_sandbox", "sandbox_not_configured", "in_use", "vhdx_in_use",
      "no_internet", "invalid_key", "rate_limited", "provider_error", "invalid_response", "generic",
    ];
    for (const c of codes) {
      expect(hasKey(dicts.fa, `errors.${c}`), c).toBe(true);
      expect(hasKey(dicts.en, `errors.${c}`), c).toBe(true);
    }
  });

  it("covers every enum used in dynamic keys", () => {
    const enums: Record<string, string[]> = {
      categories: ["system", "apps", "games", "media", "dev", "cache", "user_files", "virtualization", "messaging", "browsers", "unknown"],
      storyCategories: ["system", "apps", "games", "media", "dev", "cache", "user_files", "virtualization", "messaging", "browsers", "unknown"],
      safety: ["safe", "probably_safe", "careful", "do_not_touch"],
      methods: ["recycle", "delete_contents", "command", "open_app_setting", "compact_vhdx", "manual_only"],
      status: ["done", "dry_run", "partial", "skipped_in_use", "blocked", "failed", "needs_manual", "opened_setting"],
      fallback: ["uac_refused", "not_ntfs", "helper_failed", "timeout"],
      "types.groups": ["video", "images", "audio", "archives", "installers", "disk_images", "documents", "code", "executables", "system", "other"],
      "aiSettings.providers": ["open_ai", "gemini", "anthropic", "groq"],
      nav: ["home", "cleanup", "history", "growth", "settings", "about", "heuristics"],
    };
    for (const [prefix, values] of Object.entries(enums)) {
      for (const v of values) expect(hasKey(dicts.fa, `${prefix}.${v}`), `${prefix}.${v}`).toBe(true);
    }
  });
});

describe("translate", () => {
  it("interpolates with localized digits", () => {
    expect(translate(dicts, "fa", "tree.loadMore", { shown: 100, total: 640 })).toBe("نمایش بیشتر (۱۰۰ از ۶۴۰)");
    expect(translate(dicts, "en", "tree.loadMore", { shown: 100, total: 640 })).toBe("Show more (100 of 640)");
  });

  it("picks plural forms", () => {
    expect(translate(dicts, "en", "tree.count", { count: 1 })).toBe("1 item");
    expect(translate(dicts, "en", "tree.count", { count: 5 })).toBe("5 items");
    expect(translate(dicts, "fa", "tree.count", { count: 5 })).toBe("۵ مورد");
    expect(translate(dicts, "en", "heur.found", { count: 0, size: "0 B" })).toBe("Nothing found");
  });

  it("falls back to the key when missing", () => {
    expect(translate(dicts, "en", "nope.missing")).toBe("nope.missing");
  });
});
