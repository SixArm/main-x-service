<!--
  Persons list (/persons) — searchable, paginated grid of persons.

  Wraps SearchBox (query + fuzzy/phonetic toggles) over PersonGrid.
  Selecting a row navigates to that person's detail page.

  State:
    - query — current search text (bound to SearchBox).
    - persons / total — current result page and hit count.
    - loading / error — request lifecycle.
    - fuzzy / phonetic — search-mode toggles passed to the API.
-->
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

    // Run a search and fold the result into the grid state. On error, clear
    // the grid so stale results aren't shown alongside the error banner.
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            // Empty query becomes "*" so the initial load lists everything.
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

    // Navigate to the detail page for the selected row (ignore unsaved rows).
    function openPerson(person: Person) {
        if (person.id) goto(`/persons/${person.id}`);
    }

    // Initial load: list all persons once on mount (SSR is disabled, so no
    // +page.ts load function). Runs once because it reads no reactive deps.
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
