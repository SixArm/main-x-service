<script lang="ts">
    // CarePathwayForm — the shared create/edit form for a care pathway.
    //
    // Purpose: render bound inputs for every editable CarePathway field,
    // flatten list fields to comma-separated text for editing, and on
    // submit rebuild a clean CarePathway payload (blanks → null, empty
    // entries dropped) which it hands to the parent via the `onsubmit`
    // callback prop.
    //
    // Props ($props):
    //   - initial: CarePathway — seed values (create passes `{ name: "" }`,
    //     edit passes the fetched record). Read once, untracked.
    //   - submitLabel?: string — label for the submit button ("Save"
    //     default; "Create" / "Save changes" from callers).
    //   - onsubmit: (pathway) => Promise<void> — async save handler; the
    //     form awaits it and surfaces any thrown error inline.
    //
    // State ($state): one signal per field, plus `submitting` (disables the
    // button) and `error` (inline validation / save-failure banner). There
    // is no $derived here; `build()` recomputes the payload on submit.
    //
    // Events: none emitted; the only output is the `onsubmit` callback.
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

    // Scalar fields, defaulted to "" so the inputs are controlled.
    let name = $state(seed.name ?? "");
    let pathwayCode = $state(seed.pathway_code ?? "");
    let providerId = $state(seed.provider_id ?? "");
    let providerName = $state(seed.provider_name ?? "");
    // Only unit-variant care settings are selectable here; a `Custom`
    // object seed (or null) collapses to the "—" (empty) option.
    let careSetting = $state<CareSetting | "">(
        typeof seed.care_setting === "string" ? seed.care_setting : "",
    );
    // List fields are edited as comma-separated text; joined for display,
    // re-split on submit (see `splitList`).
    let alternateNames = $state((seed.alternate_names ?? []).join(", "));
    let interventions = $state((seed.interventions ?? []).join(", "));
    let keywords = $state((seed.keywords ?? []).join(", "));
    let sameAs = $state((seed.same_as ?? []).join(", "));
    // Repeatable rows: copied out of the seed so edits don't mutate `initial`.
    let conditionCodes = $state<ConditionCode[]>([...(seed.condition_codes ?? [])]);
    // Drop any seeded `Custom`-scheme identifier: the scheme <select> only
    // offers unit variants, so a custom one couldn't round-trip.
    let identifiers = $state<PathwayIdentifier[]>(
        (seed.identifiers ?? []).filter((i) => typeof i.scheme === "string"),
    );

    let submitting = $state(false);
    let error = $state<string | null>(null);

    // Split a comma-separated input into trimmed, non-empty entries.
    function splitList(s: string): string[] {
        return s
            .split(",")
            .map((x) => x.trim())
            .filter((x) => x.length > 0);
    }
    // Normalize a scalar text field: trimmed value, or null when blank.
    function blankToNull(s: string): string | null {
        const t = s.trim();
        return t.length > 0 ? t : null;
    }

    // Append/remove repeatable rows by reassigning a new array (so the
    // $state signal sees the change and the `{#each}` re-renders).
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

    // Assemble the current field state into a clean CarePathway payload:
    // trim scalars, blanks→null, split list text, and drop empty rows.
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
        // Keep only rows with a non-empty code / value; trim what remains.
        pathway.condition_codes = conditionCodes
            .filter((c) => c.code.trim().length > 0)
            .map((c) => ({ system: c.system, code: c.code.trim() }));
        pathway.identifiers = identifiers
            .filter((i) => i.value.trim().length > 0)
            .map((i) => ({ scheme: i.scheme, value: i.value.trim() }));
        return pathway;
    }

    // Submit handler: prevent the native POST, validate the required name,
    // then await the parent's save callback with a busy/error lifecycle.
    async function handleSubmit(event: SubmitEvent) {
        event.preventDefault();
        error = null;
        // Client-side required-field guard before hitting the service.
        if (name.trim().length === 0) {
            error = "Name is required.";
            return;
        }
        submitting = true;
        try {
            await onsubmit(build());
        } catch (err) {
            // Surface the service/network error inline; the parent decides
            // navigation on success.
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

    <!--
        Repeatable condition-code rows. Keyed by index `i` (rows have no
        stable id); `bind:value` writes straight back into each row object.
    -->
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

    <!-- Repeatable identifier rows (scheme <select> + value), keyed by index. -->
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
