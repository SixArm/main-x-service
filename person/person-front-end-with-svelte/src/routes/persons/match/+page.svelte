<!--
  Match check (/persons/match) — ad-hoc probabilistic match query.

  An operator enters demographic fields and a threshold; submitting runs a
  match against the index and renders the scored candidates. Does not create
  anything — it's a read-only "who might this be?" tool.

  State:
    - family/given/birthDate/gender/taxId/threshold — the query inputs.
    - results — the scored candidates from the last query.
    - error / loading — request lifecycle.
-->
<script lang="ts">
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { PersonRepository } from "$lib/api/persons.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { Gender, MatchRequest, MatchResult } from "$lib/api/types.js";

    const repo = PersonRepository.withFetch();

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

    // Build the MatchRequest from the form and run it. Empty optional fields
    // are sent as null; given names are split on whitespace into an array.
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

<svelte:head><title>{t("match.head.title")}</title></svelte:head>

<header><h1>{t("match.title")}</h1></header>

<section class="surface stack">
    <form onsubmit={runMatch} class="stack">
        <FieldRow>
            <LabeledField label={t("match.family")} for="m-family" required><input id="m-family" bind:value={family} required /></LabeledField>
            <LabeledField label={t("match.given")} for="m-given" required hint={t("match.givenHint")}><input id="m-given" bind:value={given} required /></LabeledField>
        </FieldRow>
        <FieldRow>
            <LabeledField label={t("match.birthDate")} for="m-dob"><input id="m-dob" type="date" bind:value={birthDate} /></LabeledField>
            <LabeledField label={t("match.gender")} for="m-gender">
                <select id="m-gender" bind:value={gender}>
                    {#each genders as g}<option value={g}>{g}</option>{/each}
                </select>
            </LabeledField>
            <LabeledField label={t("match.taxId")} for="m-tax"><input id="m-tax" bind:value={taxId} /></LabeledField>
            <LabeledField label={t("match.threshold")} for="m-threshold" hint={t("match.thresholdHint")}>
                <input id="m-threshold" type="number" step="0.05" min="0" max="1" bind:value={threshold} />
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
