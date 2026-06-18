<!--
  PersonForm — create/edit form for a Person's core demographics (name,
  birth date, gender, tax id). Shared by the "new" and "edit" routes.

  Owns a createForm() controller that holds the editable value, validation
  errors, the in-flight flag, and any submit-level error. Client-side
  validation here is a fast first pass; the service re-validates and may
  still return 422.

  Props:
    - initial: Person — the starting value (blank for create, loaded for edit).
    - submitLabel?: string — primary button text (default "Save").
    - onsubmit: (person) => Promise<void> — persistence callback; its thrown
      error surfaces as the form's submitError.

  State:
    - submitLabel ($derived) — resolves the optional prop to its default.
    - form — the reactive form controller (see createForm).
-->
<script lang="ts">
    import type { Gender, Person } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import { t } from "$lib/i18n.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import HumanNameInput from "./HumanNameInput.svelte";

    let props: {
        initial: Person;
        submitLabel?: string;
        onsubmit: (person: Person) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? t("form.save"));

    // createForm clones `props.initial` internally, so reading it here once
    // for setup is intentional — hence the lint suppression below.
    // svelte-ignore state_referenced_locally
    const form = createForm<Person>({
        initial: props.initial,
        // Client-side validation mirroring the service's required-field and
        // no-future-birth-date rules for instant feedback.
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.family.trim()) errors.family = t("form.required");
            if (!value.name.given.length) errors.given = t("form.atLeastOneGiven");
            if (value.birth_date && Date.parse(value.birth_date) > Date.now()) {
                errors.birth_date = t("form.futureBirthDate");
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(value),
    });

    // Options for the gender select, in the API's enum order.
    const genders: Gender[] = ["male", "female", "other", "unknown"];

    // Stop the native submit and delegate to the form controller (which
    // validates first). `void` discards the returned promise deliberately.
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        void form.submit();
    }
</script>

<form onsubmit={handleSubmit} class="stack">
    <HumanNameInput bind:name={form.value.name} errors={form.errors} />

    <FieldRow>
        <LabeledField label={t("form.birthDate")} for="dob" error={form.errors.birth_date}>
            <input id="dob" type="date" bind:value={form.value.birth_date} />
        </LabeledField>
        <LabeledField label={t("form.gender")} for="gender">
            <select id="gender" bind:value={form.value.gender}>
                {#each genders as g}
                    <option value={g}>{g}</option>
                {/each}
            </select>
        </LabeledField>
        <LabeledField label={t("form.taxId")} for="tax_id" hint={t("form.taxIdHint")}>
            <input id="tax_id" bind:value={form.value.tax_id} />
        </LabeledField>
    </FieldRow>

    {#if form.submitError}
        <div class="banner error">{form.submitError}</div>
    {/if}

    <div class="row">
        <button type="submit" class="button primary" disabled={form.submitting}>
            {form.submitting ? t("form.saving") : submitLabel}
        </button>
        <button type="button" class="button" onclick={() => form.reset()} disabled={form.submitting}>
            {t("form.reset")}
        </button>
    </div>
</form>
