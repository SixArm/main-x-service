<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import { CaseRepository } from "$lib/api/cases";
  import type { Case, ScoredRef } from "$lib/api/types";

  const repo = CaseRepository.withFetch();
  const pid = page.params.pid ?? "";

  let record = $state<Case | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let duplicates = $state<ScoredRef[] | null>(null);
  let checking = $state(false);

  onMount(async () => {
    try {
      record = await repo.get(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : "Not found";
    } finally {
      loading = false;
    }
  });

  async function handleDelete() {
    await repo.remove(pid);
    await goto("/");
  }

  async function handleCheckDuplicates() {
    if (!record) return;
    checking = true;
    try {
      const hits = await repo.checkDuplicates(record);
      duplicates = hits.filter((h) => h.pid !== pid);
    } catch (err) {
      error = err instanceof Error ? err.message : "Check failed";
    } finally {
      checking = false;
    }
  }
</script>

<svelte:head><title>{record?.title ?? "Case"} — Main X</title></svelte:head>

{#if loading}
  <p>Loading…</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <h1>{record.title}</h1>
  <div class="surface stack">
    {#if record.case_type}
      <div>
        <strong>Case type:</strong>
        {typeof record.case_type === "string"
          ? record.case_type
          : record.case_type.Custom}
      </div>
    {/if}
    {#if record.status}
      <div>
        <strong>Status:</strong>
        {typeof record.status === "string"
          ? record.status
          : record.status.Custom}
      </div>
    {/if}
    {#if record.priority}<div>
        <strong>Priority:</strong>
        {record.priority}
      </div>{/if}
    {#if record.agency_name}<div>
        <strong>Agency:</strong>
        {record.agency_name}
      </div>{/if}
    {#if record.case_number}
      <div><strong>Case number:</strong> <code>{record.case_number}</code></div>
    {/if}
    {#if record.opened_date}<div>
        <strong>Opened:</strong>
        {record.opened_date}
      </div>{/if}
    {#if record.subjects && record.subjects.length > 0}
      <div><strong>Subjects:</strong> {record.subjects.join(", ")}</div>
    {/if}
    {#if record.identifiers && record.identifiers.length > 0}
      <div>
        <strong>Identifiers:</strong>
        <ul>
          {#each record.identifiers as id, i (i)}
            <li>
              {typeof id.scheme === "string"
                ? id.scheme
                : `Custom(${id.scheme.Custom})`}:
              <code>{id.value}</code>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
    {#if record.keywords && record.keywords.length > 0}
      <div><strong>Keywords:</strong> {record.keywords.join(", ")}</div>
    {/if}
    <div><strong>ID:</strong> <code>{pid}</code></div>
  </div>

  <div class="row" style="margin-top:1rem">
    <a class="button" href={`/${pid}/edit`}>Edit</a>
    <button class="button" onclick={handleCheckDuplicates} disabled={checking}>
      {checking ? "Checking…" : "Check duplicates"}
    </button>
    <button onclick={handleDelete}>Delete</button>
  </div>

  {#if duplicates}
    <h2>Potential duplicates</h2>
    {#if duplicates.length === 0}
      <p>None above the match threshold.</p>
    {:else}
      <ul class="stack">
        {#each duplicates as dup (dup.pid)}
          <li class="surface row">
            <a href={`/${dup.pid}`}>{dup.title}</a>
            <span>{dup.score.toFixed(3)} · {dup.confidence}</span>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
{/if}
