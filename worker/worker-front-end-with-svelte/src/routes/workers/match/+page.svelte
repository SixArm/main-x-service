<!--
  Match check (route "/workers/match") — ad-hoc duplicate/match lookup. The
  operator enters identity criteria and a threshold; results render via
  MatchResultsList. Non-mutating: this never creates a record.

  $state:
    - family / given / birthDate / gender / taxId — match criteria inputs.
    - threshold — minimum score (0–1) for a candidate to be returned.
    - results — returned candidates.
    - error / loading — request status.
-->
<script lang="ts">
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { WorkerRepository } from "$lib/api/workers.js";
    import type { Gender, MatchRequest, MatchResult } from "$lib/api/types.js";

    const repo = WorkerRepository.withFetch();

    let family = $state("");
    let given = $state("");
    let birthDate = $state("");
    let gender = $state<Gender>("unknown");
    let threshold = $state(0.7);
    let taxId = $state("");

    let results = $state<MatchResult[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(false);

    const genders: Gender[] = ["male", "female", "other", "unknown"];

    // Build the MatchRequest from the form and run the match. Empty strings
    // are normalized to null so they aren't treated as match criteria.
    async function runMatch(e: SubmitEvent) {
        e.preventDefault();
        loading = true;
        error = null;
        try {
            const req: MatchRequest = {
                name: { family, given: given.split(/\s+/).filter(Boolean) },
                birth_date: birthDate || null,
                gender,
                tax_id: taxId || null,
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

<svelte:head><title>Match check · Worker Service</title></svelte:head>

<header><h1>Match check</h1></header>

<section class="surface stack">
    <form onsubmit={runMatch} class="stack">
        <FieldRow>
            <LabeledField label="Family" for="m-family" required><input id="m-family" bind:value={family} required /></LabeledField>
            <LabeledField label="Given" for="m-given" required hint="Space-separated"><input id="m-given" bind:value={given} required /></LabeledField>
        </FieldRow>
        <FieldRow>
            <LabeledField label="Birth date" for="m-dob"><input id="m-dob" type="date" bind:value={birthDate} /></LabeledField>
            <LabeledField label="Gender" for="m-gender">
                <select id="m-gender" bind:value={gender}>
                    {#each genders as g}<option value={g}>{g}</option>{/each}
                </select>
            </LabeledField>
            <LabeledField label="Tax ID" for="m-tax"><input id="m-tax" bind:value={taxId} /></LabeledField>
            <LabeledField label="Threshold" for="m-threshold" hint="0.0 – 1.0">
                <input id="m-threshold" type="number" step="0.05" min="0" max="1" bind:value={threshold} />
            </LabeledField>
        </FieldRow>
        {#if error}<div class="banner error">{error}</div>{/if}
        <button type="submit" class="button primary" disabled={loading}>
            {loading ? "Matching…" : "Find matches"}
        </button>
    </form>
</section>

{#if results.length > 0}
    <MatchResultsList {results} />
{/if}
