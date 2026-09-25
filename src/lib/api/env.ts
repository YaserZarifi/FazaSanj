/** True inside the Tauri webview. In a normal browser (pnpm dev) the mock backend is used. */
export const isTauri: boolean = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
