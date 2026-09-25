# Build phases

One phase at a time. Each phase ends with a working app, a pushed branch and a short CHANGELOG note. Details on how things fit together are in `ARCHITECTURE.md`.

---

## P1. App shell, i18n, design system, release pipeline

Goal: an empty but polished app that already ships as an installer.

- Tauri 2 + Svelte 5 + TypeScript + Vite project in a Cargo workspace
- `fa.json` and `en.json`, and a language switch that works instantly without a restart
- Full RTL mirroring in Persian (layout, direction-aware icons, paddings)
- Vazirmatn bundled locally
- Helpers: Persian digits, Jalali dates, and one size formatter ("۲۳٫۴ گیگابایت" / "23.4 GB")
- Light and dark theme from design tokens, including the 4 safety colors
- Layout: sidebar (drives, settings), main area, top bar with the Simple / Expert switch
- Drive list with a used/free bar, filesystem and label
- An empty state that says "Choose a drive to scan"
- `ci.yml` (clippy, tests, type check, build) and `release.yml` (tag `v*` builds NSIS and publishes a GitHub Release)
- `CHANGELOG.md` and first release `v0.1.0`

Done when:
- [ ] `pnpm tauri dev` opens the app
- [ ] FA/EN switch flips the whole layout instantly
- [ ] Digits, sizes and dates look right in both languages
- [ ] Drives show correct used and free space
- [ ] Pushing tag `v0.1.0` produces a GitHub Release with the setup `.exe`, no manual steps

## P2. Normal scanner

Goal: a correct scan without admin.

- Parallel walk, cancellable, progress events (files, bytes, current path, elapsed)
- Real size on disk: allocated size, hardlinks once, cloud placeholders as 0 but flagged
- Access denied folders are counted and listed, and never cause a crash
- Long paths and junctions/symlinks (not followed, no loops)
- Arena tree plus paged `get_children`
- Results list: breadcrumb, size bar, percent, file count, last modified
- Benchmark of a full C: scan
- Unit tests for the size logic

Done when:
- [ ] Total is close to what Windows reports as used
- [ ] WinSxS no longer looks huge
- [ ] Cancel works mid-scan
- [ ] No crash on `System Volume Information`

## P3. Fast scan (MFT)

Goal: WizTree-level speed on NTFS.

- `fast-scan-helper` sidecar, elevated via UAC, named pipe with a token
- Automatic fallback to P2 with a plain explanation (UAC refused, not NTFS, helper failed)
- Same tree format as P2
- Comparison report between the two scanners
- Scan time shown to the user ("Scanned 1.2M files in 4.1s")

Done when:
- [ ] Much faster than the normal scan
- [ ] Totals match closely
- [ ] Saying No to UAC falls back cleanly

## P4. Visualization

- ECharts treemap with drill-down and a breadcrumb back
- Sunburst as a second view
- Colors by category (for now from extensions and known paths)
- Tooltip: name, size, percent, file count, last modified
- Expert mode detail panel
- "Largest files" and "By type" tabs
- Smooth on 1M+ files, correct in RTL

Done when:
- [ ] No lag drilling into a full C: scan
- [ ] Persian labels and tooltips are readable

## P5. Knowledge base and junk generator

- Rules engine: env var expansion, the most specific rule wins, result attached to the node
- 50+ rules (system files, Windows update and temp, browsers, messaging apps, games, GPU caches, dev tool caches, virtual disks, phone backups, old installers, OneDrive cache), explanations written for normal users in natural Persian and English
- vhdx files use "compact", never delete
- `tools/junk-gen`: fills a sandbox path with a fake Windows-like tree, refuses C:
- Tests: both languages present, valid enums, nothing in the block list can match

Done when:
- [ ] Every fake item on T: is recognized
- [ ] 10 random Persian texts read naturally
- [ ] Nothing in the block list is labeled safe

## P6. Heuristics

- Orphaned app data (compared with installed programs)
- Stale files (honest about disabled access time)
- Duplicates: size, then partial hash, then full hash, plus a suggestion of which copy to keep
- Old projects and how much of them can be rebuilt
- A confidence level on each result, never "safe"

Done when:
- [ ] Duplicate finder is correct on identical and almost identical files
- [ ] Installed apps are never flagged as orphans

## P7. Simple mode story

- A short plain-language story at the top
- Top 5 reasons as cards (icon, title, size, one line why, safety color, expandable "what happens")
- "Safely free X GB" button, safe items only, with a review screen
- "Needs your decision" section for careful items

Done when:
- [ ] A non-technical person understands it without help

## P8. Cleanup engine

- Plan, dry run, execution, result screen, history
- Methods: recycle, delete contents, official command, open app setting, compact vhdx, manual only
- Clear explanation that the Recycle Bin on the same drive frees nothing, with options
- Restore point offer before system actions
- Files in use are skipped, never forced
- Block list enforced in the engine, and the sandbox is enforced in debug builds
- Every action logged in SQLite

Done when:
- [ ] Dry run matches the real run on T:
- [ ] Blocked paths are refused by every route
- [ ] Locked files are skipped
- [ ] System actions tested only in the VM

## P9. AI layer (optional, bring your own key)

- Providers: OpenAI, Gemini, Anthropic, Groq. Key, model picker, and a test button for each
- Keys in Credential Manager only
- A warning when enabling and on every answer
- Only used for items rules and heuristics could not explain
- Metadata only, username masked, a preview before the first request
- Strict JSON, validated, maximum safety level "probably_safe"
- SQLite cache, a clear badge, friendly offline and error messages
- The app works fully with this turned off

Done when:
- [ ] Every provider works with a real key
- [ ] No key appears in logs or cache
- [ ] A wrong key or no internet gives friendly errors

## P10. Snapshots and growth

- Compact snapshots in SQLite
- Compare view ("since last scan C: grew 12 GB...")
- USN journal incremental rescans (NTFS, admin)
- Optional tray mode: weekly check and low space notifications
- Growth chart per folder

## P11. Settings, polish, packaging

- Full settings page, About page, "report a wrong rule" link
- 3 onboarding screens
- Keyboard navigation, focus states, contrast
- Performance pass (memory on 2M files, startup time)
- Installer polish (icon, version info, both languages)
- Tauri updater that reads from GitHub Releases (added to `release.yml`)
- `README.md` and `README.fa.md`
- Final read of all Persian texts

Done when:
- [ ] Clean VM install and a full run in both languages
- [ ] Uninstall leaves nothing behind except what the user chose to keep
- [ ] An installed older version updates itself from a new release
