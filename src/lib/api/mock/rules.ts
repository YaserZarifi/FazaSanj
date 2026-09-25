import type { Bilingual, CleanupMethod, Explanation, ExplanationSource, SafetyLevel } from "../types";

interface RuleDef {
  title: Bilingual;
  whyBig: Bilingual;
  ifDeleted: Bilingual;
  safety: SafetyLevel;
  method: CleanupMethod;
  needsAdmin?: boolean;
  instructions?: Bilingual;
  source?: ExplanationSource;
  confidence?: number;
}

const RULES: Record<string, RuleDef> = {
  "windows-winsxs": {
    title: { fa: "انبار اجزای ویندوز (WinSxS)", en: "Windows component store (WinSxS)" },
    whyBig: {
      fa: "ویندوز نسخه‌های قبلی فایل‌های سیستمی را اینجا نگه می‌دارد تا بتواند به‌روزرسانی‌ها را برگرداند. بخش زیادی از این فضا در واقع بین چند جا مشترک است.",
      en: "Windows keeps older versions of system files here so updates can be rolled back. Much of this space is shared with other folders, so it looks bigger than it is.",
    },
    ifDeleted: {
      fa: "اگر دستی حذفش کنید ویندوز خراب می‌شود. فقط با ابزار رسمی DISM می‌شود آن را کوچک کرد.",
      en: "Deleting it by hand breaks Windows. It can only be shrunk with the official DISM tool.",
    },
    safety: "careful",
    method: "command",
    needsAdmin: true,
  },
  "windows-installer-cache": {
    title: { fa: "حافظهٔ نصب‌کنندهٔ ویندوز", en: "Windows Installer cache" },
    whyBig: {
      fa: "هر برنامه‌ای که با MSI نصب شده، یک نسخه از فایل نصبش را اینجا گذاشته تا بعداً بتوانید تعمیر یا حذفش کنید.",
      en: "Every program installed with MSI leaves a copy of its installer here, so it can be repaired or uninstalled later.",
    },
    ifDeleted: {
      fa: "برنامه‌ها دیگر درست به‌روز یا حذف نمی‌شوند. به این پوشه دست نزنید.",
      en: "Programs can no longer update or uninstall properly. Leave this folder alone.",
    },
    safety: "do_not_touch",
    method: "manual_only",
  },
  "windows-update-downloads": {
    title: { fa: "دانلودهای به‌روزرسانی ویندوز", en: "Windows Update downloads" },
    whyBig: {
      fa: "فایل‌هایی که Windows Update دانلود کرده و بعد از نصب دیگر لازم نیستند.",
      en: "Files Windows Update downloaded. Once updates are installed they are no longer needed.",
    },
    ifDeleted: {
      fa: "اتفاقی نمی‌افتد. اگر لازم باشد ویندوز دوباره دانلودشان می‌کند.",
      en: "Nothing bad. Windows downloads them again if it ever needs them.",
    },
    safety: "safe",
    method: "delete_contents",
    needsAdmin: true,
  },
  "windows-temp": {
    title: { fa: "فایل‌های موقت ویندوز", en: "Windows temporary files" },
    whyBig: {
      fa: "برنامه‌ها و نصب‌کننده‌ها فایل‌های موقت می‌سازند و خیلی وقت‌ها یادشان می‌رود پاکشان کنند.",
      en: "Programs and installers create temporary files and often forget to clean them up.",
    },
    ifDeleted: {
      fa: "این پوشه را می‌توانید بی‌خطر پاک کنید. فایل‌هایی که در حال استفاده‌اند رد می‌شوند.",
      en: "Safe to clean. Files that are in use are skipped.",
    },
    safety: "safe",
    method: "delete_contents",
    needsAdmin: true,
  },
  "user-temp": {
    title: { fa: "فایل‌های موقت شما", en: "Your temporary files" },
    whyBig: {
      fa: "فایل‌های موقتی که برنامه‌ها هنگام کار ساخته‌اند و دیگر به آن‌ها نیازی ندارند.",
      en: "Temporary files programs made while working and no longer need.",
    },
    ifDeleted: {
      fa: "این پوشه را می‌توانید بی‌خطر پاک کنید. فایل‌های باز رد می‌شوند.",
      en: "Safe to clean. Open files are skipped.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  pagefile: {
    title: { fa: "فایل حافظهٔ مجازی", en: "Virtual memory file" },
    whyBig: {
      fa: "وقتی رم پر می‌شود، ویندوز بخشی از آن را در این فایل می‌گذارد. اندازه‌اش را خود ویندوز تعیین می‌کند.",
      en: "When RAM fills up, Windows moves part of it into this file. Windows picks its size.",
    },
    ifDeleted: {
      fa: "نمی‌شود حذفش کرد و نباید هم کرد. اگر واقعاً لازم است، از تنظیمات حافظهٔ مجازی اندازه‌اش را کم کنید.",
      en: "It can't and shouldn't be deleted. If you really need to, lower its size in the virtual memory settings.",
    },
    safety: "do_not_touch",
    method: "manual_only",
    instructions: {
      fa: "تنظیمات ← سیستم ← درباره ← تنظیمات پیشرفتهٔ سیستم ← کارایی ← پیشرفته ← حافظهٔ مجازی",
      en: "Settings, System, About, Advanced system settings, Performance, Advanced, Virtual memory",
    },
  },
  hiberfil: {
    title: { fa: "فایل خواب زمستانی", en: "Hibernation file" },
    whyBig: {
      fa: "ویندوز هنگام خواب زمستانی یا روشن‌شدن سریع، محتوای رم را اینجا ذخیره می‌کند. اندازه‌اش نزدیک به اندازهٔ رم است.",
      en: "Windows saves the contents of RAM here for hibernation and Fast Startup. It is close to the size of your RAM.",
    },
    ifDeleted: {
      fa: "اگر خواب زمستانی را خاموش کنید این فایل حذف می‌شود، ولی روشن‌شدن سریع ویندوز هم از کار می‌افتد.",
      en: "Turning hibernation off removes this file, but Fast Startup stops working too.",
    },
    safety: "careful",
    method: "command",
    needsAdmin: true,
  },
  "windows-old": {
    title: { fa: "نسخهٔ قبلی ویندوز", en: "Previous Windows installation" },
    whyBig: {
      fa: "بعد از ارتقای ویندوز، نسخهٔ قبلی اینجا نگه داشته می‌شود تا اگر خواستید برگردید.",
      en: "After a Windows upgrade, the old version is kept here in case you want to go back.",
    },
    ifDeleted: {
      fa: "دیگر نمی‌توانید به نسخهٔ قبلی ویندوز برگردید. اگر همه‌چیز خوب کار می‌کند، معمولاً مشکلی نیست.",
      en: "You can no longer go back to the previous Windows. If everything works fine, that is usually OK.",
    },
    safety: "careful",
    method: "command",
    needsAdmin: true,
  },
  "recycle-bin": {
    title: { fa: "سطل بازیافت", en: "Recycle Bin" },
    whyBig: {
      fa: "فایل‌هایی که حذف کرده‌اید هنوز اینجا هستند و فضا می‌گیرند.",
      en: "Files you deleted are still here and still take up space.",
    },
    ifDeleted: {
      fa: "بعد از خالی کردن، دیگر نمی‌توانید آن فایل‌ها را برگردانید.",
      en: "After emptying it, you can't restore those files anymore.",
    },
    safety: "probably_safe",
    method: "command",
  },
  "restore-points": {
    title: { fa: "نقطه‌های بازیابی سیستم", en: "System restore points" },
    whyBig: {
      fa: "ویندوز پیش از تغییرات مهم از سیستم نسخهٔ پشتیبان می‌گیرد تا بتوانید برگردید.",
      en: "Windows saves backups before big changes so you can go back.",
    },
    ifDeleted: {
      fa: "اگر مشکلی پیش بیاید، راه برگشت کمتری دارید. بهتر است فقط از تنظیمات حفاظت سیستم کمش کنید.",
      en: "If something goes wrong, you have fewer ways back. Only reduce it from System Protection settings.",
    },
    safety: "careful",
    method: "manual_only",
    needsAdmin: true,
    instructions: {
      fa: "کنترل پنل ← سیستم ← حفاظت سیستم ← پیکربندی ← کم کردن حداکثر فضا",
      en: "Control Panel, System, System Protection, Configure, lower Max Usage",
    },
  },
  "docker-vhdx": {
    title: { fa: "دیسک مجازی داکر", en: "Docker's virtual disk" },
    whyBig: {
      fa: "داکر همهٔ ایمیج‌ها و کانتینرها را در یک دیسک مجازی نگه می‌دارد. این فایل بزرگ می‌شود ولی خودبه‌خود کوچک نمی‌شود.",
      en: "Docker keeps all images and containers inside one virtual disk. It grows but never shrinks by itself.",
    },
    ifDeleted: {
      fa: "حذفش همهٔ ایمیج‌ها و کانتینرها را از بین می‌برد. به‌جای حذف، آن را فشرده می‌کنیم تا فضای خالی داخلش آزاد شود.",
      en: "Deleting it wipes every image and container. Instead we compact it, which frees the empty space inside.",
    },
    safety: "careful",
    method: "compact_vhdx",
    needsAdmin: true,
  },
  "telegram-cache": {
    title: { fa: "حافظهٔ موقت ویدیوهای تلگرام", en: "Telegram's video cache" },
    whyBig: {
      fa: "تلگرام هر ویدیو و عکسی را که در گروه‌ها و کانال‌ها دیده‌اید اینجا ذخیره می‌کند.",
      en: "Telegram saves every video and photo you have seen in chats and channels here.",
    },
    ifDeleted: {
      fa: "پیام‌ها و فایل‌هایتان در تلگرام می‌ماند. فقط اگر دوباره ویدیویی را باز کنید، از نو دانلود می‌شود.",
      en: "Your messages and files stay in Telegram. Videos just download again if you open them.",
    },
    safety: "probably_safe",
    method: "delete_contents",
    instructions: {
      fa: "در تلگرام: تنظیمات ← پیشرفته ← مدیریت فضای ذخیره‌سازی ← پاک کردن همه",
      en: "In Telegram: Settings, Advanced, Manage local storage, Clear all",
    },
  },
  "chrome-cache": {
    title: { fa: "حافظهٔ موقت کروم", en: "Chrome cache" },
    whyBig: {
      fa: "کروم تکه‌هایی از سایت‌ها را نگه می‌دارد تا صفحه‌ها سریع‌تر باز شوند.",
      en: "Chrome keeps pieces of websites so pages open faster.",
    },
    ifDeleted: {
      fa: "این پوشه را می‌توانید بی‌خطر پاک کنید. رمزها و تاریخچه دست نمی‌خورند. سایت‌ها بار اول کمی کندتر باز می‌شوند.",
      en: "Safe to clean. Passwords and history are not touched. Sites load a bit slower the first time.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "edge-cache": {
    title: { fa: "حافظهٔ موقت اج", en: "Edge cache" },
    whyBig: {
      fa: "مرورگر اج تکه‌هایی از سایت‌ها را برای سرعت بیشتر نگه می‌دارد.",
      en: "Edge keeps pieces of websites to load them faster.",
    },
    ifDeleted: {
      fa: "این پوشه را می‌توانید بی‌خطر پاک کنید. رمزها و تاریخچه دست نمی‌خورند.",
      en: "Safe to clean. Passwords and history are not touched.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "npm-cache": {
    title: { fa: "حافظهٔ موقت npm", en: "npm cache" },
    whyBig: {
      fa: "npm هر بسته‌ای را که یک بار دانلود کرده نگه می‌دارد تا نصب بعدی سریع‌تر باشد.",
      en: "npm keeps every package it ever downloaded so the next install is faster.",
    },
    ifDeleted: {
      fa: "بی‌خطر است. نصب بعدی بسته‌ها کمی بیشتر طول می‌کشد.",
      en: "Safe. The next package install takes a little longer.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "pip-cache": {
    title: { fa: "حافظهٔ موقت pip", en: "pip cache" },
    whyBig: {
      fa: "pip بسته‌های پایتون دانلودشده را اینجا نگه می‌دارد.",
      en: "pip keeps downloaded Python packages here.",
    },
    ifDeleted: {
      fa: "بی‌خطر است. اگر لازم باشد دوباره دانلود می‌شوند.",
      en: "Safe. They are downloaded again when needed.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "gpu-shader-cache": {
    title: { fa: "حافظهٔ موقت کارت گرافیک", en: "Graphics shader cache" },
    whyBig: {
      fa: "درایور کارت گرافیک نتیجهٔ کارهای تکراری بازی‌ها را ذخیره می‌کند تا دفعهٔ بعد سریع‌تر باشد.",
      en: "The graphics driver saves compiled shaders from games so they load faster next time.",
    },
    ifDeleted: {
      fa: "بی‌خطر است. بازی‌ها بار اول کمی کندتر بالا می‌آیند.",
      en: "Safe. Games may load a bit slower the first time.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "spotify-cache": {
    title: { fa: "حافظهٔ موقت اسپاتیفای", en: "Spotify cache" },
    whyBig: {
      fa: "اسپاتیفای آهنگ‌هایی را که گوش داده‌اید اینجا نگه می‌دارد.",
      en: "Spotify keeps songs you have played here.",
    },
    ifDeleted: {
      fa: "بی‌خطر است. آهنگ‌های دانلودشده برای حالت آفلاین هم ممکن است دوباره دانلود شوند.",
      en: "Safe. Songs saved for offline may need to download again.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "discord-cache": {
    title: { fa: "حافظهٔ موقت دیسکورد", en: "Discord cache" },
    whyBig: {
      fa: "عکس‌ها و ویدیوهایی که در دیسکورد دیده‌اید اینجا ذخیره می‌شوند.",
      en: "Images and videos you have seen in Discord are stored here.",
    },
    ifDeleted: {
      fa: "بی‌خطر است. پیام‌ها روی سرور می‌مانند.",
      en: "Safe. Messages stay on the server.",
    },
    safety: "safe",
    method: "delete_contents",
  },
  "crash-dumps": {
    title: { fa: "گزارش‌های خطای ویندوز", en: "Windows error reports" },
    whyBig: {
      fa: "هر بار که برنامه‌ای کرش می‌کند، ویندوز یک گزارش بزرگ اینجا می‌سازد.",
      en: "Every time a program crashes, Windows writes a large report here.",
    },
    ifDeleted: {
      fa: "این پوشه را می‌توانید بی‌خطر پاک کنید.",
      en: "Safe to clean.",
    },
    safety: "safe",
    method: "delete_contents",
    needsAdmin: true,
  },
  "old-installer-iso": {
    title: { fa: "فایل نصب قدیمی", en: "Old installer file" },
    whyBig: {
      fa: "یک فایل نصب یا ایمیج دیسک که ماه‌هاست در پوشهٔ دانلودها مانده.",
      en: "An installer or disk image that has been sitting in Downloads for months.",
    },
    ifDeleted: {
      fa: "اگر برنامه را نصب کرده‌اید، معمولاً دیگر لازمش ندارید. به سطل بازیافت می‌رود.",
      en: "If the program is installed, you usually don't need it. It goes to the Recycle Bin.",
    },
    safety: "probably_safe",
    method: "recycle",
  },
  "steam-library": {
    title: { fa: "بازی‌های استیم", en: "Steam games" },
    whyBig: {
      fa: "بازی‌های نصب‌شده از استیم. بازی‌های امروزی خیلی جا می‌گیرند.",
      en: "Games installed through Steam. Modern games are huge.",
    },
    ifDeleted: {
      fa: "بازی‌هایی را که دیگر بازی نمی‌کنید از خود استیم حذف کنید، نه دستی. پیشرفت بازی معمولاً در فضای ابری می‌ماند.",
      en: "Uninstall games you no longer play from Steam itself, not by hand. Saves usually stay in the cloud.",
    },
    safety: "careful",
    method: "open_app_setting",
    instructions: {
      fa: "در استیم: کتابخانه ← راست‌کلیک روی بازی ← مدیریت ← حذف نصب",
      en: "In Steam: Library, right-click the game, Manage, Uninstall",
    },
  },
  "package-cache": {
    title: { fa: "حافظهٔ نصب برنامه‌ها", en: "Package Cache" },
    whyBig: {
      fa: "برنامه‌هایی مثل ویژوال استودیو فایل نصبشان را اینجا نگه می‌دارند.",
      en: "Programs like Visual Studio keep their installers here.",
    },
    ifDeleted: {
      fa: "تعمیر یا حذف آن برنامه‌ها ممکن است مشکل پیدا کند.",
      en: "Repairing or uninstalling those programs may fail.",
    },
    safety: "careful",
    method: "manual_only",
  },
  onedrive: {
    title: { fa: "وان‌درایو", en: "OneDrive" },
    whyBig: {
      fa: "فایل‌هایی از وان‌درایو که روی این کامپیوتر هم ذخیره شده‌اند.",
      en: "OneDrive files that are also stored on this PC.",
    },
    ifDeleted: {
      fa: "حذف از اینجا، از فضای ابری هم حذف می‌کند. به‌جایش گزینهٔ «آزاد کردن فضا» را در وان‌درایو بزنید.",
      en: "Deleting here also deletes from the cloud. Use Free up space in OneDrive instead.",
    },
    safety: "careful",
    method: "manual_only",
    instructions: {
      fa: "روی پوشه راست‌کلیک کنید و «آزاد کردن فضا» را بزنید.",
      en: "Right-click the folder and choose Free up space.",
    },
  },
  "node-modules": {
    title: { fa: "بسته‌های node_modules", en: "node_modules packages" },
    whyBig: {
      fa: "بسته‌هایی که یک پروژهٔ جاوااسکریپت برای اجرا لازم دارد. هر پروژه نسخهٔ خودش را دارد.",
      en: "Packages a JavaScript project needs to run. Every project has its own copy.",
    },
    ifDeleted: {
      fa: "پروژه تا وقتی دوباره npm install نزنید اجرا نمی‌شود. کد شما دست نمی‌خورد.",
      en: "The project won't run until you run npm install again. Your code is not touched.",
    },
    safety: "probably_safe",
    method: "recycle",
  },
  "python-venv": {
    title: { fa: "محیط مجازی پایتون", en: "Python virtual environment" },
    whyBig: {
      fa: "کتابخانه‌های پایتون که فقط برای همین پروژه نصب شده‌اند.",
      en: "Python libraries installed just for this project.",
    },
    ifDeleted: {
      fa: "با نصب دوبارهٔ کتابخانه‌ها می‌شود آن را ساخت. کد شما دست نمی‌خورد.",
      en: "It can be rebuilt by reinstalling the libraries. Your code is not touched.",
    },
    safety: "probably_safe",
    method: "recycle",
  },
  "android-system-images": {
    title: { fa: "ایمیج‌های شبیه‌ساز اندروید", en: "Android emulator images" },
    whyBig: {
      fa: "هر نسخهٔ اندرویدی که برای شبیه‌ساز دانلود کرده‌اید چند گیگابایت جا می‌گیرد.",
      en: "Every Android version you downloaded for the emulator takes several gigabytes.",
    },
    ifDeleted: {
      fa: "شبیه‌سازهایی که از آن‌ها استفاده می‌کنند اجرا نمی‌شوند. بهتر است از SDK Manager حذفشان کنید.",
      en: "Emulators that use them stop working. Remove them from the SDK Manager instead.",
    },
    safety: "careful",
    method: "manual_only",
  },
  "orphan-adobe": {
    title: { fa: "باقی‌ماندهٔ برنامهٔ حذف‌شده", en: "Leftovers from a removed app" },
    whyBig: {
      fa: "این پوشه مال Adobe Premiere است، ولی این برنامه دیگر روی کامپیوتر نصب نیست.",
      en: "This folder belongs to Adobe Premiere, which is no longer installed.",
    },
    ifDeleted: {
      fa: "احتمالاً هیچ اتفاقی نمی‌افتد، مگر اینکه بخواهید دوباره نصبش کنید و تنظیمات قبلی را بخواهید.",
      en: "Most likely nothing, unless you reinstall it and want your old settings back.",
    },
    safety: "probably_safe",
    method: "recycle",
    source: "heuristic",
    confidence: 0.78,
  },
  "ai-vendor-updater": {
    title: { fa: "فایل‌های به‌روزرسان یک برنامه", en: "An app updater's downloads" },
    whyBig: {
      fa: "به نظر می‌رسد این پوشه نسخه‌های دانلودشدهٔ به‌روزرسانی یک برنامه باشد که بعد از نصب پاک نشده‌اند.",
      en: "This looks like downloaded update packages for an app that were not removed after installing.",
    },
    ifDeleted: {
      fa: "احتمالاً مشکلی پیش نمی‌آید، ولی پیش از حذف بررسی کنید.",
      en: "Probably fine, but check before deleting.",
    },
    safety: "probably_safe",
    method: "recycle",
    source: "ai",
    confidence: 0.6,
  },
};

export function explanation(ruleId: string): Explanation | null {
  const r = RULES[ruleId];
  if (!r) return null;
  return {
    ruleId,
    source: r.source ?? "knowledge_base",
    title: r.title,
    whyBig: r.whyBig,
    ifDeleted: r.ifDeleted,
    safety: r.safety,
    method: r.method,
    needsAdmin: r.needsAdmin ?? false,
    instructions: r.instructions ?? null,
    confidence: r.confidence ?? null,
  };
}
