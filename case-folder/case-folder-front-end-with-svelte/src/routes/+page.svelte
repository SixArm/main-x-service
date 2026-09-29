<!--
  Home (`/`) — a welcoming splash page for visitors who are not signed in,
  and the folder-estate dashboard once they are. `signedIn` comes from the
  root layout's load (the `/api/auth/me` probe); `+page.ts` skips the
  dashboard's API reads for an anonymous visitor.
-->
<script lang="ts">
    import Dashboard from '$lib/components/Dashboard.svelte';
    import Splash from '$lib/components/Splash.svelte';

    let { data } = $props();
</script>

{#if data.signedIn}
    <Dashboard />
{:else}
    <Splash />
{/if}
