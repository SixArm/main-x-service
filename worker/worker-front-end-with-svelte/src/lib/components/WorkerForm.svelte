<!--
  WorkerForm — create/edit form for a Worker. Shared by the "new worker"
  and "edit worker" pages; the caller decides what submit does.

  $props:
    - initial: Worker — the starting value (blank for create, loaded for
      edit). Cloned by createForm, so it isn't mutated.
    - submitLabel?: string — primary button text (default "Save").
    - onsubmit: (worker) => Promise<void> — persistence callback; throwing
      surfaces the error as a submit banner (the new-worker page throws on
      409 to show duplicates).

  $derived:
    - submitLabel — resolved button label with its default.

  State: `form` is a createForm() handle holding value/errors/submitting.
-->
<script lang="ts">
    import type { Gender, Worker } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import HumanNameInput from "./HumanNameInput.svelte";
    import { t } from "$lib/i18n.svelte.js";

    let props: {
        initial: Worker;
        submitLabel?: string;
        onsubmit: (worker: Worker) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? t("form.save"));

    // svelte-ignore state_referenced_locally
    // Reading props.initial once at setup is intentional — createForm clones
    // it, so we don't need it to stay reactive after the form is created.
    const form = createForm<Worker>({
        initial: props.initial,
        // Client-side validation mirroring the service's required-field and
        // birth-date rules, to fail fast before the network round-trip.
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.family.trim())
                errors.family = t("form.errFamilyRequired");
            if (!value.name.given.length)
                errors.given = t("form.errGivenRequired");
            if (value.birth_date && Date.parse(value.birth_date) > Date.now()) {
                errors.birth_date = t("form.errBirthFuture");
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(value),
    });

    const genders: Gender[] = ["male", "female", "other", "unknown"];

    // Suppress native submit; delegate to the reactive form handle.
    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        void form.submit();
    }
</script>

<form onsubmit={handleSubmit} class="stack">
    <!-- Two-way bind the name so edits flow back into form.value. -->
    <HumanNameInput bind:name={form.value.name} errors={form.errors} />

    <FieldRow>
        <LabeledField
            label={t("form.birthDate")}
            for="dob"
            error={form.errors.birth_date}
        >
            <input id="dob" type="date" bind:value={form.value.birth_date} />
        </LabeledField>
        <LabeledField label={t("form.gender")} for="gender">
            <select id="gender" bind:value={form.value.gender}>
                {#each genders as g}
                    <option value={g}>{g}</option>
                {/each}
            </select>
        </LabeledField>
        <LabeledField
            label={t("form.taxId")}
            for="tax_id"
            hint={t("form.taxIdHint")}
        >
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
        <button
            type="button"
            class="button"
            onclick={() => form.reset()}
            disabled={form.submitting}
        >
            {t("form.reset")}
        </button>
    </div>
</form>
