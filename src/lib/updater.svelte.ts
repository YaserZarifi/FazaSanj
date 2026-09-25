// Self update from GitHub Releases (latest.json), through the Tauri updater plugin.

import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";

export const updater = $state<{
  checking: boolean;
  available: { version: string; notes: string } | null;
  installing: boolean;
  progress: number;
  error: string | null;
  lastChecked: number | null;
}>({ checking: false, available: null, installing: false, progress: 0, error: null, lastChecked: null });

let pending: Update | null = null;

/** Looks for a new version. `quiet` swallows errors (used for the check at start). */
export async function checkForUpdate(quiet = false) {
  if (updater.checking) return;
  updater.checking = true;
  updater.error = null;
  try {
    pending = await check();
    updater.available = pending ? { version: pending.version, notes: pending.body ?? "" } : null;
  } catch (e) {
    if (!quiet) updater.error = String(e);
  } finally {
    updater.checking = false;
    updater.lastChecked = Date.now();
  }
}

/** Downloads, installs and restarts. */
export async function installUpdate() {
  if (!pending) return;
  updater.installing = true;
  updater.progress = 0;
  let total = 0;
  let got = 0;
  try {
    await pending.downloadAndInstall((ev) => {
      if (ev.event === "Started") total = ev.data.contentLength ?? 0;
      if (ev.event === "Progress") {
        got += ev.data.chunkLength;
        updater.progress = total > 0 ? got / total : 0;
      }
    });
    await relaunch();
  } catch (e) {
    updater.error = String(e);
    updater.installing = false;
  }
}
