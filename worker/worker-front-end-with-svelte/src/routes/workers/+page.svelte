<!--
  Workers list (route "/workers") — search box + result grid. Runs a search
  on load and on every submit; selecting a grid row navigates to detail.

  $state:
    - query — bound search text.
    - workers / total — current results and count.
    - loading / error — request status and failure message.
    - fuzzy / phonetic — search-mode toggles passed to the API.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import WorkerGrid from "$lib/components/WorkerGrid.svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { Worker } from "$lib/api/types.js";

    let query = $state("");
    let workers = $state<Worker[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);

    const repo = WorkerRepository.withFetch();

    // Run a search; an empty query becomes "*" to list everything. Resets
    // results on failure so stale rows aren't shown alongside an error.
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({ q: q || "*", limit: 50, fuzzy, phonetic });
            workers = res.items;
            total = res.total;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            workers = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    // Grid row selected → navigate to that worker's detail page.
    function openWorker(worker: Worker) {
        if (worker.id) goto(`/workers/${worker.id}`);
    }

    // Initial load: list all workers once the component is set up. (Uses a
    // bare query so it doesn't re-run when fuzzy/phonetic toggles change.)
    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Workers · Worker Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Workers</h1>
    <a href="/workers/new" class="button primary">New worker</a>
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
    <WorkerGrid {workers} onselect={openWorker} />
</section>
