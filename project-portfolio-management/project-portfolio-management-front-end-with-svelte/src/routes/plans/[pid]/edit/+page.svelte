<!--
  Edit-plan route (`/plans/[pid]/edit`).

  Loads the existing plan, seeds a PlanForm with it, and PUTs the edited
  record on submit before returning to the detail page.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import PlanForm from "$lib/components/PlanForm.svelte";
  import { PlanRepository } from "$lib/api/plans";
  import type { Plan } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const pid = page.params.pid ?? "";
  const repo = PlanRepository.withFetch();

  let record = $state<Plan | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      record = await repo.get(pid);
    } catch (err) {
      error = err instanceof Error ? err.message : t("edit.notFound");
    } finally {
      loading = false;
    }
  });

  async function handleSubmit(updated: Plan) {
    await repo.update(pid, updated);
    await goto(`/plans/${pid}`);
  }
</script>

<svelte:head><title>{t("edit.title")} — Main X</title></svelte:head>

<h1>{t("edit.title")}</h1>

{#if loading}
  <p>{t("edit.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <PlanForm initial={record} submitLabel={t("edit.saveChanges")} onsubmit={handleSubmit} />
{/if}
