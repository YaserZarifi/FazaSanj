<script lang="ts">
  interface Props {
    /** 0..1 */
    value: number;
    label: string;
    height?: number;
    tone?: "accent" | "warn" | "danger";
  }

  let { value, label, height = 6, tone = "accent" }: Props = $props();
  const pct = $derived(Math.max(0, Math.min(1, value)) * 100);
</script>

<div
  class="bar {tone}"
  style:height="{height}px"
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={Math.round(pct)}
>
  <div class="fill" style:width="{pct}%"></div>
</div>

<style>
  .bar {
    width: 100%;
    border-radius: var(--r-full);
    background: var(--surface-3);
    overflow: hidden;
  }

  .fill {
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
    transition: width var(--dur-slow) var(--ease);
  }

  .warn .fill {
    background: var(--careful);
  }

  .danger .fill {
    background: var(--danger);
  }
</style>
