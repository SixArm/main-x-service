<!--
  CourseIdentifierInput — editor for the repeating list of course
  identifiers (scheme + value + optional URL, with a "Custom…" scheme
  that reveals a free-text label field). The `identifiers` array is
  $bindable so edits flow straight back to the parent form's model.

  $props:
    - identifiers: CourseIdentifier[] ($bindable) — the list being edited.

  Note: array entries are replaced (spread into new objects), never
  mutated in place, so Svelte reactivity reliably re-renders rows.
-->
<script lang="ts">
    import type { IdentifierType, CourseIdentifier } from "$lib/api/types.js";
    import {
        IDENTIFIER_TYPE_OPTIONS,
        blankCourseIdentifier,
    } from "$lib/api/types.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import { t } from "$lib/i18n.svelte.js";

    let {
        identifiers = $bindable(),
    }: {
        identifiers: CourseIdentifier[];
    } = $props();

    // True when this scheme is the { Custom: string } object variant.
    function isCustom(t: IdentifierType): t is { Custom: string } {
        return typeof t !== "string";
    }

    // Change the scheme of row `idx`. The sentinel "Custom" dropdown
    // value becomes an empty { Custom } object (label entered separately).
    function setType(idx: number, value: string) {
        const current = identifiers[idx];
        if (!current) return;
        if (value === "Custom") {
            identifiers[idx] = { ...current, property_id: { Custom: "" } };
        } else {
            identifiers[idx] = {
                ...current,
                property_id: value as IdentifierType,
            };
        }
    }

    // Update the free-text label of a custom-scheme row.
    function setCustomLabel(idx: number, label: string) {
        const current = identifiers[idx];
        if (!current) return;
        identifiers[idx] = { ...current, property_id: { Custom: label } };
    }

    // Append a fresh blank identifier row.
    function add() {
        identifiers = [...identifiers, blankCourseIdentifier()];
    }
    // Remove row `idx` (rebuilds the array without that index).
    function remove(idx: number) {
        identifiers = identifiers.filter((_, i) => i !== idx);
    }
</script>

<section class="stack">
    {#each identifiers as identifier, idx (idx)}
        <div class="identifier surface">
            <FieldRow>
                <LabeledField
                    label={t("identifier.type")}
                    for={`id-type-${idx}`}
                >
                    <select
                        id={`id-type-${idx}`}
                        value={isCustom(identifier.property_id)
                            ? "Custom"
                            : identifier.property_id}
                        onchange={(e) =>
                            setType(idx, (e.target as HTMLSelectElement).value)}
                    >
                        {#each IDENTIFIER_TYPE_OPTIONS as opt}
                            <option value={opt}>{opt}</option>
                        {/each}
                        <option value="Custom"
                            >{t("identifier.customOption")}</option
                        >
                    </select>
                </LabeledField>
                {#if isCustom(identifier.property_id)}
                    <LabeledField
                        label={t("identifier.customLabel")}
                        for={`id-custom-${idx}`}
                    >
                        <input
                            id={`id-custom-${idx}`}
                            value={identifier.property_id.Custom}
                            oninput={(e) =>
                                setCustomLabel(
                                    idx,
                                    (e.target as HTMLInputElement).value,
                                )}
                        />
                    </LabeledField>
                {/if}
                <LabeledField
                    label={t("identifier.value")}
                    for={`id-val-${idx}`}
                    required
                >
                    <input
                        id={`id-val-${idx}`}
                        bind:value={identifier.value}
                        required
                    />
                </LabeledField>
                <LabeledField label={t("identifier.url")} for={`id-url-${idx}`}>
                    <input id={`id-url-${idx}`} bind:value={identifier.url} />
                </LabeledField>
            </FieldRow>
            <button
                type="button"
                class="button danger small"
                onclick={() => remove(idx)}>{t("identifier.remove")}</button
            >
        </div>
    {/each}
    <button type="button" class="button" onclick={add}
        >{t("identifier.add")}</button
    >
</section>

<style>
    .identifier {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
</style>
