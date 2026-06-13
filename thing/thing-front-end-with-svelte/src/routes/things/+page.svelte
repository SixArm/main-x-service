<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import ThingGrid from "$lib/components/ThingGrid.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import type { Thing } from "$lib/api/types.js";

    let query = $state("");
    let things = $state<Thing[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);

    const repo = ThingRepository.withFetch();

    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({ q: q || "*", limit: 50, fuzzy, phonetic });
            things = res.items;
            total = res.total;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            things = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    function openThing(thing: Thing) {
        if (thing.id) goto(`/things/${thing.id}`);
    }

    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Things · Thing Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Things</h1>
    <a href="/things/new" class="button primary">New thing</a>
</header>

<section class="surface stack">
    <SearchBox bind:value={query} placeholder="Search by name, identifier…" onsearch={runSearch} />
    <div class="row small">
        <label><input type="checkbox" bind:checked={fuzzy} /> Fuzzy</label>
        <label><input type="checkbox" bind:checked={phonetic} /> Phonetic (Soundex)</label>
        <span class="muted" style="margin-left: auto">
            {loading ? "Loading…" : `${total} record${total === 1 ? "" : "s"}`}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <ThingGrid {things} onselect={openThing} />
</section>
