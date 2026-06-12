<script lang="ts">
    import { goto } from "$app/navigation";
    import CarePathwayForm from "$lib/components/CarePathwayForm.svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { CarePathway } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();
    const initial: CarePathway = { name: "" };

    async function handleSubmit(pathway: CarePathway) {
        const created = await repo.create(pathway);
        await goto(`/${created.pid}`);
    }
</script>

<svelte:head><title>New care pathway — Main X</title></svelte:head>

<h1>New care pathway</h1>
<CarePathwayForm {initial} submitLabel="Create" onsubmit={handleSubmit} />
