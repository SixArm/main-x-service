<!--
  Dashboard — the signed-in home (/).

  Purpose:
    Redirects to the caller's landing route: `/plans` by default (the four
    collections were unified into one recursive plans collection), or a
    deployment-declared `view` ABAC attribute's route (T-28f, repo
    `tasks.md` EV-1) when one is set and recognised, e.g.
    `view=executive` -> `/executive`. Rendered only when signed in.

  $props:
    - landingRoute: string — the resolved route (`+layout.server.ts`).
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { t } from "$lib/i18n.svelte";

  let { landingRoute }: { landingRoute: string } = $props();

  onMount(() => {
    void goto(landingRoute, { replaceState: true });
  });
</script>

<h1>{t("list.title")}</h1>
<p><a href="/plans">{t("list.title")}</a></p>
