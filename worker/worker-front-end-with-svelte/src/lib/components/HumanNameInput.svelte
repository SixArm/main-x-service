<!--
  HumanNameInput — editor for a structured HumanName (family + given
  names). Bridges the wire model's `given: string[]` to a single
  space-separated text input for editing.

  $props:
    - name: HumanName ($bindable) — the name being edited; mutated in place
      so the parent form sees changes.
    - errors?: Record<string,string> — field errors keyed `family`/`given`.
    - prefix?: string — id prefix so multiple instances get unique input
      ids (default "name").

  $derived:
    - givenJoined — `name.given` rendered as a space-separated string for
      display in the text input.
-->
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

    // Join the given-name array into one editable string for the input.
    let givenJoined = $derived(name.given.join(" "));

    // Parse the space-separated input back into a trimmed, non-empty array.
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
        <!-- Controlled input: display the joined string, re-parse on every
             keystroke so name.given stays a clean token array. -->
        <input
            id={`${prefix}-given`}
            value={givenJoined}
            oninput={(e) => updateGiven((e.target as HTMLInputElement).value)}
            required
        />
    </LabeledField>
</FieldRow>
