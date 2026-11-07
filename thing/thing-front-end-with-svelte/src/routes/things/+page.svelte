<!--
  +page.svelte (/things) — Things list / search page.

  Purpose: search the index (with fuzzy / phonetic / mask-sensitive toggles)
  and show results in the SVAR ThingGrid; selecting a row navigates to that
  Thing's detail.

  $state:
    - query: bound search text.
    - things: every row loaded so far; "Load more" appends the next
      page rather than replacing this one.
    - next: the next page's URL from `Link: rel="next"`, or null on the
      last page. The service sends no total (a count would describe
      rows the caller's policy may hide), so the shown count is
      `{count}+` while more follow — see `ThingRepository.list`.
    - loading / error: request status.
    - fuzzy / phonetic: search-mode toggles passed to the API.
    - maskSensitive (T-27): asks the server to mask sensitive fields in
      the returned records, mirroring the detail page's masked-view
      toggle (T-19) — the server decides what counts as sensitive, not
      this page. Unlike fuzzy/phonetic (which only take effect on the
      next manual search), toggling this one re-fetches immediately
      (same page), so switching views doesn't require re-submitting
      the query too.

  Reactive notes: an $effect runs the initial unfiltered list once on
  mount (T-15: `repo.list`, not a "*" wildcard search — see runSearch);
  subsequent searches are user-triggered via runSearch (submit, or the
  mask-sensitive toggle's own onchange). A new query or toggle always
  starts back at the first page; only "Load more" follows `next`, and
  the cursor it carries is signed for that query alone.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import SearchBox from "#lib/components/SearchBox.svelte";
    import ThingGrid from "#lib/components/ThingGrid.svelte";
    import { ThingRepository } from "#lib/api/things.js";
    import { describeApiError } from "#lib/api/errorHandling.js";
    import type { Thing } from "#lib/api/types.js";
    import { t, translate } from "#lib/i18n.svelte.js";

    const PAGE_SIZE = 50;

    let query = $state("");
    let things = $state<Thing[]>([]);
    let next = $state<string | null>(null);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let fuzzy = $state(true);
    let phonetic = $state(false);
    let maskSensitive = $state(false);

    const repo = ThingRepository.withFetch();
    // While more rows follow, the count is a lower bound.
    const countLabel = $derived(
        next ? `${things.length}+` : String(things.length),
    );

    // Execute a search, or — when the input is empty — enumerate the
    // collection directly via `repo.list` (T-15: `/things/search` has
    // no way to mean "everything"; a bare `q="*"` tokenises to nothing
    // server-side and always returns zero hits, regardless of how many
    // records exist). Always the first page: a new query or toggle
    // change starts over.
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const trimmed = q.trim();
            const res = trimmed
                ? await repo.search({
                      q: trimmed,
                      limit: PAGE_SIZE,
                      fuzzy,
                      phonetic,
                      mask_sensitive: maskSensitive,
                  })
                : await repo.list({
                      limit: PAGE_SIZE,
                      mask_sensitive: maskSensitive,
                  });
            things = res.items;
            next = res.next;
        } catch (err) {
            error = describeApiError(err);
            things = [];
            next = null;
        } finally {
            loading = false;
        }
    }

    // Append the page `next` names to the rows already shown.
    async function loadMore() {
        if (!next) return;
        loading = true;
        error = null;
        try {
            const res = await repo.list({ next });
            things = [...things, ...res.items];
            next = res.next;
        } catch (err) {
            error = describeApiError(err);
        } finally {
            loading = false;
        }
    }

    // Navigate to the detail page for a grid-selected Thing.
    function openThing(thing: Thing) {
        if (thing.id) goto(`/things/${thing.id}`);
    }

    // Populate the grid on first render — an empty query routes to
    // `repo.list` inside runSearch (T-15), not a wildcard search.
    $effect(() => {
        // Site-wide search (PickerBar) navigates here as `?<terms>`.
        const q = decodeURIComponent(page.url.search.slice(1));
        query = q;
        void runSearch(q);
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
                onchange={() => runSearch(query)}
            />
            {t("things.maskSensitive")}</label
        >
        <span class="muted" style="margin-left: auto">
            {loading
                ? t("things.loading")
                : translate(
                      things.length === 1 && !next
                          ? "things.recordCount"
                          : "things.recordCountPlural",
                  ).replace("{count}", countLabel)}
        </span>
    </div>
    {#if error}
        <div class="banner error">{error}</div>
    {/if}
    <ThingGrid {things} onselect={openThing} />
    {#if next}
        <div class="row small">
            <button
                type="button"
                class="button"
                disabled={loading}
                onclick={loadMore}
            >
                {t("list.more")}
            </button>
        </div>
    {/if}
</section>
