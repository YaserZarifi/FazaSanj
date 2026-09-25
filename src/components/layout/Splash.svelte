<script lang="ts">
  import Logo from "./Logo.svelte";
</script>

<div class="splash" aria-busy="true">
  <div class="pulse"><Logo size={56} /></div>
</div>

<style>
  .splash {
    display: grid;
    place-items: center;
    height: 100%;
  }

  .pulse {
    animation: breathe 1.6s var(--ease) infinite alternate;
  }

  @keyframes breathe {
    from {
      opacity: 0.5;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
