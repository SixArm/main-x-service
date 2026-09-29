<!--
  Home (/) — a welcoming splash page for visitors who are not signed in,
  and the workforce dashboard once they are. `signedIn` comes from the root
  layout's server load (httpOnly session cookie), so the dashboard's API
  calls never fire for an anonymous visitor.
-->
<script lang="ts">
  import Dashboard from "$lib/components/Dashboard.svelte";
  import Splash from "$lib/components/Splash.svelte";

  let { data } = $props();
</script>

{#if data.signedIn}
  <Dashboard />
{:else}
  <Splash />
{/if}
