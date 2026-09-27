import { describe, expect, it } from "vitest";
import type { Reason, Story } from "./api/types";
import { joinList, storySentences, topReasons } from "./story";

const GB = 1024 ** 3;
const t = (k: string, p?: Record<string, string | number>) => `${k}${p ? JSON.stringify(p) : ""}`;

function reason(ruleId: string, bytes: number): Reason {
  return {
    nodeId: 1,
    path: "C:\\x",
    bytes,
    category: "cache",
    explanation: {
      ruleId,
      source: "knowledge_base",
      title: { fa: "", en: "" },
      whyBig: { fa: "", en: "" },
      ifDeleted: { fa: "", en: "" },
      safety: "safe",
      method: "recycle",
      needsAdmin: false,
      instructions: null,
      confidence: null,
    },
  };
}

const story: Story = {
  scanId: 1,
  rootPath: "C:\\",
  driveTotal: 500 * GB,
  driveFree: 100 * GB,
  countedBytes: 390 * GB,
  buckets: [
    { category: "system", bytes: 90 * GB },
    { category: "unknown", bytes: 80 * GB },
    { category: "games", bytes: 70 * GB },
  ],
  reasons: [reason("a", 5 * GB), reason("a", 4 * GB), reason("b", 3 * GB)],
  safeItems: [],
  safeBytes: 12 * GB,
  needsDecision: [],
};

describe("story", () => {
  it("joins lists per language", () => {
    expect(joinList(["a", "b", "c"], "en")).toBe("a, b and c");
    expect(joinList(["a", "b"], "fa")).toBe("a و b");
  });

  it("skips unknown buckets and mentions what can be freed", () => {
    const s = storySentences(story, t, "en", true);
    expect(s[0]).toContain("400 GB");
    expect(s[1]).not.toContain("category.unknown");
    expect(s[2]).toContain("story.canFree");
  });

  it("keeps one reason per rule", () => {
    expect(topReasons(story).map((r) => r.explanation.ruleId)).toEqual(["a", "b"]);
  });
});
