import type { ApiError } from "../api/types";
import { toApiError } from "../api/errors";
import { errorText } from "../i18n/index.svelte";

export type ToastKind = "error" | "success" | "info";

export interface Toast {
  id: number;
  kind: ToastKind;
  text: string;
}

let nextId = 1;

class Toasts {
  list = $state<Toast[]>([]);

  push(kind: ToastKind, text: string, ms = 5000): void {
    const id = nextId++;
    this.list = [...this.list, { id, kind, text }];
    if (ms > 0) setTimeout(() => this.dismiss(id), ms);
  }

  error(e: unknown): void {
    const err: ApiError = toApiError(e);
    console.warn("backend error", err);
    this.push("error", errorText(err), 7000);
  }

  dismiss(id: number): void {
    this.list = this.list.filter((t) => t.id !== id);
  }
}

export const toasts = new Toasts();
