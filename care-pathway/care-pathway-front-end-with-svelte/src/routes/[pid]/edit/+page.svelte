<script lang="ts">
    // Edit route ("/[pid]/edit") — fetch the existing record, seed the
    // shared form with it, and on submit PUT the update then navigate back
    // to the detail page.
    //
    // State ($state): `pathway` (the fetched seed, null until loaded),
    // `loading`, `error`. The form is rendered only once `pathway` resolves
    // so its one-time seed reads the real values.
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import CarePathwayForm from "$lib/components/CarePathwayForm.svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { CarePathway } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();
    // The pid path param (`?? ""` only to satisfy the optional type).
    const pid = page.params.pid ?? "";

    let pathway = $state<CarePathway | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    // Load the record to edit on mount.
    onMount(async () => {
        try {
            pathway = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Not found";
        } finally {
            loading = false;
        }
    });

    // Save handler: PUT the updated record, then return to the detail page.
    async function handleSubmit(updated: CarePathway) {
        await repo.update(pid, updated);
        await goto(`/${pid}`);
    }
</script>

<svelte:head><title>Edit {pathway?.name ?? "care pathway"} — Main X</title></svelte:head>

<h1>Edit care pathway</h1>

{#if loading}
    <p>Loading…</p>
{:else if error}
    <p class="banner" role="alert">{error}</p>
{:else if pathway}
    <CarePathwayForm initial={pathway} submitLabel="Save changes" onsubmit={handleSubmit} />
{/if}
