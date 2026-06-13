<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import CaseForm from "$lib/components/CaseForm.svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { Case } from "$lib/api/types";

  const repo = CaseRepository.withFetch();
  const pid = page.params.pid ?? "";

  let record = $state<Case | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      record = await repo.get(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : "Not found";
    } finally {
      loading = false;
    }
  });

  async function handleSubmit(updated: Case) {
    await repo.update(pid, updated);
    await goto(`/${pid}`);
  }
</script>

<svelte:head><title>Edit {record?.title ?? "case"} — Main X</title></svelte:head
>

<h1>Edit case</h1>

{#if loading}
  <p>Loading…</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <CaseForm
    initial={record}
    submitLabel="Save changes"
    onsubmit={handleSubmit}
  />
{/if}
