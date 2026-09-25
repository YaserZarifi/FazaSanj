// Turns heuristic findings into cleanup targets. Heuristics never say "safe", and the backend caps
// them at "probably safe" again, so nothing found here can be deleted permanently.

import type { Bilingual, CleanupTarget, Explanation, HeuristicFinding, HeuristicKind } from "./api/types";

const TITLES: Record<HeuristicKind, Bilingual> = {
  orphan: { fa: "داده‌ی برنامه‌ای که دیگر نصب نیست", en: "Data from a program that is no longer installed" },
  stale: { fa: "فایل‌هایی که مدت‌هاست استفاده نشده‌اند", en: "Files nobody has used in a long time" },
  duplicates: { fa: "نسخه‌ی تکراری", en: "Duplicate copy" },
  old_project: { fa: "فایل‌های ساخت یک پروژه‌ی قدیمی", en: "Build files of an old project" },
};

const IF_DELETED: Record<HeuristicKind, Bilingual> = {
  orphan: {
    fa: "اگر برنامه را دوباره نصب کنید، تنظیمات و داده‌های قبلی‌اش را نخواهد داشت. پوشه به سطل بازیافت می‌رود و تا خالی کردن سطل قابل برگرداندن است.",
    en: "If you install the program again, it will start without its old settings and data. The folder goes to the Recycle Bin, so you can bring it back until you empty it.",
  },
  stale: {
    fa: "فایل‌ها به سطل بازیافت می‌روند. قبل از حذف مطمئن شوید نسخه‌ی دیگری از آن‌ها دارید یا دیگر لازمشان ندارید.",
    en: "The files go to the Recycle Bin. Make sure you have another copy or really don't need them anymore.",
  },
  duplicates: {
    fa: "یک نسخه نگه داشته می‌شود و بقیه به سطل بازیافت می‌روند. برنامه‌هایی که دقیقاً از این مسیر استفاده می‌کردند ممکن است فایل را پیدا نکنند.",
    en: "One copy is kept and the others go to the Recycle Bin. A program that used this exact path might not find the file anymore.",
  },
  old_project: {
    fa: "فقط پوشه‌هایی حذف می‌شوند که ابزار پروژه دوباره می‌سازد (مثل node_modules یا target). کد شما دست نمی‌خورد. دفعه‌ی بعد که پروژه را باز کنید، نصب یا ساخت دوباره کمی طول می‌کشد.",
    en: "Only folders the project's tools rebuild are removed (like node_modules or target). Your code is not touched. The next time you open the project, installing or building again takes a little while.",
  },
};

function explanation(f: HeuristicFinding): Explanation {
  return {
    ruleId: `heuristic-${f.kind}`,
    source: "heuristic",
    title: TITLES[f.kind],
    whyBig: f.reason,
    ifDeleted: IF_DELETED[f.kind],
    safety: f.safety,
    method: "recycle",
    needsAdmin: false,
    instructions: null,
    confidence: f.confidence,
  };
}

/** What cleaning up this finding would remove. */
export function findingTargets(f: HeuristicFinding): CleanupTarget[] {
  const e = explanation(f);
  const d = f.details;
  switch (d.kind) {
    case "duplicates":
      return d.files
        .filter((_, i) => i !== d.keepIndex)
        .map((file) => ({ path: file.path, bytes: d.fileSize, explanation: e }));
    case "old_project": {
      const n = Math.max(1, d.rebuildableDirs.length);
      return d.rebuildableDirs.map((path) => ({ path, bytes: Math.round(d.rebuildableBytes / n), explanation: e }));
    }
    default:
      return [{ path: f.path, bytes: f.bytes, explanation: e }];
  }
}

export const findingTitle = (kind: HeuristicKind) => TITLES[kind];
