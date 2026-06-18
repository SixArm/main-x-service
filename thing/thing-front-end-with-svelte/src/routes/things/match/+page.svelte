<!--
  +page.svelte (/things/match) — ad-hoc match / duplicate check.

  Purpose: lets the user describe a candidate Thing (name, description, URL,
  identifiers, same-as URLs, threshold) and run a match against the index
  without persisting anything; results render in MatchResultsList.

  $state:
    - name / description / url / threshold / sameAsRaw: bound form fields.
    - identifiers: edited via ThingIdentifierInput.
    - results / error / loading: outcome and request status.

  Reactive notes: runMatch assembles a MatchRequest, sending optional fields
  as undefined when blank so they're omitted from the query.
-->
<script lang="ts">
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import ThingIdentifierInput from "$lib/components/ThingIdentifierInput.svelte";
    import { ThingRepository } from "$lib/api/things.js";
    import type { MatchRequest, MatchResult, ThingIdentifier } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";

    const repo = ThingRepository.withFetch();

    let name = $state("");
    let description = $state("");
    let url = $state("");
    let threshold = $state(0.7);
    let identifiers = $state<ThingIdentifier[]>([]);
    let sameAsRaw = $state("");

    let results = $state<MatchResult[]>([]);
    let error = $state<string | null>(null);
    let loading = $state(false);

    async function runMatch(e: SubmitEvent) {
        e.preventDefault();
        loading = true;
        error = null;
        try {
            // Build the request; empty optionals become undefined (omitted),
            // and identifiers are sent only when at least one row exists.
            const req: MatchRequest = {
                name,
                description: description || undefined,
                url: url || undefined,
                identifiers: identifiers.length > 0 ? identifiers : undefined,
                same_as: sameAsRaw.split("\n").map((s) => s.trim()).filter(Boolean),
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

<svelte:head><title>Match check · Thing Service</title></svelte:head>

<header><h1>{t("match.title")}</h1></header>

<section class="surface stack">
    <form onsubmit={runMatch} class="stack">
        <FieldRow>
            <LabeledField label={t("match.name")} for="m-name" required><input id="m-name" bind:value={name} required /></LabeledField>
            <LabeledField label={t("match.threshold")} for="m-threshold" hint={t("match.thresholdHint")}>
                <input id="m-threshold" type="number" step="0.05" min="0" max="1" bind:value={threshold} />
            </LabeledField>
        </FieldRow>
        <LabeledField label={t("match.description")} for="m-desc">
            <textarea id="m-desc" rows={2} bind:value={description}></textarea>
        </LabeledField>
        <LabeledField label={t("match.url")} for="m-url">
            <input id="m-url" type="url" bind:value={url} />
        </LabeledField>
        <LabeledField label={t("match.sameAs")} for="m-sameas" hint={t("match.sameAsHint")}>
            <textarea id="m-sameas" rows={2} bind:value={sameAsRaw}></textarea>
        </LabeledField>
        <section class="stack">
            <h2 class="small">{t("match.identifiers")}</h2>
            <ThingIdentifierInput bind:identifiers />
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
