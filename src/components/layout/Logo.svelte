<script lang="ts">
  let { size = 34 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 40 40" aria-hidden="true" focusable="false">
  <defs>
    <linearGradient id="logo-g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#4c7df0" />
      <stop offset="1" stop-color="#2a4fc4" />
    </linearGradient>
  </defs>
  <rect width="40" height="40" rx="11" fill="url(#logo-g)" />
  <path d="M9 26a11 11 0 0 1 22 0" fill="none" stroke="#fff" stroke-opacity=".35" stroke-width="3.2" stroke-linecap="round" />
  <path d="M9 26a11 11 0 0 1 15.5-10" fill="none" stroke="#fff" stroke-width="3.2" stroke-linecap="round" />
  <path d="M20 26 26.5 17.5" stroke="#fff" stroke-width="2.6" stroke-linecap="round" />
  <circle cx="20" cy="26" r="2.6" fill="#fff" />
</svg>
