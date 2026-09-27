// Self update from GitHub releases. The backend checks latest.json and installs the new setup.

import { backend } from "../api/client";
import type { UpdateInfo } from "../api/commands";

class UpdateStore {
  available = $state<UpdateInfo | null>(null);
  checking = $state(false);
  installing = $state(false);
  dismissed = $state(false);
  failed = $state(false);
  lastChecked = $state<number | null>(null);

  /** `quiet` hides failures, used for the check at startup. */
  async check(quiet = false) {
    if (this.checking) return;
    this.checking = true;
    this.failed = false;
    try {
      this.available = await backend.checkUpdate();
    } catch {
      if (!quiet) this.failed = true;
    } finally {
      this.checking = false;
      this.lastChecked = Date.now();
    }
  }

  async install() {
    if (!this.available || this.installing) return;
    this.installing = true;
    this.failed = false;
    try {
      // The app restarts itself when the new version is in place.
      await backend.installUpdate();
    } catch {
      this.failed = true;
      this.installing = false;
    }
  }
}

export const update = new UpdateStore();
