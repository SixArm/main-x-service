<!--
  List route (`/`): fetches and renders all organizations.

  $state:
    - orgs:    OrgRef[]      — the loaded collection.
    - loading: boolean       — true until the first fetch settles.
    - error:   string | null — fetch failure message (inline banner).

  Loads on mount (client-only; the app is SPA). The markup branches
  loading / error / empty / list.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { OrganizationRepository } from "$lib/api/organizations";
    import type { OrgRef } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();

    let orgs = $state<OrgRef[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // Fetch the collection once on mount; always clear `loading` in finally.
    onMount(async () => {
        try {
            orgs = await repo.list();
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load organizations";
        } finally {
            loading = false;
        }
    });
</script>

<svelte:head><title>Organizations — Main X</title></svelte:head>

<h1>Organizations</h1>
<p><a class="button" href="/new">New organization</a></p>

{#if loading}
    <p>Loading…</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if orgs.length === 0}
    <p class="surface">No organizations yet. <a href="/new">Create one</a>.</p>
{:else}
    <ul class="stack">
        {#each orgs as org (org.pid)}
            <li class="surface row">
                <a href={`/${org.pid}`}>{org.name}</a>
                <code>{org.pid}</code>
            </li>
        {/each}
    </ul>
{/if}
