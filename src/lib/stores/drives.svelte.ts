import { backend } from "../api/client";
import type { DriveInfo } from "../api/types";
import { toasts } from "./toasts.svelte";

class DrivesStore {
  list = $state<DriveInfo[]>([]);
  loading = $state(false);
  failed = $state(false);

  async load(): Promise<void> {
    this.loading = true;
    this.failed = false;
    try {
      this.list = await backend.listDrives();
    } catch (e) {
      this.failed = true;
      toasts.error(e);
    } finally {
      this.loading = false;
    }
  }

  byPath(path: string): DriveInfo | null {
    const letter = path.slice(0, 2).toUpperCase();
    return this.list.find((d) => d.letter.toUpperCase() === letter) ?? null;
  }
}

export const drives = new DrivesStore();
