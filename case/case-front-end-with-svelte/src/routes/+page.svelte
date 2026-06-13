<script lang="ts">
  import { onMount } from "svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { CaseRef } from "$lib/api/types";

  const repo = CaseRepository.withFetch();

  let cases = $state<CaseRef[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      cases = await repo.list();
    } catch (err) {
      error = err instanceof Error ? err.message : "Failed to load cases";
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head><title>Cases — Main X</title></svelte:head>

<h1>Cases</h1>
<p><a class="button" href="/new">New case</a></p>

{#if loading}
  <p>Loading…</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if cases.length === 0}
  <p class="surface">No cases yet. <a href="/new">Create one</a>.</p>
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
