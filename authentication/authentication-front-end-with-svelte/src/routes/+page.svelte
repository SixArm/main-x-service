<!--
  Home (/) — a welcoming splash page for visitors who are not signed in
  (its call to action leads to /signin), and the account dashboard once
  they are. `data.user` comes from the root layout's server load (httpOnly
  session cookie), so nothing user-specific renders for an anonymous visitor.
-->
<script lang="ts">
    import Dashboard from "$lib/components/Dashboard.svelte";
    import Splash from "$lib/components/Splash.svelte";
    import type { PageData } from "./$types";

    let { data }: { data: PageData } = $props();
</script>

{#if data.user}
    <Dashboard user={data.user} />
{:else}
    <Splash />
{/if}
