<!--
  New worker (route "/workers/new") — renders WorkerForm and creates the
  record. Handles the service's real-time duplicate detection: a 409 with a
  candidate list is shown below the form instead of navigating away.

  $state:
    - duplicates — candidate MatchResults from a 409 conflict, rendered as
      a "Possible duplicates" list.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import WorkerForm from "#lib/components/WorkerForm.svelte";
    import MatchResultsList from "#lib/components/MatchResultsList.svelte";
    import { WorkerRepository } from "#lib/api/workers.js";
    import { ApiError } from "#lib/api/client.js";
    import type { MatchResult, Worker } from "#lib/api/types.js";
    import { t, tf } from "#lib/i18n.svelte.js";

    const repo = WorkerRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    // Empty starting record satisfying Worker's required fields.
    const blank: Worker = {
        name: { family: "", given: [] },
        gender: "unknown",
        active: true,
    };

    // WorkerForm submit handler: create, then navigate to the new record.
    /** The duplicate candidates a `409` carries, or none. The service's
     *  problem holds `{ has_duplicates, potential_matches }` in `errors`;
     *  an older one sent the `MatchResult[]` alone. Accept both. */
    function duplicateCandidates(err: unknown): MatchResult[] {
        if (!(err instanceof ApiError) || !err.isConflict) return [];
        const details = err.details as
            | MatchResult[]
            | { potential_matches?: MatchResult[] }
            | null
            | undefined;
        return Array.isArray(details)
            ? details
            : (details?.potential_matches ?? []);
    }

    async function handleSubmit(value: Worker) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/workers/${created.id}`);
        } catch (err) {
            // A 409 carries duplicate candidates. Show them and rethrow so
            // WorkerForm displays the banner and stays put, letting the
            // operator review before resubmitting.
            const candidates = duplicateCandidates(err);
            if (candidates.length > 0) {
                duplicates = candidates;
                throw new Error(
                    tf("new.duplicatesDetected", { count: duplicates.length }),
                );
            }
            // Any other error bubbles to the form banner.
            throw err;
        }
    }
</script>

<svelte:head><title>{t("new.titleTab")}</title></svelte:head>

<header><h1>{t("new.heading")}</h1></header>

<section class="surface stack">
    <WorkerForm
        initial={blank}
        submitLabel={t("new.submit")}
        onsubmit={handleSubmit}
    />
</section>

{#if duplicates.length > 0}
    <MatchResultsList
        results={duplicates}
        title={t("new.possibleDuplicates")}
    />
{/if}
