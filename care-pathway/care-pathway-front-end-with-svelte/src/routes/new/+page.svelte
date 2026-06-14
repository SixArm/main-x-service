<script lang="ts">
    // Create route ("/new") — renders the shared form with an empty seed
    // and, on submit, POSTs the new pathway then navigates to its detail
    // page. No reactive state of its own; the form owns the field state.
    import { goto } from "$app/navigation";
    import CarePathwayForm from "$lib/components/CarePathwayForm.svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { CarePathway } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();
    // Empty seed: only `name` is required by the type.
    const initial: CarePathway = { name: "" };

    // Save handler passed to the form: create, then route to the new record.
    // A thrown error propagates back into the form's inline error banner.
    async function handleSubmit(pathway: CarePathway) {
        const created = await repo.create(pathway);
        await goto(`/${created.pid}`);
    }
</script>

<svelte:head><title>New care pathway — Main X</title></svelte:head>

<h1>New care pathway</h1>
<CarePathwayForm {initial} submitLabel="Create" onsubmit={handleSubmit} />
