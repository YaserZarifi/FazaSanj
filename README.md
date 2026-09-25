# Fazasanj

[فارسی](README.fa.md)

Fazasanj is a disk analyzer for Windows 10 and 11. It shows where your space went, explains in plain words why each big thing is there, tells you what is safe to remove, and shows what happens if you remove it. The app speaks Persian and English and switches between them instantly.

## What it does

- **Fast scan** on NTFS drives reads the file table directly (a small helper asks for admin once per scan). If you say no, the normal scan does the same job without admin, just slower.
- **Simple view** tells the story in a few sentences: "412 GB of this drive is in use. Most of it is games, Windows and videos. You can safely free about 18 GB right now." One button frees the safe part after you review the list.
- **Expert view** has a folder list, a treemap and a sunburst with drill down, the largest files, space by file type, and a detail panel for every item.
- **Knowledge base** with more than 50 rules for Windows, browsers, messengers, games, GPU caches, dev tools and virtual disks. Every rule has a safety level (safe, probably safe, careful, do not touch) and says what happens if you delete the item.
- **Findings** for things rules can't know: duplicate files, leftovers of removed programs, old code projects and files nobody has used in a long time. These are guesses, so they are never marked safe.
- **Careful cleanup**: a review screen, dry run, Recycle Bin by default, an optional restore point before system actions, files in use are skipped and never forced, and a hard block list that no route can get around. Every action is logged.
- **Growth over time**: each scan keeps a small snapshot, so you can see what grew since last time.
- **Optional AI** (OpenAI, Gemini, Anthropic or Groq, with your own key) for folders nothing else can explain. Only metadata is sent, your user name is always hidden, you see the exact payload first, and keys live in Windows Credential Manager. The app works fully without it.
- Tray mode with low space warnings and a weekly check, light and dark theme, and self update from GitHub Releases.

## Install

Download `Fazasanj_x.y.z_x64-setup.exe` from the [latest release](https://github.com/YaserZarifi/FazaSanj/releases/latest) and run it. The installer isn't signed yet, so Windows SmartScreen may warn you. Click "More info" and then "Run anyway".

## Build from source

You need Windows 10 or 11, [Rust](https://rustup.rs) (stable), Node.js 22 and pnpm.

```
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` also builds the fast scan helper and puts it where Tauri expects a sidecar. To build the installer, run `pnpm tauri build --bundles nsis`.

Checks that CI runs:

```
pnpm check
pnpm test
pnpm build
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### Trying cleanup safely

Debug builds refuse to delete anything outside a sandbox folder. Fill a spare drive or folder with fake junk and point the sandbox at it:

```
cargo run -p junk-gen -- T:\
```

Then set the sandbox in Settings > Advanced (or with the `DEV_SANDBOX` environment variable).

## How it is built

Tauri 2 with a Rust core and a Svelte 5 + TypeScript UI. The Rust side is split into small crates (scanner, MFT helper, rules, heuristics, cleanup, AI, storage and the app layer that ties them together). See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the details and [docs/PHASES.md](docs/PHASES.md) for the plan.

## Found a wrong explanation?

Use "Report a wrong explanation" on the About page, or [open an issue](https://github.com/YaserZarifi/FazaSanj/issues/new). Rules live in `crates/rules/rules/*.json`, and adding a rule needs no code change.

## License

MIT
