<!--
  New place (route "/places/new") — create form with server-side duplicate
  handling. On success it navigates to the new record's detail page.

  Local $state:
    - duplicates — candidate matches returned in a 409 Conflict, rendered
      below the form so the operator can review before resubmitting.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import PlaceForm from "$lib/components/PlaceForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    // Seed value for an empty create form (only `name` is required).
    const blank: Place = { name: "" };

    // Passed to PlaceForm.onsubmit. Rethrows so the form shows submitError.
    async function handleSubmit(value: Place) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/places/${created.id}`);
        } catch (err) {
            // 409 with a details array = duplicate candidates: surface them
            // for review and convert to a form-level message (not a hard error).
            if (err instanceof ApiError && err.isConflict && Array.isArray(err.details)) {
                duplicates = err.details as MatchResult[];
                throw new Error(`Duplicates detected (${duplicates.length}) — review below before resubmitting.`);
            }
            throw err; // any other error propagates to the form unchanged
        }
    }
</script>

<svelte:head><title>New place · Place Service</title></svelte:head>

<header><h1>New place</h1></header>

<section class="surface stack">
    <PlaceForm initial={blank} submitLabel="Create" onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title="Possible duplicates" />
{/if}
