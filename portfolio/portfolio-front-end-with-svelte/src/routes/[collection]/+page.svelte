<!--
  Work-item list route (`/[collection]`).

  Purpose: fetch and render the collection's work-item refs as links, with
  a name-search box.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { WorkItemRepository } from "$lib/api/work-items";
  import type { WorkItemRef } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
  const collection = $derived(data.collection);
  const repo = $derived(WorkItemRepository.withFetch(collection));

  let items = $state<WorkItemRef[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state("");
  let searching = $state(false);

  onMount(async () => {
    try {
      items = await repo.list();
    } catch (err) {
      error = err instanceof Error ? err.message : t("list.loadFailed");
    } finally {
      loading = false;
    }
  });

  async function runSearch(event: SubmitEvent) {
    event.preventDefault();
    error = null;
    searching = true;
    try {
      items = query.trim().length > 0 ? await repo.search(query.trim()) : await repo.list();
    } catch (err) {
      error = err instanceof Error ? err.message : t("list.loadFailed");
    } finally {
      searching = false;
    }
  }
</script>

<svelte:head><title>{collection} — Main X</title></svelte:head>

<h1>{t("list.title")} — {collection}</h1>
<p><a class="button" href={`/${collection}/new`}>{t("list.new")}</a></p>

<form class="row" onsubmit={runSearch}>
  <input type="search" bind:value={query} placeholder={t("list.title")} aria-label={t("list.title")} />
  <button class="button" type="submit" disabled={searching}>{t("detail.checkDuplicates")}</button>
</form>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if items.length === 0}
  <p class="surface">{t("list.empty")} <a href={`/${collection}/new`}>{t("list.createOne")}</a>.</p>
{:else}
  <ul class="stack">
    {#each items as record (record.pid)}
      <li class="surface row">
        <a href={`/${collection}/${record.pid}`}>{record.name}</a>
        <code>{record.pid}</code>
      </li>
    {/each}
  </ul>
{/if}
