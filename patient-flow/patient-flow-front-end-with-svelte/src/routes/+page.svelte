<!--
  Home (/) — a welcoming splash page for visitors who are not signed in,
  and the ward dashboard once they are. `signedIn` comes from the root
  layout's server load (httpOnly session cookie); `+page.ts` only fetches
  the dashboard data when signed in, so an anonymous visitor triggers no
  API call.
-->
<script lang="ts">
  import Dashboard from "$lib/components/Dashboard.svelte";
  import Splash from "$lib/components/Splash.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
</script>

{#if data.signedIn && data.glance}
  <Dashboard glance={data.glance} />
{:else}
  <Splash />
{/if}
