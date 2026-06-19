<!--
  Edit-work-item route (`/[collection]/[pid]/edit`).

  Loads the existing work item, seeds a WorkItemForm with it, and PUTs the
  edited record on submit before returning to the detail page.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import WorkItemForm from "$lib/components/WorkItemForm.svelte";
  import { WorkItemRepository } from "$lib/api/work-items";
  import { KIND_OF_COLLECTION } from "$lib/api/types";
  import type { WorkItem } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
  const collection = $derived(data.collection);
  const kind = $derived(KIND_OF_COLLECTION[collection]);
  const pid = page.params.pid ?? "";
  const repo = $derived(WorkItemRepository.withFetch(collection));

  let record = $state<WorkItem | null>(null);
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

  async function handleSubmit(updated: WorkItem) {
    await repo.update(pid, updated);
    await goto(`/${collection}/${pid}`);
  }
</script>

<svelte:head><title>{t("edit.title")} — Main X</title></svelte:head>

<h1>{t("edit.title")}</h1>

{#if loading}
  <p>{t("edit.loading")}</p>
{:else if error}
  <p class="banner" role="alert">{error}</p>
{:else if record}
  <WorkItemForm {kind} initial={record} submitLabel={t("edit.saveChanges")} onsubmit={handleSubmit} />
{/if}
