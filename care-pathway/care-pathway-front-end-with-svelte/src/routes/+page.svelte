<script lang="ts">
    import { onMount } from "svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { PathwayRef } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();

    let pathways = $state<PathwayRef[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            pathways = await repo.list();
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load care pathways";
        } finally {
            loading = false;
        }
    });
</script>

<svelte:head><title>Care pathways — Main X</title></svelte:head>

<h1>Care pathways</h1>
<p><a class="button" href="/new">New care pathway</a></p>

{#if loading}
    <p>Loading…</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if pathways.length === 0}
    <p class="surface">No care pathways yet. <a href="/new">Create one</a>.</p>
{:else}
    <ul class="stack">
        {#each pathways as pathway (pathway.pid)}
            <li class="surface row">
                <a href={`/${pathway.pid}`}>{pathway.name}</a>
                <code>{pathway.pid}</code>
            </li>
        {/each}
    </ul>
{/if}
