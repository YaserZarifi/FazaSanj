# Changelog

Every release gets a short note here. The text under each version is also what shows up on the GitHub release page.

## 0.2.0

The whole app is here. Scan, understand, clean up, and watch your drives over time.

- Fast scan on NTFS drives (asks for admin once) and a normal scan that needs no admin. If you say no to the admin prompt, the normal scan takes over and tells you why
- Simple view with a short story about where the space went, the five biggest reasons, a "Safely free" button and a list of things that need your decision
- Expert view with a folder list, treemap and sunburst, largest files, space by file type, folders we could not read, and a detail panel for every item
- Findings: duplicate files, leftovers of removed programs, old code projects and files nobody has used in a long time
- Cleanup with a review screen, dry run, Recycle Bin by default, an optional restore point, and a history of everything that was done
- Growth over time: see what grew between two scans, with a chart per folder
- Optional AI explanations with your own OpenAI, Gemini, Anthropic or Groq key. You see exactly what is sent first
- Tray mode with low space warnings and a weekly check
- Settings, an About page with self update, and three short welcome screens

## 0.1.0

First build. Nothing to scan yet, but the base is in place.

- The app opens with a sidebar that lists your drives, how full each one is and its filesystem
- Persian and English, switchable on the fly. Persian flips the whole layout right to left
- Light and dark theme
