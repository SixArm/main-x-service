<!--
  Landing route (`/`) — redirects to `data.landingRoute`
  (`+layout.server.ts`): `/plans` by default (the four collections were
  unified into one recursive plans collection), or a deployment-declared
  `view` ABAC attribute's route (T-28f, repo `tasks.md` EV-1) when one is
  set and recognised, e.g. `view=executive` -> `/executive`.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();

  onMount(() => {
    void goto(data.landingRoute, { replaceState: true });
  });
</script>

<svelte:head><title>{t("brand.name")}</title></svelte:head>

<h1>{t("list.title")}</h1>
<p><a href="/plans">{t("list.title")}</a></p>
