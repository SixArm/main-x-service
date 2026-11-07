<!--
  Cases list dashboard — the signed-in home (`/`).

  Purpose: fetch and render case refs as links, with a search box wired
  to the Tantivy full-text search endpoint. A blank query shows every
  active case (`GET /api/cases`); a non-blank query runs `GET
  /api/cases/search?q=` (with optional fuzzy/phonetic toggles), since the
  service rejects a blank `q` with `400`.

  State:
    - query / fuzzy / phonetic — the search inputs.
    - cases / next — the rows shown so far, and the next page's URL from
      `Link: rel="next"` (`null` on the last page). There is no total, so
      the count shows "+" while more follow, with a "Load more" button.
    - loading / error — request status, driving the loading / error /
      empty / list branches in the markup below.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CaseRepository } from "#lib/api/cases.js";
  import SearchBox from "#lib/components/SearchBox.svelte";
  import type { CaseRef } from "#lib/api/types.js";
  import { t } from "#lib/i18n.svelte.js";

  const repo = CaseRepository.withFetch();

  let query = $state("");
  let fuzzy = $state(false);
  let phonetic = $state(false);
  let cases = $state<CaseRef[]>([]);
  let next = $state<string | null>(null);
  // The trimmed query `next` belongs to: "" for the plain list.
  let shown = $state("");
  let loading = $state(true);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);

  // Run a search (or, for a blank query, the plain list) and update state.
  async function runSearch(q: string) {
    loading = true;
    error = null;
    try {
      const trimmed = q.trim();
      const page =
        trimmed === ""
          ? await repo.listPage()
          : await repo.search({ q: trimmed, fuzzy, phonetic });
      cases = page.items;
      next = page.next;
      shown = trimmed;
    } catch (err) {
      error = err instanceof Error ? err.message : t("search.failed");
      cases = [];
      next = null;
    } finally {
      loading = false;
    }
  }

  // Append the page after the last one shown, following its `next` URL,
  // which already carries the query, the limit and the signed cursor.
  async function loadMore() {
    if (!next || loadingMore) return;
    const from = next;
    loadingMore = true;
    try {
      const page =
        shown === ""
          ? await repo.listPage({}, from)
          : await repo.search({ q: shown }, from);
      // A new search may have replaced the list while this was in flight.
      if (next !== from) return;
      cases = [...cases, ...page.items];
      next = page.next;
    } catch (err) {
      error = err instanceof Error ? err.message : t("search.failed");
    } finally {
      loadingMore = false;
    }
  }

  // Load the collection once on mount (empty query ⇒ "show all").
  onMount(() => {
    void runSearch("");
  });
</script>

<h1>{t("list.title")}</h1>
<p><a class="button" href="/new">{t("list.new")}</a></p>

<div class="surface stack" style="margin-bottom:1rem">
  <SearchBox bind:value={query} onsearch={runSearch} />
  <div class="row small">
    <label><input type="checkbox" bind:checked={fuzzy} /> {t("search.fuzzy")}</label>
    <label><input type="checkbox" bind:checked={phonetic} /> {t("search.phonetic")}</label>
  </div>
</div>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if cases.length === 0}
  <p class="surface">{t("list.empty")} <a href="/new">{t("list.createOne")}</a>.</p>
{:else}
  <p class="count">{next ? `${cases.length}+` : `${cases.length}`}</p>
  <ul class="stack">
    {#each cases as record (record.pid)}
      <li class="surface row">
        <a href={`/${record.pid}`}>{record.title}</a>
        <code>{record.pid}</code>
      </li>
    {/each}
  </ul>
  {#if next}
    <button type="button" onclick={loadMore} disabled={loadingMore}>
      {t("list.more")}
    </button>
  {/if}
{/if}

<style>
  .count {
    margin: 0 0 0.5rem;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }
</style>
