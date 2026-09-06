<!--
  +page.svelte (/things) — Things list / search page.

  Purpose: search the index (with fuzzy / phonetic / mask-sensitive toggles)
  and show results in the SVAR ThingGrid; selecting a row navigates to that
  Thing's detail.

  $state:
    - query: bound search text.
    - things / total: current result set and count (T-28: `total`
      comes from the family-wide `X-Total-Count` header, ignoring
      `limit`/`offset` — see `ThingRepository.search`).
    - offset (T-28): the window's start, as actually applied by the
      service (`X-Offset`, which may differ from what was asked for).
    - loading / error: request status.
    - fuzzy / phonetic: search-mode toggles passed to the API.
    - maskSensitive (T-27): asks the server to mask sensitive fields in
      the returned records, mirroring the detail page's masked-view
      toggle (T-19) — the server decides what counts as sensitive, not
      this page. Unlike fuzzy/phonetic (which only take effect on the
      next manual search), toggling this one re-fetches immediately
      (same page), so switching views doesn't require re-submitting
      the query too.

  Reactive notes: an $effect runs the initial unfiltered search once on mount
  ("*" wildcard); subsequent searches are user-triggered via runSearch
  (submit, the mask-sensitive toggle's own onchange, or the
  previous/next page controls). A new query/toggle-submit always starts
  back at offset 0 (runSearch's default); only the pagination controls
  advance it.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import ThingGrid from "$lib/components/ThingGrid.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import { describeApiError } from "$lib/api/errorHandling.js";
    import type { Thing } from "$lib/api/types.js";
    import { t, translate } from "$lib/i18n.svelte.js";

    const PAGE_SIZE = 50;

    let query = $state("");
    let things = $state<Thing[]>([]);
    let total = $state(0);
    let offset = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);
    let maskSensitive = $state(false);

    const repo = ThingRepository.withFetch();
    const hasPreviousPage = $derived(offset > 0);
    const hasNextPage = $derived(offset + things.length < total);

    // Execute a search; empty input falls back to "*" to list everything.
    // `requestedOffset` defaults to 0 — a new query or toggle change
    // starts back at the first page; only the pagination controls pass
    // a non-zero value.
    async function runSearch(q: string, requestedOffset = 0) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({
                q: q || "*",
                limit: PAGE_SIZE,
                offset: requestedOffset,
                fuzzy,
                phonetic,
                mask_sensitive: maskSensitive,
            });
            things = res.items;
            total = res.total;
            // The service's actually-applied offset (X-Offset), not just
            // what was requested — it may clamp.
            offset = res.offset;
        } catch (err) {
            error = describeApiError(err);
            things = [];
            total = 0;
            offset = 0;
        } finally {
            loading = false;
        }
    }

    function previousPage() {
        void runSearch(query, Math.max(0, offset - PAGE_SIZE));
    }
    function nextPage() {
        void runSearch(query, offset + PAGE_SIZE);
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
        <label
            ><input
                type="checkbox"
                bind:checked={maskSensitive}
                onchange={() => runSearch(query, offset)}
            />
            {t("things.maskSensitive")}</label
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
    {#if total > 0}
        <div class="row small">
            <button
                type="button"
                class="button"
                disabled={!hasPreviousPage || loading}
                onclick={previousPage}
            >
                {t("things.previousPage")}
            </button>
            <button
                type="button"
                class="button"
                disabled={!hasNextPage || loading}
                onclick={nextPage}
            >
                {t("things.nextPage")}
            </button>
            <span class="muted" style="margin-left: auto">
                {translate("things.pageRange")
                    .replace("{from}", String(offset + 1))
                    .replace("{to}", String(offset + things.length))
                    .replace("{total}", String(total))}
            </span>
        </div>
    {/if}
</section>
