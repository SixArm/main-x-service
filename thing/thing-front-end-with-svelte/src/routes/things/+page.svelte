<!--
  +page.svelte (/things) — Things list / search page.

  Purpose: search the index (with fuzzy / phonetic toggles) and show results
  in the SVAR ThingGrid; selecting a row navigates to that Thing's detail.

  $state:
    - query: bound search text.
    - things / total: current result set and count.
    - loading / error: request status.
    - fuzzy / phonetic: search-mode toggles passed to the API.

  Reactive notes: an $effect runs the initial unfiltered search once on mount
  ("*" wildcard); subsequent searches are user-triggered via runSearch.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import ThingGrid from "$lib/components/ThingGrid.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import { describeApiError } from "$lib/api/errorHandling.js";
    import type { Thing } from "$lib/api/types.js";
    import { t, translate } from "$lib/i18n.svelte.js";

    let query = $state("");
    let things = $state<Thing[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);

    const repo = ThingRepository.withFetch();

    // Execute a search; empty input falls back to "*" to list everything.
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({
                q: q || "*",
                limit: 50,
                fuzzy,
                phonetic,
            });
            things = res.items;
            total = res.total;
        } catch (err) {
            error = describeApiError(err);
            things = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    // Navigate to the detail page for a grid-selected Thing.
    function openThing(thing: Thing) {
        if (thing.id) goto(`/things/${thing.id}`);
    }

    // Populate the grid with an initial wildcard search on first render.
    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Things · Thing Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("things.title")}</h1>
    <a href="/things/new" class="button primary">{t("things.new")}</a>
</header>

<section class="surface stack">
    <SearchBox
        bind:value={query}
        placeholder={t("things.searchPlaceholder")}
        onsearch={runSearch}
    />
    <div class="row small">
        <label
            ><input type="checkbox" bind:checked={fuzzy} />
            {t("things.fuzzy")}</label
        >
        <label
            ><input type="checkbox" bind:checked={phonetic} />
            {t("things.phonetic")}</label
        >
        <span class="muted" style="margin-left: auto">
            {loading
                ? t("things.loading")
                : translate(
                      total === 1
                          ? "things.recordCount"
                          : "things.recordCountPlural",
                  ).replace("{count}", String(total))}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <ThingGrid {things} onselect={openThing} />
</section>
