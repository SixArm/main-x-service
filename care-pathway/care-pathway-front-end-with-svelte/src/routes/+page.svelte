<script lang="ts">
    // List route ("/") — the care-pathway index.
    //
    // Purpose: load and list all pathways on mount, offer a name search box
    // (with a Clear that restores the full list), and a lazily-loaded
    // "Recent activity" panel over the system-wide event stream.
    //
    // State ($state): `pathways` (current list, list OR search results),
    // `loading`/`searching` (separate busy flags for first paint vs.
    // search), `error`, `query` (search box), plus the `showEvents` /
    // `events` / `eventsLoading` / `eventsError` group for the activity
    // panel. No props; no $derived (the markup branches on the flags).
    import { onMount } from "svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { PathwayEvent, PathwayRef } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();

    let pathways = $state<PathwayRef[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);
    let query = $state("");
    let searching = $state(false);

    // Recent activity: the system-wide event stream, lazy-loaded behind a
    // toggle so the list stays lean on first paint.
    let showEvents = $state(false);
    let events = $state<PathwayEvent[] | null>(null);
    let eventsLoading = $state(false);
    let eventsError = $state<string | null>(null);

    // First paint: load the full list.
    onMount(async () => {
        try {
            pathways = await repo.list();
        } catch (err) {
            error = err instanceof Error ? err.message : "Failed to load care pathways";
        } finally {
            loading = false;
        }
    });

    // Search submit: an empty query falls back to the full list; otherwise
    // hit the name-search endpoint. Prevents the native form GET.
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

    // Clear the search box and restore the full list.
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

    /// Toggle the recent-activity panel; lazy-load the stream on first open.
    async function toggleEvents() {
        showEvents = !showEvents;
        // Fetch only when opening, and only once (cached in `events`) or
        // when not already in flight.
        if (!showEvents || events !== null || eventsLoading) return;
        eventsLoading = true;
        eventsError = null;
        try {
            const rows = await repo.recentEvents();
            // Newest-first by sequence number (the service returns them
            // oldest-first / highest seq last).
            events = [...rows].sort((a, b) => b.seq - a.seq);
        } catch (err) {
            eventsError = err instanceof Error ? err.message : "Recent activity load failed";
        } finally {
            eventsLoading = false;
        }
    }
</script>

<svelte:head><title>Care pathways — Main X</title></svelte:head>

<h1>Care pathways</h1>
<p><a class="button" href="/new">New care pathway</a></p>

<!-- Name search box; Clear appears only when there is a query to clear. -->
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

<div class="row" style="margin-top:1rem">
    <button class="button" onclick={toggleEvents}>
        {showEvents ? "Hide recent activity" : "Show recent activity"}
    </button>
</div>

<!-- Recent-activity panel: rendered only when toggled open; rows are
     newest-first by `seq` (sorted in `toggleEvents`). -->
{#if showEvents}
    <h2>Recent activity</h2>
    {#if eventsLoading}
        <p>Loading recent activity…</p>
    {:else if eventsError}
        <p class="banner error" role="alert">{eventsError}</p>
    {:else if events && events.length > 0}
        <ul class="stack">
            {#each events as event (event.seq)}
                <li class="surface row">
                    <strong>{event.kind}</strong>
                    <a href={`/${event.pid}`}>{event.name}</a>
                    <span>#{event.seq}</span>
                </li>
            {/each}
        </ul>
    {:else}
        <p>No recent activity.</p>
    {/if}
{/if}
