import type { ApiError } from "./types";

function isApiError(v: unknown): v is ApiError {
  return typeof v === "object" && v !== null && typeof (v as { code?: unknown }).code === "string";
}

/** Tauri rejects with the serialized ApiError. Anything else becomes a generic error. */
export function toApiError(e: unknown): ApiError {
  if (isApiError(e)) return { code: e.code, detail: e.detail ?? null };
  if (typeof e === "string") {
    try {
      const parsed: unknown = JSON.parse(e);
      if (isApiError(parsed)) return { code: parsed.code, detail: parsed.detail ?? null };
    } catch {
      // plain message, not JSON
    }
    return { code: "unknown", detail: e };
  }
  if (e instanceof Error) return { code: "unknown", detail: e.message };
  return { code: "unknown", detail: null };
}

export function apiError(code: string, detail: string | null = null): ApiError {
  return { code, detail };
}
