<!--
  Cases list route (`/`).

  Purpose: fetch and render case refs as links, with a search box wired
  to the Tantivy full-text search endpoint. A blank query shows every
  active case (`GET /api/cases`); a non-blank query runs `GET
  /api/cases/search?q=` (with optional fuzzy/phonetic toggles), since the
  service rejects a blank `q` with `400`.

  State:
    - query / fuzzy / phonetic — the search inputs.
    - cases / total — the last result set and its overall count.
    - loading / error — request status, driving the loading / error /
      empty / list branches in the markup below.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CaseRepository } from "$lib/api/cases";
  import SearchBox from "$lib/components/SearchBox.svelte";
  import type { CaseRef } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();

  let query = $state("");
  let fuzzy = $state(false);
  let phonetic = $state(false);
  let cases = $state<CaseRef[]>([]);
  let total = $state(0);
  let loading = $state(true);
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
      total = page.total;
    } catch (err) {
      error = err instanceof Error ? err.message : t("search.failed");
      cases = [];
      total = 0;
    } finally {
      loading = false;
    }
  }

  // Load the collection once on mount (empty query ⇒ "show all").
  onMount(() => {
    void runSearch("");
  });
</script>

<svelte:head><title>Cases — Main X</title></svelte:head>

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
  <p class="count">
    {cases.length === total ? `${total}` : `${cases.length} / ${total}`}
  </p>
  <ul class="stack">
    {#each cases as record (record.pid)}
      <li class="surface row">
        <a href={`/${record.pid}`}>{record.title}</a>
        <code>{record.pid}</code>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .count {
    margin: 0 0 0.5rem;
    opacity: 0.75;
    font-variant-numeric: tabular-nums;
  }
</style>
