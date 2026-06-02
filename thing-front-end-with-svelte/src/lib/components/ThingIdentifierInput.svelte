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

    function isCustom(t: IdentifierType): t is { Custom: string } {
        return typeof t !== "string";
    }

    function setType(idx: number, value: string) {
        const current = identifiers[idx];
        if (!current) return;
        if (value === "Custom") {
            identifiers[idx] = { ...current, property_id: { Custom: "" } };
        } else {
            identifiers[idx] = { ...current, property_id: value as IdentifierType };
        }
    }

    function setCustomLabel(idx: number, label: string) {
        const current = identifiers[idx];
        if (!current) return;
        identifiers[idx] = { ...current, property_id: { Custom: label } };
    }

    function add() {
        identifiers = [...identifiers, blankThingIdentifier()];
    }
    function remove(idx: number) {
        identifiers = identifiers.filter((_, i) => i !== idx);
    }
</script>

<section class="stack">
    {#each identifiers as identifier, idx (idx)}
        <div class="identifier surface">
            <FieldRow>
                <LabeledField label="Type" for={`id-type-${idx}`}>
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
