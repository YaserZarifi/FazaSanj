# Fazasanj architecture

Fazasanj is a Windows disk analyzer. It tells you where your space went, why, what is safe to delete, and what happens if you delete it. The UI is bilingual (Persian RTL and English).

Stack: Tauri 2, a Rust core, Svelte 5 + TypeScript + Vite for the UI, ECharts for charts, SQLite (rusqlite) for storage, and Windows Credential Manager (keyring crate) for API keys. Target: Windows 10/11 x64 only.

This doc is the plan. It will change as we build. When it does, update it in the same commit as the code.

---

## 1. Folder structure

```
FazaSanj/
  Cargo.toml                 workspace
  package.json               frontend deps and scripts (pnpm)
  CHANGELOG.md               written by hand, used as release notes
  .github/workflows/
    ci.yml                   build, clippy, tests on every push
    release.yml              builds the installer and publishes a release on a v* tag
  scripts/                   release helpers (version check, notes, sidecar copy)
  docs/
  crates/
    model/                   serde types shared with the UI (mirrored in src/lib/api/types.ts)
    safety/                  hard block list and the dev sandbox check
    platform/                every raw Win32 call (volumes, file ids, allocation, elevation)
    scan/                    arena tree, size logic, normal walker, fast scan client, queries
    mft-helper/              fast-scan-helper.exe, runs elevated, reads the MFT, streams over a pipe
    rules/                   knowledge base engine, rule JSON files live in crates/rules/rules/
    heuristics/              orphans, stale files, duplicates, old projects
    cleanup/                 plans, dry run, executors, cleanup log writer
    ai/                      providers, payload builder, validation
    store/                   SQLite: snapshots, cleanup log, AI cache, settings
  tools/
    junk-gen/                dev only CLI, fills a sandbox drive with fake junk (refuses C:)
  src-tauri/                 the app: Tauri setup, commands, events, app state (glue only)
    src/commands/            one file per area
    binaries/                sidecar copy of fast-scan-helper (built, not committed)
  src/                       Svelte UI
    lib/api/                 typed wrappers around Tauri commands and events
    lib/i18n/                fa.json, en.json, t()
    lib/format/              sizes, numbers, Persian digits, Jalali and Gregorian dates
    lib/theme/               design tokens
    lib/stores/              app state
    components/              layout, drives, scan, viz, simple, cleanup, settings, common
```

Each crate owns one job and can be built and tested on its own. The `src-tauri` crate only wires them together, so there is no logic in command handlers.

---

## 2. Rust crates

| Module | Job |
|---|---|
| `platform` | The only place with `unsafe` and `windows` crate calls. Lists volumes (label, filesystem, total, free). Reads a file ID (volume serial + 128 bit file ID), allocated size, attributes (compressed, sparse, reparse point, recall on open / offline). Also checks if the process is elevated and launches the helper with ShellExecuteExW "runas". |
| `tree` | Arena storage for a scan: a flat `Vec<Node>` with parent index, first child, next sibling, and names in a string interner. Computes sizes and counts bottom-up after the walk. Answers "top N children of node X sorted by size", paths, and search. Both scanners fill the same structure. |
| `size` | Decides the size of one entry. Allocated size, not logical size. Hardlinks are counted once, using a set of seen file IDs. Cloud placeholders (OneDrive and friends) count as 0 and get a `cloud_only` flag. Reparse points and junctions are not followed. Unit tested without touching the disk. |
| `scanner` | The normal walk. Needs no admin. Parallel directory walk on a thread pool (rayon or a custom work queue). Uses `FindFirstFileExW` with large fetch, `\\?\` long paths, and a cancel flag checked often. Sends progress events every ~100 ms. Collects access denied folders into a list instead of failing. |
| `mft` | The Fast Scan client. Checks that the drive is NTFS, starts the helper elevated, reads records from the named pipe, and rebuilds the tree from parent file reference numbers. Then it fills the same `tree` arena. Falls back to `scanner` on any failure. |
| `rules` | Loads `rules/*.json` at startup, expands env vars (`%LOCALAPPDATA%`, `%USERPROFILE%`...) for the current user, and compiles patterns into a matcher. Matches every node. The most specific rule wins (longest literal path prefix, then file pattern, then priority). The result is attached to the node as a rule index. |
| `heuristics` | Things rules do not know. Orphaned app data (compared with the installed programs in the registry), stale files, duplicates (size, then partial hash, then full hash), old projects. Output always has a confidence level and is never "safe". |
| `cleanup` | Builds a cleanup plan from the user's selection. Supports dry run and runs actions: recycle, delete contents, official command, open app setting, compact vhdx, manual only. Enforces the hard block list inside the engine. In debug builds it refuses anything outside `DEV_SANDBOX`. Skips files in use and never forces. Logs every action. |
| `ai` | Optional. Provider trait with OpenAI, Gemini, Anthropic and Groq implementations. Builds a metadata only payload with the username masked, asks for strict JSON, validates it, and caps the safety level at "probably_safe". Caches answers. API keys come from `keyring` only. |
| `storage` | SQLite through rusqlite, with versioned migrations. Tables: `scans`, `snapshot_nodes`, `cleanup_log`, `ai_cache`, `settings`. |
| `i18n` | Backend strings for errors and events. Rule texts are already bilingual in the JSON. The backend sends keys plus params, and the UI renders them. |
| `src-tauri` commands | Thin handlers that validate input, call a crate and map errors to `ApiError`. No logic here. |

Errors use `thiserror` in each module and are converted into one `AppError` with a stable `code` the UI can translate. There is no `unwrap()` outside tests.

---

## 3. Tauri commands and events

All sizes are bytes (u64, sent as numbers; safe up to 9 PB in JS). The UI does all formatting.

### Commands

| Command | Input | Output |
|---|---|---|
| `list_drives` | nothing | List of drives: letter, label, filesystem, total, free, drive type (fixed, removable, network), and whether Fast Scan is possible (NTFS). |
| `start_scan` | drive or folder path, mode (`normal` or `fast`) | A scan ID. Work continues in the background and reports through events. |
| `cancel_scan` | scan ID | Nothing. The scan stops soon after and emits `scan://cancelled`. |
| `get_node` | scan ID, node ID | One node: name, full path, size, file count, folder count, last modified, flags (cloud_only, access_denied, reparse), category, and matched rule info. |
| `get_children` | scan ID, node ID, sort (size, name, modified), offset, limit | One page of children plus the total child count. The UI never loads more than it shows. |
| `get_treemap_slice` | scan ID, node ID, depth (1 to 3), max items per level | A small nested tree for the chart. Tiny items are grouped into one "other" item. |
| `get_largest_files` | scan ID, limit | Top N files on the whole scan. |
| `get_by_type` | scan ID | Space per extension group (video, images, archives, installers, disk images...). |
| `get_access_denied` | scan ID | Folders we could not read. |
| `get_story` | scan ID | The Simple mode data: total, the main buckets, top 5 reasons with rule keys, the safe to free total, and items that need a decision. |
| `run_heuristics` | scan ID, which ones | A job ID. Results arrive through events. |
| `build_cleanup_plan` | scan ID, list of node IDs or rule IDs | A plan: actions with method, size, consequence key, whether admin is needed, and warnings (for example "Recycle Bin on the same drive does not free space"). |
| `run_cleanup` | plan ID, dry run flag, create restore point flag | A job ID. Progress and the final report arrive through events. |
| `get_cleanup_history` | paging | Past runs with per action results and what can still be restored. |
| `ai_explain` | scan ID, node ID | The cached or fresh AI answer, or a clear error code. |
| `ai_preview_payload` | scan ID, node ID | The exact JSON that would be sent. |
| `settings_get` / `settings_set` | key / key and value | Settings values. |
| `ai_key_set` / `ai_key_test` / `ai_key_delete` | provider, key | OK or an error code. Keys never come back to the UI. |

### Events

| Event | Payload |
|---|---|
| `scan://progress` | scan ID, files scanned, bytes counted, current path, elapsed ms. Throttled to about 10 per second. |
| `scan://done` | scan ID, totals, duration, scanner used, and fallback reason if any. |
| `scan://cancelled` | scan ID. |
| `scan://error` | scan ID, error code. |
| `heuristics://progress` / `heuristics://done` | job ID, stage, counts. |
| `cleanup://progress` | job ID, action index, current path, bytes freed so far. |
| `cleanup://done` | job ID, before and after free space, list of results (ok, skipped in use, blocked, failed with code). |

---

## 4. Scan data model

A full C: drive can have 2 million or more entries. Sending that to a WebView would freeze it, so the tree stays in Rust.

Node (in the arena, roughly 48 to 64 bytes each):

- `name`: an index into the string interner (repeated names like `cache`, `node_modules` and `index.js` are stored once)
- `parent`, `first_child`, `next_sibling`: u32 indices
- `kind`: file, dir, or reparse
- `own_size` (allocated) and `total_size` (own + children)
- `file_count` and `dir_count` for the subtree
- `modified`: the newest modified time in the subtree, for folders
- `flags`: cloud_only, access_denied, compressed, sparse, hardlink_dup, system
- `rule`: optional rule index, plus a category

Flow:

1. The scanner pushes nodes into the arena from several threads (per thread buffers, merged in order).
2. When the walk ends, one pass computes `total_size` and counts bottom-up.
3. Children of each folder are sorted by size once, so `get_children` is just a slice.
4. The rules engine tags nodes. The category rolls up to parents for coloring.
5. The UI asks for pages: the top 100 children, the next 100, a treemap slice limited to depth 3 with an "other" bucket.

2M nodes at about 64 bytes is around 130 MB in Rust memory, which is fine. The UI holds at most a few thousand items at a time.

Only the latest scan per drive stays in memory. Phase 10 adds snapshots, which are saved to SQLite in a compact form (folders only, down to a fixed depth).

---

## 5. Elevation (Fast Scan)

The main app always runs as a normal user. Running the WebView as admin causes problems (drag and drop breaks, and a bigger attack surface), so only a small helper gets admin.

How it works:

1. The user picks Fast Scan on an NTFS drive.
2. The main app creates a named pipe `\\.\pipe\fazasanj-<random>`. Its security descriptor only allows the current user's SID, and it accepts one client.
3. It launches `fast-scan-helper.exe` (bundled as a Tauri sidecar) with `ShellExecuteExW` and the `runas` verb. The arguments are the drive letter, the pipe name and a one time token.
4. Windows shows UAC.
   - If the user says no, `ShellExecuteExW` fails with `ERROR_CANCELLED`. The app falls back to the normal scanner and says so in plain words, for example "Fast scan needs admin permission. We're using the normal scan instead. It's slower but works the same."
5. The helper opens the volume (`\\.\C:`), runs `FSCTL_ENUM_USN_DATA` to get every entry (file reference, parent reference, name, attributes). USN records have no sizes, so it also reads `$MFT` itself in large chunks and parses the `$DATA` attribute of each record to get allocated size (the way WizTree does it). It connects to the pipe, sends the token first, and then streams records in batches in a simple length prefixed binary format.
6. The main app checks the token and rebuilds the tree from parent references into the same arena. It then fixes up hardlinks and cloud placeholders the same way as the normal scanner.
7. The helper exits when it is done. It never deletes or writes anything, and it only accepts a drive letter as input.

Fallback cases, all with a clear message in both languages: UAC refused, drive not NTFS (FAT32, exFAT, ReFS), helper crashed or timed out, token mismatch.

Hard parts to watch: sizes from the MFT (non resident data, compressed streams, alternate data streams). Phase 3 compares both scanners on the same drive and reports differences.

---

## 6. Knowledge base rule schema

One JSON file per category in `src-tauri/rules/`. Each file holds an array of rules.

| Field | Type | Required | Meaning |
|---|---|---|---|
| `id` | string | yes | Stable, unique, kebab case. Example: `telegram-desktop-cache`. |
| `category` | enum | yes | system, apps, games, media, dev, cache, user_files, virtualization, messaging, browsers. |
| `paths` | string[] | yes | Path patterns with env vars and globs. Example: `%APPDATA%\Telegram Desktop\tdata\user_data`. |
| `file_patterns` | string[] | no | Extra filter inside the path. Example: `*.exe`, `*.iso`. |
| `min_age_days` | number | no | Only match files older than this (used for old installers in Downloads). |
| `match` | enum | no | `folder` (the folder itself), `contents` (what's inside), or `file`. Default `folder`. |
| `priority` | number | no | Tie breaker when two rules are equally specific. |
| `title` | {fa, en} | yes | Short name shown on cards. |
| `why_big` | {fa, en} | yes | Why this is big, for a normal user. |
| `if_deleted` | {fa, en} | yes | What happens if you remove it. |
| `safety` | enum | yes | `safe`, `probably_safe`, `careful`, `do_not_touch`. |
| `method` | enum | yes | `recycle`, `delete_contents`, `command`, `open_app_setting`, `compact_vhdx`, `manual_only`. |
| `command` | object | no | For `command`: program, args, and whether it shows its own UI. Only programs from an allow list (dism, powercfg, vssadmin, cleanmgr). |
| `instructions` | {fa, en} | no | Steps for `manual_only` and `open_app_setting`. |
| `open_target` | string | no | URI or exe to open for `open_app_setting`. |
| `needs_admin` | bool | yes | Whether the cleanup needs elevation. |
| `permanent_ok` | bool | no | Whether this can skip the Recycle Bin (only allowed for `safe` caches). |
| `source` | string | no | Where the info comes from (doc link, vendor page). |
| `notes` | string | no | Notes for maintainers. Not shown to users. |

Tests check that every rule has both languages, a valid enum value, a valid method and command combination, and no pattern that can match the hard block list.

---

## 7. Phases

See `docs/PHASES.md`:

| Phase | What |
|---|---|
| P1 | Shell, i18n, design system, CI and release pipeline |
| P2 | Normal scanner |
| P3 | Fast MFT scan |
| P4 | Visualization |
| P5 | Knowledge base and junk generator |
| P6 | Heuristics |
| P7 | Simple mode story |
| P8 | Cleanup engine |
| P9 | AI layer |
| P10 | Snapshots and growth |
| P11 | Settings, polish, packaging, updater |

---

## 8. Releases

- `ci.yml` runs on every push: `cargo clippy -D warnings`, `cargo test`, `pnpm check`, `pnpm build`.
- `release.yml` runs when a tag like `v0.3.0` is pushed. On `windows-latest` it installs Rust and pnpm, runs `tauri-apps/tauri-action`, builds the NSIS installer (Persian and English), and publishes a GitHub Release named `Fazasanj v0.3.0` with the setup `.exe` attached.
- The release text is the matching section of `CHANGELOG.md`, pulled out by a small script step. The changelog is written by hand in plain words.
- The version lives in three places (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`). A CI check fails the release if they don't match the tag.
- The installer is unsigned for now, so Windows SmartScreen shows a warning. A code signing certificate is needed before a wide public release.
- In P11 the Tauri updater reads `latest.json` from the GitHub release, so installed copies update themselves. The same workflow uploads it.

---

## 9. Risks and open questions

Risks:

1. **MFT size accuracy.** Compressed, sparse and alternate data streams are easy to get wrong. We compare both scanners on the same drive and accept small differences only.
2. **Memory on huge drives.** 5M+ files could push past 300 MB. Mitigation: a compact node layout and the string interner. We measure in P11.
3. **ECharts with many items.** The treemap gets slow above a few thousand rectangles, so we cap each slice and group the rest into "other".
4. **Wrong safety labels.** This is the biggest trust risk. Mitigation: conservative defaults, tests against the block list, the sandbox rule in debug builds, and review of every `safe` rule.
5. **Helper pipe security.** Another process could try to connect first. Mitigation: a random pipe name, a current user only ACL, a single client, and a one time token.
6. **Access time is often disabled** on Windows, so "not accessed in a year" can be misleading. Stale detection says clearly when it only has the modified date.
7. **SmartScreen warnings** on an unsigned installer will scare some users.
8. **Persian text quality.** Every text gets a manual read. The wording aims to be neutral so both Iranian and Dari speakers are comfortable.

Open questions (answers welcome, defaults in brackets):

1. Should the app remember the last scan across restarts, or always start fresh until P10? [start fresh]
2. Which drives should be listed: network and removable drives too, or fixed only? [fixed and removable, network hidden by default]
3. Minimum Windows version: 10 1809+ or 10 21H2+? [10 1809, since WebView2 and USN APIs work there]
4. License? [MIT for the code, rules maybe CC BY 4.0 so others can contribute]
5. Should rule files be editable by users (a local overrides folder)? [not in v1]
