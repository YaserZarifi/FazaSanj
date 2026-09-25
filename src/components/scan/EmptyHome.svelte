<script lang="ts">
  import { t } from "../../lib/i18n/index.svelte";
  import { drives } from "../../lib/stores/drives.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import Icon from "../common/Icon.svelte";
</script>

<section class="empty">
  <svg class="art" viewBox="0 0 240 180" aria-hidden="true" focusable="false">
    <ellipse cx="120" cy="160" rx="84" ry="10" fill="var(--surface-3)" />
    <rect x="44" y="70" width="152" height="70" rx="16" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="2" />
    <rect x="60" y="112" width="120" height="8" rx="4" fill="var(--surface-3)" />
    <rect x="60" y="112" width="78" height="8" rx="4" fill="var(--accent)" />
    <circle cx="68" cy="90" r="4" fill="var(--safe)" />
    <rect x="80" y="86" width="52" height="8" rx="4" fill="var(--surface-3)" />
    <g transform="translate(150 22)">
      <circle cx="30" cy="30" r="26" fill="var(--surface)" stroke="var(--border-strong)" stroke-width="2" />
      <path d="M30 4a26 26 0 0 1 24.7 18" fill="none" stroke="var(--cat-virtualization)" stroke-width="8" />
      <path d="M54.7 22A26 26 0 0 1 44 52" fill="none" stroke="var(--cat-messaging)" stroke-width="8" />
      <path d="M44 52A26 26 0 0 1 8 44" fill="none" stroke="var(--cat-system)" stroke-width="8" />
      <path d="M8 44A26 26 0 0 1 30 4" fill="none" stroke="var(--cat-media)" stroke-width="8" />
      <circle cx="30" cy="30" r="15" fill="var(--surface)" />
    </g>
    <g transform="translate(18 18)">
      <circle cx="30" cy="30" r="20" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="4" />
      <path d="m45 45 16 16" stroke="var(--accent)" stroke-width="7" stroke-linecap="round" />
    </g>
  </svg>

  <h2>{t("home.emptyTitle")}</h2>
  <p class="muted lead">{t("home.emptyText")}</p>

  {#if drives.list.length}
    <div class="quick">
      {#each drives.list as d (d.letter)}
        <button type="button" class="btn" onclick={() => scan.chooseDrive(d)}>
          <Icon name={d.kind === "removable" ? "usb" : "drive"} size={16} />
          <bdi class="ltr">{d.letter}</bdi>
          {d.label}
        </button>
      {/each}
    </div>
  {/if}

  <ul class="promises">
    <li><Icon name="shield-check" size={16} /> {t("home.promise1")}</li>
    <li><Icon name="eye" size={16} /> {t("home.promise2")}</li>
    <li><Icon name="lock" size={16} /> {t("home.promise3")}</li>
  </ul>
</section>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--sp-3);
    max-width: 560px;
    margin: 0 auto;
    padding: var(--sp-12) var(--sp-6);
  }

  .art {
    width: 240px;
    height: 180px;
    margin-bottom: var(--sp-2);
  }

  h2 {
    font-size: var(--fs-2xl);
  }

  .lead {
    font-size: var(--fs-lg);
  }

  .quick {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--sp-2);
    margin-top: var(--sp-3);
  }

  .promises {
    list-style: none;
    padding: 0;
    margin: var(--sp-8) 0 0;
    display: flex;
    flex-direction: column;
    gap: var(--sp-2);
    color: var(--text-2);
    font-size: var(--fs-sm);
  }

  .promises li {
    display: flex;
    align-items: center;
    gap: var(--sp-2);
  }

  .promises :global(svg) {
    color: var(--safe);
  }
</style>
