<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import PlaceGrid from "$lib/components/PlaceGrid.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import type { Place } from "$lib/api/types.js";

    let query = $state("");
    let places = $state<Place[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);

    const repo = PlaceRepository.withFetch();

    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({ q: q || "*", limit: 50, fuzzy, phonetic });
            places = res.items;
            total = res.total;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            places = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    function openPlace(place: Place) {
        if (place.id) goto(`/places/${place.id}`);
    }

    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Places · Place Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Places</h1>
    <a href="/places/new" class="button primary">New place</a>
</header>

<section class="surface stack">
    <SearchBox bind:value={query} placeholder="Search by name, locality, identifier…" onsearch={runSearch} />
    <div class="row small">
        <label><input type="checkbox" bind:checked={fuzzy} /> Fuzzy</label>
        <label><input type="checkbox" bind:checked={phonetic} /> Phonetic (Soundex)</label>
        <span class="muted" style="margin-left: auto">
            {loading ? "Loading…" : `${total} place${total === 1 ? "" : "s"}`}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <PlaceGrid {places} onselect={openPlace} />
</section>
