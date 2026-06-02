<script lang="ts">
    import { goto } from "$app/navigation";
    import PersonForm from "$lib/components/PersonForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    const blank: Person = {
        name: { family: "", given: [] },
        gender: "unknown",
        active: true,
    };

    async function handleSubmit(value: Person) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/persons/${created.id}`);
        } catch (err) {
            if (err instanceof ApiError && err.isConflict && Array.isArray(err.details)) {
                duplicates = err.details as MatchResult[];
                throw new Error(`Duplicates detected (${duplicates.length}) — review below before resubmitting.`);
            }
            throw err;
        }
    }
</script>

<svelte:head><title>New person · Person Service</title></svelte:head>

<header><h1>New person</h1></header>

<section class="surface stack">
    <PersonForm initial={blank} submitLabel="Create" onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title="Possible duplicates" />
{/if}
