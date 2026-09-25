<script lang="ts">
  import { formatPercent, formatSize } from "../../lib/format";
  import { lang, t } from "../../lib/i18n/index.svelte";
  import { usage } from "../../lib/labels";
  import { app } from "../../lib/stores/app.svelte";
  import Icon from "../common/Icon.svelte";
  import UsageBar from "../common/UsageBar.svelte";

  const drives = $derived(app.drives.filter((d) => d.kind !== "network"));
</script>

<section class="home">
  <div class="hero">
    <Icon name="pie" size={44} />
    <h1>{t("home.title")}</h1>
    <p class="muted">{t("home.subtitle")}</p>
  </div>

  <div class="grid">
    {#each drives as d (d.root)}
      <button class="card drive" onclick={() => app.go({ kind: "target", root: d.root })}>
        <div class="row">
          <Icon name={d.kind === "removable" ? "usb" : "drive"} size={26} />
          <div class="name">
            <strong>{d.label || t("sidebar.localDisk")}</strong>
            <span class="mono faint">{d.letter}</span>
          </div>
          <span class="spacer"></span>
          <span class="pct">{formatPercent(usage(d.total, d.free).share, lang())}</span>
        </div>
        <UsageBar total={d.total} free={d.free} height={8} />
        <div class="row faint">
          <span>{t("home.used", { used: formatSize(d.total - d.free, lang()) })}</span>
          <span class="spacer"></span>
          <span>{t("home.free", { free: formatSize(d.free, lang()) })}</span>
        </div>
        {#if app.session(d.root)?.status === "done"}
          <span class="scanned">{t("home.scanned")}</span>
        {/if}
      </button>
    {/each}
  </div>
</section>

<style>
  .home {
    padding: 40px 32px;
    max-width: 980px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 10px;
    color: var(--accent);
  }
  .hero h1 {
    color: var(--text);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 14px;
  }
  .drive {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 16px;
    text-align: start;
    cursor: pointer;
    position: relative;
    transition:
      border-color 0.12s,
      box-shadow 0.12s;
  }
  .drive:hover {
    border-color: var(--accent);
    box-shadow: var(--shadow);
  }
  .name {
    display: flex;
    flex-direction: column;
    line-height: 1.3;
  }
  .pct {
    font-size: 20px;
    font-weight: 700;
  }
  .scanned {
    font-size: 12px;
    color: var(--safe);
  }
</style>
