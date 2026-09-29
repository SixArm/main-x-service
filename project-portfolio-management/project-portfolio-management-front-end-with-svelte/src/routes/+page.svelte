<!--
  Home (/) — a welcoming splash page for visitors who are not signed in;
  once signed in, the dashboard, which sends the caller to their landing
  route (T-28f: `/plans`, or the route of a deployment-declared `view`
  ABAC attribute). `signedIn` and `landingRoute` come from the root
  layout's server load (httpOnly session cookie), so nothing fires for an
  anonymous visitor.
-->
<script lang="ts">
  import Dashboard from "$lib/components/Dashboard.svelte";
  import Splash from "$lib/components/Splash.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
</script>

{#if data.signedIn}
  <Dashboard landingRoute={data.landingRoute} />
{:else}
  <Splash />
{/if}
