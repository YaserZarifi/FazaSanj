import { describe, expect, it } from "vitest";
import {
  formatDate,
  formatDelta,
  formatDuration,
  formatInt,
  formatPercent,
  formatSize,
  toPersianDigits,
} from "./index";

const GB = 1024 ** 3;

describe("sizes", () => {
  it("formats in English", () => {
    expect(formatSize(0, "en")).toBe("0 bytes");
    expect(formatSize(512, "en")).toBe("512 bytes");
    expect(formatSize(23.4 * GB, "en")).toBe("23.4 GB");
    expect(formatSize(250 * GB, "en")).toBe("250 GB");
  });

  it("formats in Persian with Persian digits and decimal sign", () => {
    expect(formatSize(23.4 * GB, "fa")).toBe("۲۳٫۴ گیگابایت");
    expect(formatSize(1024 * 1024, "fa")).toBe("۱٫۰ مگابایت");
  });

  it("signs deltas", () => {
    expect(formatDelta(2 * GB, "en")).toBe("+2.0 GB");
    expect(formatDelta(-2 * GB, "en")).toBe("−2.0 GB");
  });
});

describe("numbers", () => {
  it("uses Persian digits", () => {
    expect(toPersianDigits("C: 123")).toBe("C: ۱۲۳");
    expect(formatInt(1234, "fa")).toMatch(/^۱.۲۳۴$/);
    expect(formatInt(1234, "en")).toBe("1,234");
  });

  it("formats percents", () => {
    expect(formatPercent(0.234, "en")).toBe("23%");
    expect(formatPercent(0.052, "en")).toBe("5.2%");
    expect(formatPercent(0.5, "fa")).toBe("٪۵۰");
  });
});

describe("dates and durations", () => {
  it("uses the Jalali calendar in Persian", () => {
    // 2026-09-25 is 3 Mehr 1405.
    const ms = Date.UTC(2026, 8, 25, 12);
    expect(formatDate(ms, "fa")).toContain("۱۴۰۵");
    expect(formatDate(ms, "fa")).toContain("مهر");
    expect(formatDate(ms, "en")).toContain("2026");
    expect(formatDate(null, "en")).toBe("");
  });

  it("formats durations", () => {
    expect(formatDuration(850, "en")).toBe("850 ms");
    expect(formatDuration(4100, "en")).toBe("4.1 s");
    expect(formatDuration(125000, "en")).toBe("2 min 5 s");
    expect(formatDuration(4100, "fa")).toBe("۴٫۱ ثانیه");
  });
});
