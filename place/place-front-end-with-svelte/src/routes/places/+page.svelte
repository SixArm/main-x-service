<!--
  Places index (route "/places") — searchable, paginated grid of places.

  Local $state:
    - query           — current search text (bound to SearchBox).
    - places / total  — current result page and its total count.
    - loading / error — request status surfaced in the UI.
    - fuzzy / phonetic — search toggles forwarded to the search endpoint;
      take effect on the next explicit search submit.
    - maskSensitive   — mask_sensitive toggle (T-27); re-fetches
      immediately on change, mirroring the detail page's masked-view
      toggle (T-19).

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
    let maskSensitive = $state(false);

    // One repository instance for this page (no global HTTP store).
    const repo = PlaceRepository.withFetch();

    // Fetch a page of results; `*` is the "match all" query for empty input.
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({
                q: q || "*",
                limit: 50,
                fuzzy,
                phonetic,
                mask_sensitive: maskSensitive,
            });
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

    // Unlike fuzzy/phonetic (which only take effect on the next explicit
    // search submit), masking re-fetches immediately on toggle — the same
    // "flip and re-fetch now" behaviour the detail page's masked-view
    // toggle (T-19) uses, since it's a view choice rather than a search
    // strategy the operator is still composing.
    function toggleMaskSensitive() {
        maskSensitive = !maskSensitive;
        void runSearch(query);
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
    <SearchBox
        bind:value={query}
        placeholder={t("places.searchPlaceholder")}
        onsearch={runSearch}
    />
    <div class="row small">
        <label
            ><input type="checkbox" bind:checked={fuzzy} />
            {t("places.fuzzy")}</label
        >
        <label
            ><input type="checkbox" bind:checked={phonetic} />
            {t("places.phonetic")}</label
        >
        <label
            ><input
                type="checkbox"
                checked={maskSensitive}
                onchange={toggleMaskSensitive}
            />
            {t("places.maskSensitive")}</label
        >
        <span class="muted" style="margin-left: auto">
            {loading
                ? t("places.loading")
                : `${total} ${total === 1 ? t("places.countOne") : t("places.countMany")}`}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <PlaceGrid {places} onselect={openPlace} />
</section>
