<!--
  Match-check page (route "/events/match") — an ad-hoc form to score
  hypothetical event attributes against existing records, showing ranked
  candidates. Does not create anything.

  State ($state): the form fields (name/start/end/organizer/threshold) and
  the results/error/loading flags.
-->
<script lang="ts">
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { EventRepository } from "$lib/api/events.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { MatchRequest, MatchResult } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();

    let name = $state("");
    let startDate = $state("");
    let endDate = $state("");
    let organizerName = $state("");
    let threshold = $state(0.7);

    let results = $state<MatchResult[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(false);

    // Build the MatchRequest from the form and fetch candidates. Datetime
    // inputs (local) are converted to ISO/UTC; blank fields are omitted.
    async function runMatch(e: SubmitEvent) {
        e.preventDefault();
        loading = true;
        error = null;
        try {
            const req: MatchRequest = {
                name,
                start_date: startDate
                    ? new Date(startDate).toISOString()
                    : undefined,
                end_date: endDate ? new Date(endDate).toISOString() : undefined,
                organizers: organizerName
                    ? [{ kind: "organization", name: organizerName }]
                    : undefined,
                threshold,
            };
            results = await repo.match(req);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            results = [];
        } finally {
            loading = false;
        }
    }
</script>

<svelte:head><title>Match check · Event Service</title></svelte:head>

<header><h1>{t("match.title")}</h1></header>

<section class="surface stack">
    <form onsubmit={runMatch} class="stack">
        <FieldRow>
            <LabeledField label={t("match.name")} for="m-name" required
                ><input id="m-name" bind:value={name} required /></LabeledField
            >
            <LabeledField
                label={t("match.threshold")}
                for="m-threshold"
                hint={t("match.thresholdHint")}
            >
                <input
                    id="m-threshold"
                    type="number"
                    step="0.05"
                    min="0"
                    max="1"
                    bind:value={threshold}
                />
            </LabeledField>
        </FieldRow>
        <FieldRow>
            <LabeledField label={t("match.start")} for="m-start">
                <input
                    id="m-start"
                    type="datetime-local"
                    bind:value={startDate}
                />
            </LabeledField>
            <LabeledField label={t("match.end")} for="m-end">
                <input id="m-end" type="datetime-local" bind:value={endDate} />
            </LabeledField>
            <LabeledField label={t("match.organizerName")} for="m-org">
                <input id="m-org" bind:value={organizerName} />
            </LabeledField>
        </FieldRow>
        {#if error}<div class="banner error">{error}</div>{/if}
        <button type="submit" class="button primary" disabled={loading}>
            {loading ? t("match.matching") : t("match.find")}
        </button>
    </form>
</section>

{#if results.length > 0}
    <MatchResultsList {results} />
{/if}
