<!--
  Edit-case route (`/[pid]/edit`).

  Purpose: load the existing case, seed a `CaseForm` with it, and PUT the
  edited record on submit before returning to the detail page.

  State: `record` (the loaded case used as the form seed), `loading`,
  `error` — gating the form behind the load phase below.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import CaseForm from "$lib/components/CaseForm.svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { Case } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();
  // Route param; `?? ""` satisfies strict typing (params may be undefined).
  const pid = page.params.pid ?? "";

  let record = $state<Case | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Fetch the case to edit on mount; the form mounts only once loaded.
  onMount(async () => {
    try {
      record = await repo.get(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : t("edit.notFound");
    } finally {
      loading = false;
    }
  });

  // Persist the edited record, then return to its detail page.
  async function handleSubmit(updated: Case) {
    await repo.update(pid, updated);
    await goto(`/${pid}`);
  }
</script>

<svelte:head><title>Edit {record?.title ?? "case"} — Main X</title></svelte:head
>

<h1>{t("edit.title")}</h1>

{#if loading}
  <p>{t("edit.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <CaseForm
    initial={record}
    submitLabel={t("edit.saveChanges")}
    onsubmit={handleSubmit}
  />
{/if}
