<script lang="ts">
    import { untrack } from "svelte";
    import { ALL_CARE_SETTINGS, ALL_CODE_SYSTEMS, ALL_SCHEMES } from "$lib/api/types";
    import type {
        CarePathway,
        CareSetting,
        CodeSystem,
        ConditionCode,
        IdentifierScheme,
        PathwayIdentifier,
    } from "$lib/api/types";

    let {
        initial,
        submitLabel = "Save",
        onsubmit,
    }: {
        initial: CarePathway;
        submitLabel?: string;
        onsubmit: (pathway: CarePathway) => Promise<void>;
    } = $props();

    // Seed the form once from `initial` (read without tracking).
    const seed = untrack(() => initial);

    let name = $state(seed.name ?? "");
    let pathwayCode = $state(seed.pathway_code ?? "");
    let providerId = $state(seed.provider_id ?? "");
    let providerName = $state(seed.provider_name ?? "");
    let careSetting = $state<CareSetting | "">(
        typeof seed.care_setting === "string" ? seed.care_setting : "",
    );
    let alternateNames = $state((seed.alternate_names ?? []).join(", "));
    let interventions = $state((seed.interventions ?? []).join(", "));
    let keywords = $state((seed.keywords ?? []).join(", "));
    let sameAs = $state((seed.same_as ?? []).join(", "));
    let conditionCodes = $state<ConditionCode[]>([...(seed.condition_codes ?? [])]);
    let identifiers = $state<PathwayIdentifier[]>(
        (seed.identifiers ?? []).filter((i) => typeof i.scheme === "string"),
    );

    let submitting = $state(false);
    let error = $state<string | null>(null);

    function splitList(s: string): string[] {
        return s
            .split(",")
            .map((x) => x.trim())
            .filter((x) => x.length > 0);
    }
    function blankToNull(s: string): string | null {
        const t = s.trim();
        return t.length > 0 ? t : null;
    }

    function addCondition() {
        conditionCodes = [...conditionCodes, { system: "Icd10", code: "" }];
    }
    function removeCondition(i: number) {
        conditionCodes = conditionCodes.filter((_, idx) => idx !== i);
    }
    function addIdentifier() {
        identifiers = [...identifiers, { scheme: "GuidelineId", value: "" }];
    }
    function removeIdentifier(i: number) {
        identifiers = identifiers.filter((_, idx) => idx !== i);
    }

    function build(): CarePathway {
        const pathway: CarePathway = { name: name.trim() };
        pathway.pathway_code = blankToNull(pathwayCode);
        pathway.provider_id = blankToNull(providerId);
        pathway.provider_name = blankToNull(providerName);
        pathway.care_setting = careSetting === "" ? null : (careSetting as CareSetting);
        pathway.alternate_names = splitList(alternateNames);
        pathway.interventions = splitList(interventions);
        pathway.keywords = splitList(keywords);
        pathway.same_as = splitList(sameAs);
        pathway.condition_codes = conditionCodes
            .filter((c) => c.code.trim().length > 0)
            .map((c) => ({ system: c.system, code: c.code.trim() }));
        pathway.identifiers = identifiers
            .filter((i) => i.value.trim().length > 0)
            .map((i) => ({ scheme: i.scheme, value: i.value.trim() }));
        return pathway;
    }

    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        error = null;
        if (name.trim().length === 0) {
            error = "Name is required.";
            return;
        }
        submitting = true;
        try {
            await onsubmit(build());
        } catch (err) {
            error = err instanceof Error ? err.message : "Save failed";
        } finally {
            submitting = false;
        }
    }
</script>

<form class="stack" onsubmit={handleSubmit}>
    <label>Name<input type="text" bind:value={name} required /></label>
    <div class="row">
        <label>Care setting
            <select bind:value={careSetting}>
                <option value="">—</option>
                {#each ALL_CARE_SETTINGS as setting (String(setting))}
                    <option value={setting as CareSetting}>{setting}</option>
                {/each}
            </select>
        </label>
        <label>Pathway code<input type="text" bind:value={pathwayCode} placeholder="STROKE-01" /></label>
    </div>
    <div class="row">
        <label>Provider id<input type="text" bind:value={providerId} /></label>
        <label>Provider name<input type="text" bind:value={providerName} /></label>
    </div>
    <label>Alternate names <small>(comma-separated)</small><input type="text" bind:value={alternateNames} /></label>
    <label>Interventions <small>(comma-separated)</small><input type="text" bind:value={interventions} /></label>
    <label>Keywords <small>(comma-separated)</small><input type="text" bind:value={keywords} /></label>
    <label>Same-as URLs <small>(comma-separated)</small><input type="text" bind:value={sameAs} /></label>

    <fieldset class="stack">
        <legend>Target condition codes</legend>
        {#each conditionCodes as condition, i (i)}
            <div class="row">
                <select bind:value={condition.system}>
                    {#each ALL_CODE_SYSTEMS as system (String(system))}
                        <option value={system as CodeSystem}>{system}</option>
                    {/each}
                </select>
                <input type="text" bind:value={condition.code} placeholder="I63" />
                <button type="button" onclick={() => removeCondition(i)}>Remove</button>
            </div>
        {/each}
        <button type="button" onclick={addCondition}>+ Add condition code</button>
    </fieldset>

    <fieldset class="stack">
        <legend>Identifiers</legend>
        {#each identifiers as identifier, i (i)}
            <div class="row">
                <select bind:value={identifier.scheme}>
                    {#each ALL_SCHEMES as scheme (String(scheme))}
                        <option value={scheme as IdentifierScheme}>{scheme}</option>
                    {/each}
                </select>
                <input type="text" bind:value={identifier.value} placeholder="value" />
                <button type="button" onclick={() => removeIdentifier(i)}>Remove</button>
            </div>
        {/each}
        <button type="button" onclick={addIdentifier}>+ Add identifier</button>
    </fieldset>

    <button class="button" type="submit" disabled={submitting}>
        {submitting ? "Saving…" : submitLabel}
    </button>
    {#if error}<p class="banner" role="alert">{error}</p>{/if}
</form>
