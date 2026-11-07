<!--
  Places index (route "/places") — searchable, paginated grid of places.

  Local $state:
    - query           — current search text (bound to SearchBox).
    - places / next   — the rows loaded so far, and the next page's path
      (`Link: rel="next"`), or null on the last. There is no total, so
      the count reads "N+" while more follow, with a "Load more" button.
    - loading / error — request status surfaced in the UI.
    - fuzzy / phonetic — search toggles forwarded to the search endpoint;
      take effect on the next explicit search submit.
    - maskSensitive   — mask_sensitive toggle (T-27); re-fetches
      immediately on change, mirroring the detail page's masked-view
      toggle (T-19).

  Selecting a grid row navigates to that place's detail page.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import SearchBox from "#lib/components/SearchBox.svelte";
    import PlaceGrid from "#lib/components/PlaceGrid.svelte";
    import { PlaceRepository } from "#lib/api/places.js";
    import { t } from "#lib/i18n.svelte.js";
    import type { Place } from "#lib/api/types.js";

    let query = $state("");
    let places = $state<Place[]>([]);
    let next = $state<string | null>(null);
    let loading = $state(false);
    let loadingMore = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);
    let maskSensitive = $state(false);

    // One repository instance for this page (no global HTTP store).
    const repo = PlaceRepository.withFetch();

    // Fetch a page of results, or — when the input is empty — enumerate
    // the collection directly via `repo.list` (mirrors thing-front-end's
    // T-15/T-31 fix: `/places/search` has no way to mean "everything";
    // a bare `q="*"` tokenises to nothing server-side and always
    // returns zero hits, regardless of how many records exist).
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const trimmed = q.trim();
            const res = trimmed
                ? await repo.search({
                      q: trimmed,
                      limit: 50,
                      fuzzy,
                      phonetic,
                      mask_sensitive: maskSensitive,
                  })
                : await repo.list({ limit: 50, mask_sensitive: maskSensitive });
            places = res.items;
            next = res.next;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            places = [];
            next = null;
        } finally {
            loading = false;
        }
    }

    // Append the next page; its path carries the query and signed cursor.
    async function loadMore() {
        if (!next || loadingMore) return;
        loadingMore = true;
        try {
            const res = await repo.nextPage(next);
            places = [...places, ...res.items];
            next = res.next;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loadingMore = false;
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

    // Initial load: list everything once on mount, via `repo.list`
    // inside runSearch (not a wildcard search). The effect has no
    // reactive deps inside the call, so it runs a single time.
    $effect(() => {
        // Site-wide search (PickerBar) navigates here as `?<terms>`.
        const q = decodeURIComponent(page.url.search.slice(1));
        query = q;
        void runSearch(q);
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
                : `${places.length}${next ? "+" : ""} ${places.length === 1 && !next ? t("places.countOne") : t("places.countMany")}`}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <PlaceGrid {places} onselect={openPlace} />
    {#if next}
        <button type="button" onclick={loadMore} disabled={loadingMore}>
            {t("list.more")}
        </button>
    {/if}
</section>
