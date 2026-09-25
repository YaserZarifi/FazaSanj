import { describe, expect, it } from "vitest";
import en from "./en.json";
import fa from "./fa.json";

const enKeys = Object.keys(en);
const faKeys = Object.keys(fa);

const files = import.meta.glob<string>(["/src/**/*.svelte", "/src/**/*.ts", "!/src/**/*.test.ts"], {
  query: "?raw",
  import: "default",
  eager: true,
});
const code = Object.values(files).join("\n");

// Keys built at runtime from a prefix and a value.
const DYNAMIC: Record<string, string[]> = {
  "category.": ["system", "apps", "games", "media", "dev", "cache", "user_files", "virtualization", "messaging", "browsers", "unknown"],
  "safety.": ["safe", "probably_safe", "careful", "do_not_touch"],
  "method.": ["recycle", "delete_contents", "command", "open_app_setting", "compact_vhdx", "manual_only"],
  "status.": ["done", "dry_run", "partial", "skipped_in_use", "blocked", "failed", "needs_manual", "opened_setting"],
  "fallback.": ["uac_refused", "not_ntfs", "helper_failed", "timeout"],
  "types.": ["video", "images", "audio", "archives", "installers", "disk_images", "documents", "code", "executables", "system", "other"],
  "findings.kind.": ["duplicates", "old_project", "orphan", "stale"],
  "expert.tab.": ["folders", "largest", "types", "findings", "denied", "compare"],
  "expert.look.": ["list", "treemap", "sunburst"],
  "nav.": ["history", "growth", "settings", "about"],
  "topbar.": ["simple", "expert"],
  "list.": ["size", "name", "modified"],
  "warning.": ["recycle_same_drive", "needs_admin", "system_action", "blocked", "outside_sandbox"],
};

describe("translations", () => {
  it("have the same keys in both languages", () => {
    expect(enKeys.filter((k) => !faKeys.includes(k))).toEqual([]);
    expect(faKeys.filter((k) => !enKeys.includes(k))).toEqual([]);
  });

  it("are never empty and keep the same placeholders", () => {
    for (const k of enKeys) {
      const e = (en as Record<string, string>)[k];
      const f = (fa as Record<string, string>)[k];
      expect(e.trim(), k).not.toBe("");
      expect(f.trim(), k).not.toBe("");
      const ph = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
      expect(ph(f), k).toEqual(ph(e));
    }
  });

  it("cover every key the code uses", () => {
    expect(Object.keys(files).length).toBeGreaterThan(30);
    const used = new Set<string>();
    for (const m of code.matchAll(/\bt\(\s*"([a-zA-Z0-9_.]+)"/g)) used.add(m[1]);
    for (const m of code.matchAll(/(?:key|title|body): "([a-z]+\.[a-zA-Z0-9_.]+)"/g)) used.add(m[1]);
    for (const [prefix, values] of Object.entries(DYNAMIC)) for (const v of values) used.add(prefix + v);
    const missing = [...used].filter((k) => !enKeys.includes(k));
    expect(missing).toEqual([]);
  });

  it("use Persian digits nowhere in English and no em dashes", () => {
    for (const k of enKeys) {
      expect((en as Record<string, string>)[k], k).not.toMatch(/[۰-۹—]/);
      expect((fa as Record<string, string>)[k], k).not.toMatch(/—/);
    }
  });

  it("write common Persian verb prefixes with a zero width non-joiner", () => {
    for (const k of faKeys) {
      const v = (fa as Record<string, string>)[k];
      // "می شود" with a plain space is a typo; it must be "می‌شود".
      expect(v, k).not.toMatch(/(^|\s)(می|نمی) (?=[؀-ۿ])/);
    }
  });
});
