<script lang="ts">
    import { goto } from "$app/navigation";
    import CourseForm from "$lib/components/CourseForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Course } from "$lib/api/types.js";

    const repo = CourseRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    const blank: Course = { name: "" };

    async function handleSubmit(value: Course) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/courses/${created.id}`);
        } catch (err) {
            if (err instanceof ApiError && err.isConflict) {
                // Service wraps the duplicates in { has_duplicates, potential_matches };
                // older shape was a bare MatchResult[]. Normalise both.
                const details = err.details as
                    | MatchResult[]
                    | { has_duplicates?: boolean; potential_matches?: MatchResult[] }
                    | null;
                duplicates = Array.isArray(details)
                    ? details
                    : (details?.potential_matches ?? []);
                throw new Error(`Duplicates detected (${duplicates.length}) — review below before resubmitting.`);
            }
            throw err;
        }
    }
</script>

<svelte:head><title>New course · Course Service</title></svelte:head>

<header><h1>New course</h1></header>

<section class="surface stack">
    <CourseForm initial={blank} submitLabel="Create" onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title="Possible duplicates" />
{/if}
