<!--
  Landing route (`/`) — the four collections were unified into one
  recursive plans collection, so this simply redirects to `/plans`.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n.svelte";

  onMount(() => {
    void goto("/plans", { replaceState: true });
  });
</script>

<svelte:head><title>{t("brand.name")}</title></svelte:head>

<h1>{t("list.title")}</h1>
<p><a href="/plans">{t("list.title")}</a></p>
