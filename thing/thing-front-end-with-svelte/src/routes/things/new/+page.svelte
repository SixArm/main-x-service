<script lang="ts">
    import { goto } from "$app/navigation";
    import ThingForm from "$lib/components/ThingForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Thing } from "$lib/api/types.js";

    const repo = ThingRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    const blank: Thing = { name: "" };

    async function handleSubmit(value: Thing) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/things/${created.id}`);
        } catch (err) {
            if (err instanceof ApiError && err.isConflict && Array.isArray(err.details)) {
                duplicates = err.details as MatchResult[];
                throw new Error(`Duplicates detected (${duplicates.length}) — review below before resubmitting.`);
            }
            throw err;
        }
    }
</script>

<svelte:head><title>New thing · Thing Service</title></svelte:head>

<header><h1>New thing</h1></header>

<section class="surface stack">
    <ThingForm initial={blank} submitLabel="Create" onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title="Possible duplicates" />
{/if}
