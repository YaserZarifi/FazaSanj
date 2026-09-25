import { isTauri } from "./env";

export const MOCK_PICKED_FOLDER = "C:\Users\Ali\Projects";

/** Native folder picker. Returns null when the user closes the dialog. */
export async function pickFolder(): Promise<string | null> {
  if (!isTauri) return MOCK_PICKED_FOLDER;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

export async function openExternal(url: string): Promise<void> {
  if (!isTauri) {
    window.open(url, "_blank", "noopener");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
