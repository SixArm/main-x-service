<script lang="ts">
    import { goto } from "$app/navigation";
    import WorkerForm from "$lib/components/WorkerForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Worker } from "$lib/api/types.js";

    const repo = WorkerRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    const blank: Worker = {
        name: { family: "", given: [] },
        gender: "unknown",
        active: true,
    };

    async function handleSubmit(value: Worker) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/workers/${created.id}`);
        } catch (err) {
            if (err instanceof ApiError && err.isConflict && Array.isArray(err.details)) {
                duplicates = err.details as MatchResult[];
                throw new Error(`Duplicates detected (${duplicates.length}) — review below before resubmitting.`);
            }
            throw err;
        }
    }
</script>

<svelte:head><title>New worker · Worker Service</title></svelte:head>

<header><h1>New worker</h1></header>

<section class="surface stack">
    <WorkerForm initial={blank} submitLabel="Create" onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title="Possible duplicates" />
{/if}
