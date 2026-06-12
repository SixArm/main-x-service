<script lang="ts">
    import { goto } from "$app/navigation";
    import OrganizationForm from "$lib/components/OrganizationForm.svelte";
    import { OrganizationRepository } from "$lib/api/organizations";
    import type { Organization } from "$lib/api/types";

    const repo = OrganizationRepository.withFetch();
    const initial: Organization = { name: "" };

    async function handleSubmit(org: Organization) {
        const created = await repo.create(org);
        await goto(`/${created.pid}`);
    }
</script>

<svelte:head><title>New organization — Main X</title></svelte:head>

<h1>New organization</h1>
<OrganizationForm {initial} submitLabel="Create" onsubmit={handleSubmit} />
