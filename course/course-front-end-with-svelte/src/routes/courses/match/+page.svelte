<!--
  Match check (route "/courses/match") — ad-hoc probe form that scores a
  hypothetical course against the index without persisting it. The
  service returns every blocked candidate sorted by score; the page
  filters by a client-side display threshold so the slider is
  responsive without re-querying.

  Reactive state:
    - name/courseCode/providerId/educationalLevel/keywordsRaw/teachesRaw/
      sameAsRaw/identifiers — the probe inputs.
    - threshold — client-side display cutoff.
    - rawResults — full result set from the service.
    - results ($derived) — rawResults filtered to score >= threshold.
    - loading / error — request status.
-->
<script lang="ts">
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import CourseIdentifierInput from "$lib/components/CourseIdentifierInput.svelte";
    import { CourseRepository } from "$lib/api/courses.js";
    import { EDUCATIONAL_LEVEL_OPTIONS } from "$lib/api/types.js";
    import type {
        CourseIdentifier,
        EducationalLevel,
        MatchRequest,
        MatchResult,
    } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    const repo = CourseRepository.withFetch();

    let name = $state("");
    let courseCode = $state("");
    let providerId = $state("");
    let threshold = $state(0.7);
    let educationalLevel = $state<string>("");
    let identifiers = $state<CourseIdentifier[]>([]);
    let keywordsRaw = $state("");
    let teachesRaw = $state("");
    let sameAsRaw = $state("");

    let rawResults = $state<MatchResult[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(false);
    // Service returns every blocked candidate sorted by score; we
    // filter client-side so the threshold slider is responsive without
    // an extra round-trip.
    let results = $derived(rawResults.filter((r) => r.score >= threshold));

    // Build the probe request and run the match; blank optional fields
    // are sent as undefined (omitted) rather than empty strings/arrays.
    async function runMatch(e: SubmitEvent) {
        e.preventDefault();
        loading = true;
        error = null;
        try {
            const req: MatchRequest = {
                name,
                course_code: courseCode || undefined,
                provider_id: providerId || undefined,
                educational_level: educationalLevel
                    ? (educationalLevel as EducationalLevel)
                    : undefined,
                keywords: keywordsRaw
                    .split(/[,\n]/)
                    .map((s) => s.trim())
                    .filter(Boolean),
                teaches: teachesRaw
                    .split("\n")
                    .map((s) => s.trim())
                    .filter(Boolean),
                identifiers: identifiers.length > 0 ? identifiers : undefined,
                same_as: sameAsRaw
                    .split("\n")
                    .map((s) => s.trim())
                    .filter(Boolean),
            };
            rawResults = await repo.match(req);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            rawResults = [];
        } finally {
            loading = false;
        }
    }
</script>

<svelte:head><title>Match check · Course Service</title></svelte:head>

<header><h1>{t("match.title")}</h1></header>

<section class="surface stack">
    <form onsubmit={runMatch} class="stack">
        <FieldRow>
            <LabeledField label={t("match.name")} for="m-name" required>
                <input id="m-name" bind:value={name} required />
            </LabeledField>
            <LabeledField label={t("match.courseCode")} for="m-code">
                <input id="m-code" bind:value={courseCode} />
            </LabeledField>
            <LabeledField label={t("match.providerId")} for="m-prov">
                <input id="m-prov" bind:value={providerId} />
            </LabeledField>
            <LabeledField
                label={t("match.displayThreshold")}
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
            <LabeledField label={t("match.educationalLevel")} for="m-level">
                <select id="m-level" bind:value={educationalLevel}>
                    <option value="">—</option>
                    {#each EDUCATIONAL_LEVEL_OPTIONS as l}
                        <option value={l}>{l}</option>
                    {/each}
                </select>
            </LabeledField>
        </FieldRow>
        <LabeledField
            label={t("match.keywords")}
            for="m-kw"
            hint={t("match.keywordsHint")}
        >
            <input id="m-kw" bind:value={keywordsRaw} />
        </LabeledField>
        <LabeledField
            label={t("match.teaches")}
            for="m-teaches"
            hint={t("match.teachesHint")}
        >
            <textarea id="m-teaches" rows={2} bind:value={teachesRaw}
            ></textarea>
        </LabeledField>
        <LabeledField
            label={t("match.sameAs")}
            for="m-sameas"
            hint={t("match.sameAsHint")}
        >
            <textarea id="m-sameas" rows={2} bind:value={sameAsRaw}></textarea>
        </LabeledField>
        <section class="stack">
            <h2 class="small">{t("match.identifiers")}</h2>
            <CourseIdentifierInput bind:identifiers />
        </section>
        {#if error}<div class="banner error">{error}</div>{/if}
        <button type="submit" class="button primary" disabled={loading}>
            {loading ? t("match.matching") : t("match.findMatches")}
        </button>
    </form>
</section>

{#if results.length > 0}
    <MatchResultsList {results} />
{/if}
