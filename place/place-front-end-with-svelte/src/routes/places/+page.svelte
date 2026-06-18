<!--
  Places index (route "/places") — searchable, paginated grid of places.

  Local $state:
    - query           — current search text (bound to SearchBox).
    - places / total  — current result page and its total count.
    - loading / error — request status surfaced in the UI.
    - fuzzy / phonetic — search toggles forwarded to the search endpoint.

  Selecting a grid row navigates to that place's detail page.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import PlaceGrid from "$lib/components/PlaceGrid.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { Place } from "$lib/api/types.js";

    let query = $state("");
    let places = $state<Place[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);

    // One repository instance for this page (no global HTTP store).
    const repo = PlaceRepository.withFetch();

    // Fetch a page of results; `*` is the "match all" query for empty input.
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

    // Row-select handler: navigate to the place detail page.
    function openPlace(place: Place) {
        if (place.id) goto(`/places/${place.id}`);
    }

    // Initial load: list everything once on mount. The effect has no
    // reactive deps inside the call, so it runs a single time.
    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Places · Place Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("places.title")}</h1>
    <a href="/places/new" class="button primary">{t("places.new")}</a>
</header>

<section class="surface stack">
    <SearchBox bind:value={query} placeholder={t("places.searchPlaceholder")} onsearch={runSearch} />
    <div class="row small">
        <label><input type="checkbox" bind:checked={fuzzy} /> {t("places.fuzzy")}</label>
        <label><input type="checkbox" bind:checked={phonetic} /> {t("places.phonetic")}</label>
        <span class="muted" style="margin-left: auto">
            {loading ? t("places.loading") : `${total} ${total === 1 ? t("places.countOne") : t("places.countMany")}`}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <PlaceGrid {places} onselect={openPlace} />
</section>
