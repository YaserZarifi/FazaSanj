<script lang="ts">
  import { errorText, t } from "../../lib/i18n/index.svelte";
  import { scan } from "../../lib/stores/scan.svelte";
  import EmptyState from "../common/EmptyState.svelte";
</script>

<EmptyState icon="alert" title={t("scanError.title")} text={errorText(scan.error)}>
  <button type="button" class="btn btn-primary" onclick={() => scan.target && scan.choose(scan.target)}>
    {t("common.tryAgain")}
  </button>
</EmptyState>
