<!--
  New course (route "/courses/new") — wraps CourseForm to create a
  course, navigating to its detail page on success. If the service
  rejects with 409 (duplicate), the candidate matches are extracted and
  shown below the form, and an error is rethrown so the form surfaces a
  banner instead of navigating away.

  Reactive state:
    - duplicates — candidate matches from the last 409, rendered below.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import CourseForm from "$lib/components/CourseForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Course } from "$lib/api/types.js";
    import { t, translate, i18n } from "$lib/i18n.svelte.js";

    const repo = CourseRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    // Empty seed for the create form (only `name` is required).
    const blank: Course = { name: "" };

    // Create the course; on 409 surface duplicate candidates and rethrow.
    async function handleSubmit(value: Course) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/courses/${created.id}`);
        } catch (err) {
            if (err instanceof ApiError && err.isConflict) {
                // The Course Service ships `details` as a flat
                // MatchResult[]. A sibling family service was rumoured
                // to wrap it in `{ has_duplicates, potential_matches }`;
                // keep the wrapper-aware branch so this page survives a
                // future service-side shape change without redeploy.
                const details = err.details as
                    | MatchResult[]
                    | { has_duplicates?: boolean; potential_matches?: MatchResult[] }
                    | null;
                duplicates = Array.isArray(details)
                    ? details
                    : (details?.potential_matches ?? []);
                throw new Error(translate("new.duplicatesDetected", i18n.locale).replace("{n}", String(duplicates.length)));
            }
            throw err;
        }
    }
</script>

<svelte:head><title>New course · Course Service</title></svelte:head>

<header><h1>{t("new.title")}</h1></header>

<section class="surface stack">
    <CourseForm initial={blank} submitLabel={t("new.create")} onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title={t("new.possibleDuplicates")} />
{/if}
