<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let drives = $state<string[]>([]);

  $effect(() => {
    invoke<{ letter: string }[]>("list_drives")
      .then((list) => (drives = list.map((d) => d.letter)))
      .catch(() => (drives = []));
  });
</script>

<main>
  <h1>فضاسنج</h1>
  <p>{drives.join(" ")}</p>
</main>
