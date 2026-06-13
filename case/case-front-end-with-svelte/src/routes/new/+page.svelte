<script lang="ts">
  import { goto } from "$app/navigation";
  import CaseForm from "$lib/components/CaseForm.svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { Case } from "$lib/api/types";

  const repo = CaseRepository.withFetch();
  const initial: Case = { title: "" };

  async function handleSubmit(record: Case) {
    const created = await repo.create(record);
    await goto(`/${created.pid}`);
  }
</script>

<svelte:head><title>New case — Main X</title></svelte:head>

<h1>New case</h1>
<CaseForm {initial} submitLabel="Create" onsubmit={handleSubmit} />
