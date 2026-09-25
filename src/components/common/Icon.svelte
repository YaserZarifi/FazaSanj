<script lang="ts">
  import { ICONS, type IconName } from "./icons";

  interface Props {
    name: IconName;
    size?: number;
    /** mirror in RTL (chevrons, arrows) */
    flip?: boolean;
    strokeWidth?: number;
    class?: string;
  }

  let { name, size = 18, flip = false, strokeWidth = 1.8, class: cls = "" }: Props = $props();
</script>

<svg
  class="icon {cls}"
  class:flip-rtl={flip}
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {@html ICONS[name]}
</svg>

<style>
  .icon {
    flex-shrink: 0;
    display: inline-block;
    vertical-align: middle;
  }
</style>
