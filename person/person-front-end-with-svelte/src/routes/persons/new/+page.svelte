<!--
  New person (/persons/new) — create form with inline duplicate handling.

  Renders PersonForm over a blank Person. On submit it creates the record
  and navigates to its detail page; if the service responds 409 Conflict
  (duplicate detected) it extracts the candidate matches, shows them below,
  and rethrows so the form surfaces a "review before resubmitting" message.

  State:
    - duplicates — candidate matches from the most recent 409, if any.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import PersonForm from "$lib/components/PersonForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { ApiError } from "$lib/api/client.js";
    import type { MatchResult, Person } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    // Empty starting record for the create form.
    const blank: Person = {
        name: { family: "", given: [] },
        gender: "unknown",
        active: true,
    };

    // Submit handler passed to PersonForm; runs inside the form's submit
    // pipeline, so a thrown error becomes the form's submitError.
    async function handleSubmit(value: Person) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/persons/${created.id}`);
        } catch (err) {
            // 409 Conflict means the service detected potential duplicates.
            if (err instanceof ApiError && err.isConflict) {
                // Service emits one of:
                //   • `MatchResult[]` directly                 (legacy)
                //   • `{ has_duplicates, potential_matches }`  (current)
                // Normalise both into the MatchResult[] the UI renders.
                const details = err.details as
                    | MatchResult[]
                    | { has_duplicates?: boolean; potential_matches?: MatchResult[] }
                    | null;
                const extracted: MatchResult[] = Array.isArray(details)
                    ? details
                    : (details?.potential_matches ?? []);
                duplicates = extracted;
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
