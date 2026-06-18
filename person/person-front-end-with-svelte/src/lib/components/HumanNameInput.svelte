<!--
  HumanNameInput — edits a HumanName's family + given names.

  Two-way binds `name` so edits flow straight back to the parent's model.
  Given names are stored as a string[] but edited as one space-separated
  text field for ergonomics.

  Props:
    - name (bindable): HumanName — the name object being edited.
    - errors?: Record<string,string> — per-field messages keyed `family`/`given`.
    - prefix?: string — id prefix so multiple instances get unique input ids.

  State:
    - givenJoined ($derived) — the given[] array rendered as a single string.
-->
<script lang="ts">
    import type { HumanName } from "$lib/api/types.js";
    import { t } from "$lib/i18n.svelte.js";
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

    // Display the given-name array as one space-separated value for editing.
    let givenJoined = $derived(name.given.join(" "));

    // Parse the space-separated input back into a clean string[]:
    // split on whitespace, trim, and drop empties (so trailing spaces and
    // double spaces don't create blank given names).
    function updateGiven(value: string) {
        name.given = value
            .split(/\s+/)
            .map((s) => s.trim())
            .filter(Boolean);
    }
</script>

<FieldRow>
    <LabeledField label={t("name.family")} for={`${prefix}-family`} required error={errors.family}>
        <input id={`${prefix}-family`} bind:value={name.family} required />
    </LabeledField>
    <LabeledField label={t("name.given")} for={`${prefix}-given`} required error={errors.given} hint={t("name.givenHint")}>
        <input
            id={`${prefix}-given`}
            value={givenJoined}
            oninput={(e) => updateGiven((e.target as HTMLInputElement).value)}
            required
        />
    </LabeledField>
</FieldRow>
