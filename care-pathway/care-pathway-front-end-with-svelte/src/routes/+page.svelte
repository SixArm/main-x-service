<script lang="ts">
    import { onMount } from "svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { PathwayRef } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();

    let pathways = $state<PathwayRef[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let query = $state("");
    let searching = $state(false);

    onMount(async () => {
        try {
            pathways = await repo.list();
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load care pathways";
        } finally {
            loading = false;
        }
    });

    async function runSearch(event: SubmitEvent) {
        event.preventDefault();
        const q = query.trim();
        error = null;
        searching = true;
        try {
            pathways = q === "" ? await repo.list() : await repo.search(q);
        } catch (err) {
            error = err instanceof Error ? err.message : "Search failed";
        } finally {
            searching = false;
        }
    }

    async function clearSearch() {
        query = "";
        error = null;
        searching = true;
        try {
            pathways = await repo.list();
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load care pathways";
        } finally {
            searching = false;
        }
    }
</script>

<svelte:head><title>Care pathways — Main X</title></svelte:head>

<h1>Care pathways</h1>
<p><a class="button" href="/new">New care pathway</a></p>

<form class="row" onsubmit={runSearch} role="search">
    <input
        type="search"
        name="q"
        bind:value={query}
        placeholder="Search by name…"
        aria-label="Search care pathways by name"
    />
    <button class="button primary" type="submit" disabled={searching}>Search</button>
    {#if query.trim() !== ""}
        <button class="button" type="button" onclick={clearSearch} disabled={searching}>Clear</button>
    {/if}
</form>

{#if loading || searching}
    <p>Loading…</p>
{:else if error}
    <p class="banner error" role="alert">{error}</p>
{:else if pathways.length === 0}
    {#if query.trim() !== ""}
        <p class="surface">No care pathways match “{query.trim()}”.</p>
    {:else}
        <p class="surface">No care pathways yet. <a href="/new">Create one</a>.</p>
    {/if}
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
