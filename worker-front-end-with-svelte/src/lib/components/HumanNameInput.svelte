<script lang="ts">
    import type { HumanName } from "$lib/api/types.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";

    let {
        name = $bindable(),
        errors = {},
        prefix = "name",
    }: {
        name: HumanName;
        errors?: Record<string, string>;
        prefix?: string;
    } = $props();

    let givenJoined = $derived(name.given.join(" "));

    function updateGiven(value: string) {
        name.given = value
            .split(/\s+/)
            .map((s) => s.trim())
            .filter(Boolean);
    }
</script>

<FieldRow>
    <LabeledField label="Family name" for={`${prefix}-family`} required error={errors.family}>
        <input id={`${prefix}-family`} bind:value={name.family} required />
    </LabeledField>
    <LabeledField label="Given names" for={`${prefix}-given`} required error={errors.given} hint="Space-separated">
        <input
            id={`${prefix}-given`}
            value={givenJoined}
            oninput={(e) => updateGiven((e.target as HTMLInputElement).value)}
            required
        />
    </LabeledField>
</FieldRow>
