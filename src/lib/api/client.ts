import * as commands from "./commands";
import { isTauri } from "./env";

type Commands = typeof commands;
/** Every command wrapper from commands.ts. The mock implements the same shape. */
export type Backend = Omit<Commands, "EVENTS">;

const { EVENTS: _events, ...real } = commands;
const realBackend: Backend = real;

let active: Backend = realBackend;

/**
 * Picks the backend once at startup. Outside Tauri the mock is loaded lazily,
 * so it never ends up in the code path of the real app.
 */
export async function initBackend(): Promise<void> {
  if (isTauri) return;
  const { mockBackend } = await import("./mock");
  active = mockBackend;
}

/** Proxy so imports stay valid after initBackend swaps the implementation. */
export const backend: Backend = new Proxy({} as Backend, {
  get(_t, key: string) {
    return active[key as keyof Backend];
  },
});
