<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import CarePathwayForm from "$lib/components/CarePathwayForm.svelte";
    import { CarePathwayRepository } from "$lib/api/care-pathways";
    import type { CarePathway } from "$lib/api/types";

    const repo = CarePathwayRepository.withFetch();
    const pid = page.params.pid ?? "";

    let pathway = $state<CarePathway | null>(null);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            pathway = await repo.get(pid);
        } catch (err) {
            error = err instanceof Error ? err.message : "Not found";
        } finally {
            loading = false;
        }
    });

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
