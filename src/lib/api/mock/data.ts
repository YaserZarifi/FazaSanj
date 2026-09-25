import type { DriveInfo } from "../types";
import { GB, MB, bulk, d, f, type Spec } from "./tree";

const hex = (i: number) => ((i * 2654435761) >>> 0).toString(16).slice(0, 6).toUpperCase();

const NPM_PACKAGES = [
  "typescript",
  "@babel",
  "webpack",
  "esbuild",
  "@swc",
  "next",
  "react-dom",
  "lodash",
  "@types",
  "prettier",
  "eslint",
  "sharp",
  "rxjs",
  "caniuse-lite",
  "core-js",
  "date-fns",
  "@aws-sdk",
  "puppeteer",
  "electron",
  "vite",
];

const nodeModules = (size: number, files: number, age: number): Spec =>
  bulk("node_modules", size, files, {
    gen: 20,
    genDirs: true,
    genName: (i) => NPM_PACKAGES[i % NPM_PACKAGES.length],
    rule: "node-modules",
    cat: "dev",
    age,
  });

const winsxs = bulk("WinSxS", 9.5 * GB, 88_000, {
  gen: 640,
  genDirs: true,
  rule: "windows-winsxs",
  genName: (i) => `amd64_microsoft-windows-${["shell", "font", "media", "net", "print", "ie"][i % 6]}-${hex(i).toLowerCase()}_31bf3856ad364e35_10.0.22621.${3000 + i}_none_${hex(i + 7).toLowerCase()}`,
});

const windowsDir = d(
  "Windows",
  [
    winsxs,
    bulk("System32", 6.2 * GB, 21_000, { gen: 90, genName: (i) => `${["win", "ntos", "shell", "d3d", "api-ms-win"][i % 5]}${i}.dll` }),
    bulk("SysWOW64", 1.8 * GB, 9_800, { gen: 30, genName: (i) => `wow${i}.dll` }),
    bulk("Installer", 5.4 * GB, 1_240, { rule: "windows-installer-cache", gen: 40, genName: (i) => `${hex(i + 3)}.msi` }),
    d("SoftwareDistribution", [
      bulk("Download", 3.1 * GB, 2_600, { rule: "windows-update-downloads", gen: 24, age: 20, genName: (i) => `windows10.0-kb50${31000 + i}-x64.cab` }),
      bulk("DataStore", 210 * MB, 30, { genName: (i) => `DataStore${i}.edb` }),
    ]),
    bulk("Temp", 1.2 * GB, 3_400, { rule: "windows-temp", cat: "cache", gen: 30, genName: (i) => `tmp${hex(i)}.tmp` }),
    bulk("assembly", 2.3 * GB, 12_000, { gen: 20, genDirs: true, genName: (i) => `NativeImages_v4.0_${i}` }),
    bulk("Microsoft.NET", 1.4 * GB, 8_000, { gen: 16, genDirs: true, genName: (i) => `Framework64_${i}` }),
    bulk("SystemApps", 820 * MB, 3_100, { gen: 12, genDirs: true, genName: (i) => `Microsoft.Windows.App${i}_cw5n1h2txyewy` }),
    bulk("Fonts", 610 * MB, 1_100, { gen: 40, genName: (i) => `font${i}.ttf` }),
    bulk("servicing", 900 * MB, 2_300, { gen: 10, genDirs: true, genName: (i) => `Packages${i}` }),
    bulk("Logs", 410 * MB, 900, { gen: 12, genName: (i) => `CBS${i}.log` }),
    d("Containers", [], { denied: true }),
  ],
  { cat: "system" },
);

const programFiles = d(
  "Program Files",
  [
    bulk("Microsoft Visual Studio", 11.2 * GB, 64_000, { gen: 10, genDirs: true, cat: "dev", genName: (i) => ["2022", "Shared", "Installer", "Common7", "VC", "MSBuild", "DIA SDK", "Team Tools", "Xamarin", "Licenses"][i] }),
    bulk("Microsoft Office", 3.9 * GB, 11_000, { gen: 8, genDirs: true, genName: (i) => `root${i}` }),
    bulk("Docker", 3.1 * GB, 2_100, { cat: "virtualization", gen: 6, genDirs: true }),
    bulk("JetBrains", 3.5 * GB, 18_000, { cat: "dev", gen: 3, genDirs: true, genName: (i) => ["IntelliJ IDEA 2025.2", "WebStorm 2025.2", "Toolbox"][i] }),
    bulk("Adobe", 2.8 * GB, 9_000, { gen: 2, genDirs: true, genName: (i) => ["Adobe Photoshop 2025", "Acrobat DC"][i] }),
    bulk("NVIDIA Corporation", 1.8 * GB, 3_000, { gen: 8, genDirs: true }),
    bulk("Mozilla Firefox", 410 * MB, 300, { cat: "browsers", gen: 12, genName: (i) => `xul${i}.dll` }),
    bulk("Google", 520 * MB, 400, { cat: "browsers", gen: 3, genDirs: true, genName: () => "Chrome" }),
    bulk("Git", 390 * MB, 3_900, { cat: "dev", gen: 5, genDirs: true }),
    bulk("Python312", 310 * MB, 7_600, { cat: "dev", gen: 5, genDirs: true }),
    bulk("7-Zip", 6 * MB, 20, { gen: 4, genName: (i) => ["7z.exe", "7z.dll", "7zFM.exe", "7zG.exe"][i] }),
    d("WindowsApps", [], { denied: true }),
  ],
  { cat: "apps" },
);

const steam = d(
  "Steam",
  [
    d(
      "steamapps",
      [
        d("common", [
          bulk("Counter-Strike Global Offensive", 11.4 * GB, 1_800, { gen: 10, age: 90 }),
          bulk("Hollow Knight", 8.8 * GB, 620, { gen: 8, age: 400 }),
          bulk("Stardew Valley", 640 * MB, 2_400, { gen: 8, age: 40 }),
          bulk("Celeste", 1.2 * GB, 900, { gen: 8, age: 600 }),
        ]),
        bulk("shadercache", 1.9 * GB, 1_100, { gen: 10 }),
      ],
      { rule: "steam-library" },
    ),
    bulk("bin", 380 * MB, 700, { gen: 10, genName: (i) => `steamclient${i}.dll` }),
  ],
  { cat: "games" },
);

const programFilesX86 = d(
  "Program Files (x86)",
  [
    steam,
    bulk("Microsoft", 2.4 * GB, 8_000, { gen: 6, genDirs: true }),
    bulk("Windows Kits", 3.2 * GB, 22_000, { cat: "dev", gen: 4, genDirs: true }),
    bulk("Common Files", 1.1 * GB, 3_000, { gen: 6, genDirs: true }),
    bulk("Microsoft SDKs", 1.6 * GB, 5_000, { cat: "dev", gen: 4, genDirs: true }),
  ],
  { cat: "apps" },
);

const programData = d(
  "ProgramData",
  [
    bulk("Package Cache", 3.2 * GB, 1_900, { rule: "package-cache", gen: 30, genDirs: true, genName: (i) => `{${hex(i)}-${hex(i + 11)}}` }),
    d("Microsoft", [
      d("Windows", [bulk("WER", 1.4 * GB, 260, { rule: "crash-dumps", cat: "cache", gen: 20, genName: (i) => `AppCrash_${hex(i)}.dmp` })]),
      bulk("Windows Defender", 1.2 * GB, 1_400, { gen: 6, genDirs: true }),
      bulk("Search", 860 * MB, 40, { gen: 4, genName: (i) => `Windows${i}.edb` }),
    ]),
    bulk("Docker", 1.1 * GB, 400, { cat: "virtualization", gen: 5, genDirs: true }),
    d("VendorSync", [bulk("downloads", 1.3 * GB, 14, { rule: "ai-vendor-updater", gen: 7, genName: (i) => `VendorSync-update-4.${i}.msi` })]),
    bulk("NVIDIA", 640 * MB, 500, { gen: 6, genDirs: true }),
  ],
  { cat: "apps" },
);

const appDataLocal = d("Local", [
  d("Docker", [d("wsl", [d("disk", [f("docker_data.vhdx", 52.4 * GB, { age: 1, rule: "docker-vhdx", cat: "virtualization" })])])], { cat: "virtualization" }),
  bulk("Temp", 4.8 * GB, 18_400, { rule: "user-temp", cat: "cache", gen: 340, genName: (i) => (i % 7 === 0 ? `${hex(i)}.tmp` : i % 5 === 0 ? `setup_${hex(i)}.log` : `~DF${hex(i)}.TMP`) }),
  d(
    "Google",
    [
      d("Chrome", [
        d("User Data", [
          d("Default", [
            d("Cache", [bulk("Cache_Data", 2.4 * GB, 9_800, { gen: 80, genName: (i) => `f_${hex(i).toLowerCase()}` })], { rule: "chrome-cache", cat: "cache", age: 0 }),
            bulk("Code Cache", 410 * MB, 2_100, { gen: 20, genName: (i) => `js_${hex(i).toLowerCase()}` }),
            bulk("IndexedDB", 360 * MB, 800, { gen: 10, genDirs: true }),
            f("History", 92 * MB, { age: 0 }),
          ]),
        ]),
      ]),
    ],
    { cat: "browsers" },
  ),
  d(
    "Microsoft",
    [
      d("Edge", [d("User Data", [d("Default", [bulk("Cache", 1.1 * GB, 5_200, { rule: "edge-cache", cat: "cache", gen: 40, genName: (i) => `f_${hex(i + 99).toLowerCase()}` })])])], { cat: "browsers" }),
      bulk("OneDrive", 420 * MB, 900, { gen: 6, genDirs: true }),
      bulk("Windows", 780 * MB, 3_000, { gen: 8, genDirs: true, cat: "system" }),
    ],
    { cat: "apps" },
  ),
  bulk("npm-cache", 3.6 * GB, 140_000, { rule: "npm-cache", cat: "dev", gen: 3, genDirs: true, genName: (i) => ["_cacache", "_logs", "_npx"][i] }),
  d("pip", [bulk("cache", 1.9 * GB, 12_000, { rule: "pip-cache", gen: 4, genDirs: true, genName: (i) => ["http-v2", "wheels", "selfcheck", "http"][i] })], { cat: "dev" }),
  d("NVIDIA", [bulk("DXCache", 1.2 * GB, 3_100, { rule: "gpu-shader-cache", gen: 30, genName: (i) => `${hex(i)}.nvph` })], { cat: "cache" }),
  d("Spotify", [bulk("Storage", 1.6 * GB, 4_000, { rule: "spotify-cache", gen: 30, genName: (i) => `${hex(i).toLowerCase()}.file` })], { cat: "media" }),
  d(
    "Android",
    [
      d("Sdk", [
        bulk("system-images", 9.4 * GB, 1_900, { rule: "android-system-images", gen: 3, genDirs: true, genName: (i) => `android-${33 + i}` }),
        bulk("platforms", 2.1 * GB, 9_000, { gen: 4, genDirs: true, genName: (i) => `android-${31 + i}` }),
        bulk("build-tools", 920 * MB, 3_000, { gen: 3, genDirs: true, genName: (i) => `34.0.${i}` }),
      ]),
    ],
    { cat: "dev" },
  ),
  bulk("Packages", 3.2 * GB, 16_000, { gen: 18, genDirs: true, genName: (i) => `Microsoft.App${i}_8wekyb3d8bbwe` }),
  d("JetBrains", [bulk("PyCharm2021.3", 1.4 * GB, 22_000, { gen: 4, genDirs: true, age: 820, genName: (i) => ["caches", "index", "log", "tmp"][i] })], { cat: "dev" }),
  d("Zoomify", [bulk("updates", 2.2 * GB, 9, { gen: 9, age: 160, genName: (i) => `zoomify-setup-5.${10 + i}.exe` })]),
]);

const appDataRoaming = d("Roaming", [
  d(
    "Telegram Desktop",
    [
      d("tdata", [
        bulk("user_data", 31.2 * GB, 46_000, { rule: "telegram-cache", age: 0, gen: 60, genName: (i) => `${hex(i)}${hex(i + 5)}.mp4` }),
        bulk("emoji", 120 * MB, 900, { gen: 10 }),
        f("settings.dat", 2 * MB),
      ]),
      f("Telegram.exe", 180 * MB, { age: 12 }),
    ],
    { cat: "messaging" },
  ),
  d("discord", [bulk("Cache", 820 * MB, 7_800, { rule: "discord-cache", cat: "cache", gen: 30, genName: (i) => `data_${i}` })], { cat: "messaging" }),
  d("Adobe", [bulk("Premiere Pro", 2.1 * GB, 1_300, { rule: "orphan-adobe", age: 700, gen: 6, genDirs: true })], { cat: "apps" }),
  bulk("Code", 910 * MB, 8_000, { cat: "dev", gen: 6, genDirs: true, genName: (i) => ["User", "CachedData", "logs", "Cache", "CachedExtensionVSIXs", "Service Worker"][i] }),
]);

const projects = d(
  "Projects",
  [
    d(
      "shop-api",
      [nodeModules(2.9 * GB, 210_000, 540), bulk("src", 42 * MB, 900, { gen: 12, age: 540, genName: (i) => `module${i}.ts` }), bulk(".git", 310 * MB, 3_000, { gen: 8, genDirs: true, age: 540 }), f("package.json", 4_000, { age: 540 })],
      { age: 540 },
    ),
    d("fazasanj-site", [nodeModules(1.1 * GB, 90_000, 8), bulk("src", 18 * MB, 300, { gen: 12, age: 3, genName: (i) => `page${i}.svelte` }), f("package.json", 2_000, { age: 8 })]),
    d(
      "thesis-ml",
      [
        bulk(".venv", 4.1 * GB, 61_000, { rule: "python-venv", gen: 4, genDirs: true, age: 700, genName: (i) => ["Lib", "Scripts", "share", "include"][i] }),
        bulk("data", 6.2 * GB, 240, { gen: 16, age: 710, genName: (i) => (i % 2 ? `features_${i}.npy` : `raw_${i}.csv`) }),
        bulk("notebooks", 120 * MB, 40, { gen: 10, age: 700, genName: (i) => `exp${i}.ipynb` }),
      ],
      { age: 700 },
    ),
    d(
      "mobile-app",
      [nodeModules(1.6 * GB, 150_000, 420), d("android", [bulk("build", 2.2 * GB, 30_000, { gen: 6, genDirs: true, age: 420 })]), bulk("src", 30 * MB, 400, { gen: 10, age: 420, genName: (i) => `screen${i}.tsx` })],
      { age: 420 },
    ),
  ],
  { cat: "dev" },
);

const userAli = d(
  "Ali",
  [
    d("AppData", [appDataLocal, appDataRoaming]),
    d(
      "Downloads",
      [
        f("Win11_24H2_English_x64.iso", 6.2 * GB, { rule: "old-installer-iso", age: 310 }),
        f("ubuntu-24.04-desktop-amd64.iso", 5.7 * GB, { rule: "old-installer-iso", age: 220 }),
        f("Docker Desktop Installer.exe", 610 * MB, { rule: "old-installer-iso", age: 400 }),
        f("wedding (1).mp4", 2.3 * GB, { cat: "media", age: 260 }),
        bulk("Series", 14.2 * GB, 24, { cat: "media", gen: 24, age: 120, genName: (i) => `episode-${String(i + 1).padStart(2, "0")}.mkv` }),
        bulk("misc", 3.1 * GB, 420, { gen: 40, genName: (i) => [`scan${i}.pdf`, `photo${i}.jpg`, `backup${i}.zip`, `invoice${i}.pdf`][i % 4] }),
      ],
      { age: 30 },
    ),
    d("Documents", [
      bulk("Archive", 3.2 * GB, 120, { age: 720, gen: 20, genName: (i) => `archive-20${18 + (i % 5)}-${i}.zip` }),
      bulk("Work", 5.8 * GB, 3_900, { gen: 60, genName: (i) => [`report${i}.docx`, `sheet${i}.xlsx`, `slides${i}.pptx`, `contract${i}.pdf`][i % 4] }),
      bulk("Visual Studio 2022", 1.2 * GB, 800, { cat: "dev", gen: 6, genDirs: true }),
      bulk("Zoom", 1.9 * GB, 60, { cat: "media", gen: 20, genName: (i) => `meeting_${i}.mp4` }),
    ]),
    bulk("Pictures", 18.4 * GB, 21_000, { cat: "media", gen: 7, genDirs: true, genName: (i) => `${2019 + i}` }),
    d(
      "Videos",
      [
        f("wedding.mp4", 2.3 * GB, { age: 900 }),
        bulk("Captures", 11.6 * GB, 130, { gen: 40, genName: (i) => `Game ${i + 1}.mp4` }),
        bulk("Movies", 30.1 * GB, 22, { gen: 22, age: 500, genName: (i) => `movie-${i + 1}.mkv` }),
      ],
      { cat: "media" },
    ),
    bulk("Music", 6.1 * GB, 1_400, { cat: "media", gen: 60, genName: (i) => `track-${i + 1}.mp3` }),
    bulk("Desktop", 3.1 * GB, 210, { gen: 30, age: 2, genName: (i) => [`notes${i}.txt`, `shot${i}.png`, `draft${i}.docx`][i % 3] }),
    projects,
    d(
      "OneDrive",
      [
        bulk("Documents", 5.2 * GB, 2_000, { gen: 30, genName: (i) => `shared${i}.docx` }),
        bulk("Photos", 2.9 * GB, 3_000, { gen: 20, genName: (i) => `IMG_${1000 + i}.jpg` }),
        f("Personal Vault.lnk", 0, { flags: { cloudOnly: true } }),
        f("Big video (online only).mp4", 0, { flags: { cloudOnly: true } }),
      ],
      { rule: "onedrive" },
    ),
  ],
  { cat: "user_files" },
);

export const C_DRIVE_SPEC: Spec[] = [
  windowsDir,
  f("pagefile.sys", 12 * GB, { cat: "system", rule: "pagefile", age: 0, flags: { system: true } }),
  f("hiberfil.sys", 6.4 * GB, { cat: "system", rule: "hiberfil", age: 0, flags: { system: true } }),
  f("swapfile.sys", 256 * MB, { cat: "system", age: 0, flags: { system: true } }),
  bulk("Windows.old", 21.3 * GB, 160_000, { cat: "system", rule: "windows-old", age: 190, gen: 4, genDirs: true, genName: (i) => ["Windows", "Program Files", "Users", "ProgramData"][i] }),
  programFiles,
  programFilesX86,
  programData,
  d("Users", [userAli, bulk("Public", 310 * MB, 200, { cat: "user_files", gen: 5, genDirs: true }), bulk("Default", 90 * MB, 120, { gen: 3, genDirs: true, cat: "system" })]),
  bulk("$Recycle.Bin", 3.8 * GB, 640, { cat: "user_files", rule: "recycle-bin", gen: 20, genName: (i) => `$R${hex(i)}.zip` }),
  d("System Volume Information", [], { denied: true, cat: "system" }),
  d("Config.Msi", [], { denied: true, cat: "system" }),
  d("Documents and Settings", [], { cat: "system", flags: { reparse: true } }),
];

export const PROJECTS_PATH = "C:\\Users\\Ali\\Projects";
export const PROJECTS_SPEC: Spec[] = projects.children ?? [];

export const D_DRIVE_SPEC: Spec[] = [
  bulk("Movies", 320 * GB, 410, { cat: "media", gen: 60, genName: (i) => `film-${i + 1}.mkv` }),
  d("Games", [bulk("Epic Games", 180 * GB, 90_000, { gen: 6, genDirs: true, genName: (i) => ["Fortnite", "RocketLeague", "GTAV", "Control", "Hades", "Launcher"][i] })], { cat: "games" }),
  d("Backups", [bulk("iPhone", 96 * GB, 180_000, { gen: 3, genDirs: true, age: 300 }), bulk("Laptop 2022", 44 * GB, 400_000, { gen: 6, genDirs: true, age: 900 })], { cat: "user_files" }),
  bulk("Music", 42 * GB, 9_000, { cat: "media", gen: 50, genName: (i) => `album-${i}.flac` }),
  d("VMs", [f("ubuntu-dev.vhdx", 38 * GB, { cat: "virtualization", age: 60 })], { cat: "virtualization" }),
  d("$RECYCLE.BIN", [], { denied: true }),
];

export const E_DRIVE_SPEC: Spec[] = [
  bulk("Photos 2024", 12.4 * GB, 4_200, { cat: "media", gen: 40, genName: (i) => `DSC_${4000 + i}.jpg` }),
  bulk("Installers", 8.1 * GB, 30, { gen: 12, genName: (i) => `setup-${i}.exe` }),
  bulk("School", 2.2 * GB, 900, { cat: "user_files", gen: 30, genName: (i) => `lesson${i}.pdf` }),
];

export const DRIVES: DriveInfo[] = [
  { letter: "C:", root: "C:\\", label: "Windows", filesystem: "NTFS", total: 476 * GB, free: 0, kind: "fixed", isSystem: true, fastScanAvailable: true },
  { letter: "D:", root: "D:\\", label: "Data", filesystem: "NTFS", total: 931 * GB, free: 0, kind: "fixed", isSystem: false, fastScanAvailable: true },
  { letter: "E:", root: "E:\\", label: "USB", filesystem: "exFAT", total: 58 * GB, free: 0, kind: "removable", isSystem: false, fastScanAvailable: false },
];

/** Space Windows reports as used but no scan can see (metadata, reserved storage). */
export const HIDDEN_USED: Record<string, number> = { "C:": 2.4 * GB, "D:": 0.6 * GB, "E:": 0.05 * GB };
