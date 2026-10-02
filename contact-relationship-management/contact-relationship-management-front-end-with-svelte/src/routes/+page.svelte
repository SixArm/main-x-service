<!--
  Home (/) — a welcoming splash page for visitors who are not signed in,
  and the CRM dashboard once they are. `signedIn` comes from the root
  layout's server load (httpOnly session cookie), so the dashboard's API
  calls never fire for an anonymous visitor.
-->
<script lang="ts">
  import Dashboard from "#lib/components/Dashboard.svelte";
  import Splash from "#lib/components/Splash.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
</script>

{#if data.signedIn}
  <Dashboard />
{:else}
  <Splash />
{/if}
