<!--
  Create-case route (`/new`).

  Purpose: render an empty `CaseForm` and, on submit, POST it then navigate
  to the new record's detail page. Errors surface inside the form's banner
  (the thrown rejection propagates back into `CaseForm.handleSubmit`).
-->
<script lang="ts">
  import { goto } from "$app/navigation";
  import CaseForm from "$lib/components/CaseForm.svelte";
  import { CaseRepository } from "$lib/api/cases";
  import type { Case } from "$lib/api/types";
  import { t } from "$lib/i18n.svelte";

  const repo = CaseRepository.withFetch();
  // Empty seed — only the required title, blank.
  const initial: Case = { title: "" };

  // Create, then route to the newly created case's detail page.
  async function handleSubmit(record: Case) {
    const created = await repo.create(record);
    await goto(`/${created.pid}`);
  }
</script>

<svelte:head><title>New case — Main X</title></svelte:head>

<h1>{t("new.title")}</h1>
<CaseForm {initial} submitLabel={t("new.create")} onsubmit={handleSubmit} />
