<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import PersonGrid from "$lib/components/PersonGrid.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import type { Person } from "$lib/api/types.js";

    let query = $state("");
    let persons = $state<Person[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);

    const repo = PersonRepository.withFetch();

    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({ q: q || "*", limit: 50, fuzzy, phonetic });
            persons = res.items;
            total = res.total;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            persons = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    function openPerson(person: Person) {
        if (person.id) goto(`/persons/${person.id}`);
    }

    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Persons · Person Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Persons</h1>
    <a href="/persons/new" class="button primary">New person</a>
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
    <PersonGrid {persons} onselect={openPerson} />
</section>
