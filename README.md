# Fazasanj

[فارسی](README.fa.md)

Fazasanj is a disk analyzer for Windows 10 and 11. It doesn't just draw a map of your drive. It tells you **where** your space went, **why** it's there, what is **safe** to remove, and **what happens** if you remove it.

The app speaks Persian and English. The Persian UI is fully right to left.

## What it does

- Scans a drive or a folder and shows the real size on disk. Hard links count once, compressed files use their real size, and OneDrive files that live only in the cloud count as zero.
- **Fast scan** reads the NTFS master file table directly, like WizTree. It asks for admin permission once. If you say no, the normal scan runs instead.
- Recognizes more than 100 known space eaters: Windows update leftovers, browser caches, Telegram and Discord caches, Docker and WSL disks, npm/pip/Cargo caches, old installers in Downloads and many more. Each one comes with a plain explanation.
- Finds things no list can know about: app data left behind by uninstalled programs, duplicate files, files nobody has touched in a year, and old coding projects with big `node_modules` or `target` folders.
- **Simple mode** gives you a short story ("Your C: drive is 476 GB. Docker uses 52 GB...") and one button that only frees what is safe. **Expert mode** has a tree, a treemap, a sunburst chart, the largest files and a breakdown by file type.
- Remembers past scans and shows what grew since last time.
- Optional AI explanations for folders nothing else recognizes, with your own API key (OpenAI, Gemini, Claude or Groq). Only names, sizes and dates are sent, never file contents. It is off by default.

## Safety

- Nothing is deleted without your confirmation, and every cleanup can be tried as a dry run first.
- A hard block list protects System32, Program Files, registry files, user profile roots and drive roots. The cleanup engine enforces it, not just the UI.
- Windows items like the page file, hibernation file, WinSxS and restore points are never deleted as files. Fazasanj uses the official Windows tools for those (DISM, powercfg, vssadmin, Disk Cleanup).
- Files that are in use are skipped, never forced.
- Every cleanup is logged, and you can see what is still in the Recycle Bin.

## Install

Download `Fazasanj_x.y.z_x64-setup.exe` from the [releases page](https://github.com/YaserZarifi/FazaSanj/releases/latest) and run it.

The installer isn't signed yet, so Windows SmartScreen may warn you. Click "More info" and then "Run anyway". Installed copies update themselves when a new release comes out.

## Build from source

You need Rust (MSVC toolchain), Node.js 22 or newer, pnpm, and the Visual Studio C++ build tools.

```
pnpm install
pnpm tauri dev      # run in development
pnpm tauri build    # make the installer in target/release/bundle/nsis
```

Tests: `cargo test --workspace` and `pnpm test`.

To try cleanup safely, make a test drive and fill it with fake junk:

```
mkdir D:\fz-sandbox
subst T: D:\fz-sandbox
cargo run -p junk-gen -- T:\ --scale small
```

In debug builds, cleanup only works inside the path set as "Dev sandbox" in Settings.

## Project layout

- `crates/` has the Rust core: scanner, fast scan helper, rules engine, heuristics, cleanup engine, AI layer and storage.
- `crates/rules/rules/` has the knowledge base as JSON files. If a rule is wrong or missing, please [open an issue](https://github.com/YaserZarifi/FazaSanj/issues/new?labels=rule).
- `src/` has the Svelte UI.
- `docs/ARCHITECTURE.md` explains how it all fits together.

## License

MIT
