<!--
  Cases list route (`/`).

  Purpose: fetch and render all case refs as links.
  State: `cases` (the loaded refs), `loading`, `error` — driving the
  loading / error / empty / list branches in the markup below.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { CaseRef } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();

  let cases = $state<CaseRef[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Load the collection once on mount; always clear `loading` afterward.
  onMount(async () => {
    try {
      cases = await repo.list();
    } catch (err) {
      error = err instanceof Error ? err.message : t("list.loadFailed");
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head><title>Cases — Main X</title></svelte:head>

<h1>{t("list.title")}</h1>
<p><a class="button" href="/new">{t("list.new")}</a></p>

{#if loading}
  <p>{t("list.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if cases.length === 0}
  <p class="surface">{t("list.empty")} <a href="/new">{t("list.createOne")}</a>.</p>
{:else}
  <ul class="stack">
    {#each cases as record (record.pid)}
      <li class="surface row">
        <a href={`/${record.pid}`}>{record.title}</a>
        <code>{record.pid}</code>
      </li>
    {/each}
  </ul>
{/if}
