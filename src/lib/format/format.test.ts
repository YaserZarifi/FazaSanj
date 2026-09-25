import { describe, expect, it } from "vitest";
import {
  formatClock,
  formatCompact,
  formatDate,
  formatDuration,
  formatNumber,
  formatPercent,
  formatRelative,
  formatSize,
  formatSizeDelta,
  toPersianDigits,
  truncateMiddle,
} from "./index";

const GB = 1024 ** 3;
const MB = 1024 ** 2;

// Intl may add bidi marks around signs and percent; tests compare the visible text
const clean = (s: string) => s.replace(/[\u200e\u200f\u061c]/g, "").replace(/\s/g, " ");

describe("formatSize", () => {
  it("formats english with 1024 base", () => {
    expect(formatSize(23.4 * GB, "en")).toBe("23.4 GB");
    expect(formatSize(476 * GB, "en")).toBe("476 GB");
    expect(formatSize(512, "en")).toBe("512 B");
    expect(formatSize(0, "en")).toBe("0 B");
    expect(formatSize(1536, "en")).toBe("1.5 KB");
    expect(formatSize(12 * MB, "en")).toBe("12 MB");
    expect(formatSize(2.5 * 1024 * GB, "en")).toBe("2.5 TB");
  });

  it("formats persian with persian digits and decimal separator", () => {
    expect(formatSize(23.4 * GB, "fa")).toBe("۲۳٫۴ گیگابایت");
    expect(formatSize(476 * GB, "fa")).toBe("۴۷۶ گیگابایت");
    expect(formatSize(300, "fa")).toBe("۳۰۰ بایت");
    expect(formatSize(5 * 1024, "fa")).toBe("۵ کیلوبایت");
    expect(formatSize(700 * MB, "fa")).toBe("۷۰۰ مگابایت");
    expect(formatSize(1.2 * 1024 * GB, "fa")).toBe("۱٫۲ ترابایت");
  });

  it("rolls over to the next unit after rounding", () => {
    expect(formatSize(1024 * 1024 - 10, "en")).toBe("1 MB");
  });

  it("formats deltas with a sign", () => {
    expect(formatSizeDelta(8 * GB, "en")).toBe("+8 GB");
    expect(formatSizeDelta(-2.5 * GB, "en")).toBe("\u22122.5 GB");
    expect(formatSizeDelta(0, "en")).toBe("0 B");
  });
});

describe("numbers", () => {
  it("formats plain numbers", () => {
    expect(formatNumber(1234567, "en")).toBe("1,234,567");
    expect(formatNumber(1234567, "fa")).toBe("۱٬۲۳۴٬۵۶۷");
    expect(formatNumber(4.12, "fa", 1)).toBe("۴٫۱");
  });

  it("formats compact counts", () => {
    expect(formatCompact(1_234_567, "en")).toBe("1.2M");
    expect(clean(formatCompact(1_234_567, "fa"))).toBe("۱٫۲ میلیون");
    expect(formatCompact(950, "en")).toBe("950");
  });

  it("formats percents", () => {
    expect(clean(formatPercent(0.234, "en"))).toBe("23%");
    expect(clean(formatPercent(0.234, "fa"))).toBe("۲۳٪");
    expect(clean(formatPercent(0.004, "en"))).toBe("0.4%");
    expect(clean(formatPercent(Number.NaN, "en"))).toBe("0%");
  });

  it("converts digits", () => {
    expect(toPersianDigits("C: 2024")).toBe("C: ۲۰۲۴");
  });
});

describe("dates", () => {
  const d = new Date(2026, 8, 25, 14, 5).getTime();

  it("uses the jalali calendar in persian", () => {
    expect(formatDate(d, "fa")).toBe("۳ مهر ۱۴۰۵");
  });

  it("uses gregorian in english", () => {
    expect(formatDate(d, "en")).toBe("September 25, 2026");
  });

  it("handles null", () => {
    expect(formatDate(null, "en")).toBe("");
  });

  it("formats relative times", () => {
    const now = d;
    const day = 86_400_000;
    expect(formatRelative(now - 90 * day, "en", now)).toBe("3 months ago");
    expect(formatRelative(now - 90 * day, "fa", now)).toBe("۳ ماه پیش");
    expect(formatRelative(now - day, "en", now)).toBe("yesterday");
    expect(formatRelative(now - 5000, "en", now)).toBe("now");
    expect(formatRelative(now - 400 * day, "en", now)).toBe("last year");
  });
});

describe("durations", () => {
  it("formats short durations", () => {
    expect(formatDuration(4100, "en")).toBe("4.1s");
    expect(formatDuration(4100, "fa")).toBe("۴٫۱ ثانیه");
    expect(formatDuration(350, "en")).toBe("350ms");
  });

  it("formats minutes", () => {
    expect(formatDuration(125_000, "en")).toBe("2m 5s");
    expect(formatDuration(125_000, "fa")).toBe("۲ دقیقه و ۵ ثانیه");
    expect(formatDuration(3_720_000, "en")).toBe("1h 2m");
  });

  it("formats a clock", () => {
    expect(formatClock(67_000, "en")).toBe("1:07");
    expect(formatClock(67_000, "fa")).toBe("۱:۰۷");
  });
});

describe("truncateMiddle", () => {
  it("keeps short strings", () => {
    expect(truncateMiddle("C:\\Windows", 20)).toBe("C:\\Windows");
  });

  it("cuts long strings in the middle", () => {
    const s = truncateMiddle("C:\\Users\\Ali\\AppData\\Local\\Google\\Chrome\\User Data\\Default\\Cache", 30);
    expect(s.length).toBe(30);
    expect(s.startsWith("C:\\Users")).toBe(true);
    expect(s.endsWith("Cache")).toBe(true);
    expect(s).toContain("…");
  });
});
