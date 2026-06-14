<!--
  ThingIdentifierInput — editor for a Thing's list of external identifiers
  (schema.org/PropertyValue: DOI / ISBN / GTIN / SKU / URI / UUID / Custom).

  Purpose: lets the user add, remove, and edit identifier rows, including
  choosing a well-known scheme or a free-text "Custom" label. Renders one
  card per identifier with type / value / URL controls.

  $props:
    - identifiers ($bindable ThingIdentifier[]): the array to edit, mutated
      in place and reassigned so the parent's binding stays in sync.

  Reactive notes: rows are keyed by index in the {#each}; the type <select>
  distinguishes the tagged `{ Custom }` variant from the bare string schemes.
-->
<script lang="ts">
    import type { IdentifierType, ThingIdentifier } from "$lib/api/types.js";
    import { IDENTIFIER_TYPE_OPTIONS, blankThingIdentifier } from "$lib/api/types.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";

    let {
        identifiers = $bindable(),
    }: {
        identifiers: ThingIdentifier[];
    } = $props();

    // Type guard: the Custom variant is the only non-string IdentifierType,
    // so a non-string value implies `{ Custom: string }`.
    function isCustom(t: IdentifierType): t is { Custom: string } {
        return typeof t !== "string";
    }

    // Change a row's scheme. "Custom" maps to the tagged object with an
    // empty label (the extra label field then appears); anything else is a
    // bare scheme literal. Reassign the element so the binding updates.
    function setType(idx: number, value: string) {
        const current = identifiers[idx];
        if (!current) return;
        if (value === "Custom") {
            identifiers[idx] = { ...current, property_id: { Custom: "" } };
        } else {
            identifiers[idx] = { ...current, property_id: value as IdentifierType };
        }
    }

    // Update the free-text label of a Custom-scheme row.
    function setCustomLabel(idx: number, label: string) {
        const current = identifiers[idx];
        if (!current) return;
        identifiers[idx] = { ...current, property_id: { Custom: label } };
    }

    // Append a fresh blank identifier row.
    function add() {
        identifiers = [...identifiers, blankThingIdentifier()];
    }
    // Drop the row at idx (filter keeps array identity reassignment reactive).
    function remove(idx: number) {
        identifiers = identifiers.filter((_, i) => i !== idx);
    }
</script>

<section class="stack">
    {#each identifiers as identifier, idx (idx)}
        <div class="identifier surface">
            <FieldRow>
                <LabeledField label="Type" for={`id-type-${idx}`}>
                    <!-- Show "Custom" when the scheme is the tagged variant,
                         otherwise the bare scheme literal. -->
                    <select
                        id={`id-type-${idx}`}
                        value={isCustom(identifier.property_id) ? "Custom" : identifier.property_id}
                        onchange={(e) => setType(idx, (e.target as HTMLSelectElement).value)}
                    >
                        {#each IDENTIFIER_TYPE_OPTIONS as t}
                            <option value={t}>{t}</option>
                        {/each}
                        <option value="Custom">Custom…</option>
                    </select>
                </LabeledField>
                <!-- Extra free-text label field, only for Custom schemes. -->
                {#if isCustom(identifier.property_id)}
                    <LabeledField label="Custom label" for={`id-custom-${idx}`}>
                        <input
                            id={`id-custom-${idx}`}
                            value={identifier.property_id.Custom}
                            oninput={(e) => setCustomLabel(idx, (e.target as HTMLInputElement).value)}
                        />
                    </LabeledField>
                {/if}
                <LabeledField label="Value" for={`id-val-${idx}`} required>
                    <input id={`id-val-${idx}`} bind:value={identifier.value} required />
                </LabeledField>
                <LabeledField label="URL" for={`id-url-${idx}`}>
                    <input id={`id-url-${idx}`} bind:value={identifier.url} />
                </LabeledField>
            </FieldRow>
            <button type="button" class="button danger small" onclick={() => remove(idx)}>Remove</button>
        </div>
    {/each}
    <button type="button" class="button" onclick={add}>+ Add identifier</button>
</section>

<style>
    .identifier { display: flex; flex-direction: column; gap: 0.5rem; }
</style>
