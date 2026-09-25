import { backend } from "../api/client";
import { toApiError } from "../api/errors";
import type { AiAnswer, AiProviderStatus, ApiError, NodeInfo } from "../api/types";
import { i18n } from "../i18n/index.svelte";
import { scan } from "./scan.svelte";
import { settings } from "./settings.svelte";

interface Preview {
  node: NodeInfo;
  payload: unknown;
}

class AiStore {
  providers = $state<AiProviderStatus[]>([]);
  answers = $state<Record<string, AiAnswer>>({});
  loading = $state<Record<string, boolean>>({});
  errors = $state<Record<string, ApiError>>({});
  preview = $state<Preview | null>(null);

  get enabled(): boolean {
    return settings.value?.aiEnabled ?? false;
  }

  get ready(): boolean {
    return this.enabled && this.providers.some((p) => p.hasKey);
  }

  async loadProviders(): Promise<void> {
    try {
      this.providers = await backend.aiGetProviders();
    } catch {
      this.providers = [];
    }
  }

  /** Asks the AI about a node. Shows the payload preview the very first time. */
  async ask(node: NodeInfo): Promise<void> {
    if (scan.scanId == null) return;
    if (!settings.value?.aiPreviewAcknowledged) {
      try {
        const payload = await backend.aiPreviewPayload(scan.scanId, node.id);
        this.preview = { node, payload };
      } catch (e) {
        this.errors = { ...this.errors, [node.path]: toApiError(e) };
      }
      return;
    }
    await this.explain(node);
  }

  async confirmPreview(): Promise<void> {
    const p = this.preview;
    this.preview = null;
    if (!p) return;
    const ok = await settings.update({ aiPreviewAcknowledged: true });
    if (ok) await this.explain(p.node);
  }

  cancelPreview(): void {
    this.preview = null;
  }

  private async explain(node: NodeInfo): Promise<void> {
    if (scan.scanId == null) return;
    const key = node.path;
    this.loading = { ...this.loading, [key]: true };
    const { [key]: _old, ...rest } = this.errors;
    this.errors = rest;
    try {
      const a = await backend.aiExplain(scan.scanId, node.id, i18n.lang);
      this.answers = { ...this.answers, [key]: a };
    } catch (e) {
      this.errors = { ...this.errors, [key]: toApiError(e) };
    } finally {
      this.loading = { ...this.loading, [key]: false };
    }
  }
}

export const ai = new AiStore();
