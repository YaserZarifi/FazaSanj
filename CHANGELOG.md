# Changelog

Every release gets a short note here. The text under each version is also what shows up on the GitHub release page.

## 0.1.0

First public version. Fazasanj scans your drives, tells you what is using the space and helps you get it back without breaking anything.

- Normal scan works without admin rights and counts the real size on disk (hard links once, cloud-only OneDrive files as zero)
- Fast scan reads the NTFS file table directly. It asks for admin permission, and falls back to the normal scan if you say no
- Knows over 100 common space eaters, from Windows update leftovers and browser caches to Telegram, Docker, WSL and developer caches, each with a plain explanation of why it's big and what happens if you remove it
- Finds leftovers of uninstalled apps, duplicate files, files you haven't touched in a year and old coding projects
- Simple mode tells the story of your drive in a few sentences and frees only what is safe. Expert mode has a tree, treemap, sunburst chart, largest files and a breakdown by file type
- Cleanup always shows a review first, supports a dry run, skips files in use and keeps a history
- System items like hiberfil, WinSxS and restore points are handled only through the official Windows tools
- Remembers past scans and shows what grew since last time
- Optional AI explanations for unknown folders with your own key. Only names, sizes and dates are sent
- Persian and English, light and dark theme, and an optional tray mode with a weekly check
