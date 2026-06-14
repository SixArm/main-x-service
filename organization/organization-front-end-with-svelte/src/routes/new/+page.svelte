<!--
  Create route (`/new`): renders an empty OrganizationForm and POSTs the
  result. On success, navigates to the new record's detail page. Errors
  thrown by `create` propagate to the form's inline error banner.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import OrganizationForm from "$lib/components/OrganizationForm.svelte";
    import { OrganizationRepository } from "$lib/api/organizations";
    import type { Organization } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();
    // Empty seed: a blank form (only the required `name`).
    const initial: Organization = { name: "" };

    /** Persist the new organization, then route to its detail page. */
    async function handleSubmit(org: Organization) {
        const created = await repo.create(org);
        await goto(`/${created.pid}`);
    }
</script>

<svelte:head><title>New organization — Main X</title></svelte:head>

<h1>New organization</h1>
<OrganizationForm {initial} submitLabel="Create" onsubmit={handleSubmit} />
