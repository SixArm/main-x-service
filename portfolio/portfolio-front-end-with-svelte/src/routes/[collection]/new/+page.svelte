<!--
  Create-work-item route (`/[collection]/new`).

  Renders an empty WorkItemForm seeded with the collection's kind and, on
  submit, POSTs it then navigates to the new record's detail page.
-->
<script lang="ts">
  import { goto } from "$app/navigation";
  import WorkItemForm from "$lib/components/WorkItemForm.svelte";
  import { WorkItemRepository } from "$lib/api/work-items";
  import { KIND_OF_COLLECTION } from "$lib/api/types";
  import type { WorkItem } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";
  import type { PageData } from "./$types";

  let { data }: { data: PageData } = $props();
  const collection = $derived(data.collection);
  const kind = $derived(KIND_OF_COLLECTION[collection]);
  const repo = $derived(WorkItemRepository.withFetch(collection));
  const initial = $derived<WorkItem>({ kind, name: "" });

  async function handleSubmit(record: WorkItem) {
    const created = await repo.create(record);
    await goto(`/${collection}/${created.pid}`);
  }
</script>

<svelte:head><title>{t("new.title")} — Main X</title></svelte:head>

<h1>{t("new.title")} — {collection}</h1>
<WorkItemForm {kind} {initial} submitLabel={t("new.create")} onsubmit={handleSubmit} />
