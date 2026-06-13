<script lang="ts">
    import type { Gender, Worker } from "$lib/api/types.js";
    import { createForm } from "$lib/forms/form.svelte.js";
    import LabeledField from "$lib/forms/LabeledField.svelte";
    import FieldRow from "$lib/forms/FieldRow.svelte";
    import HumanNameInput from "./HumanNameInput.svelte";

    let props: {
        initial: Worker;
        submitLabel?: string;
        onsubmit: (worker: Worker) => Promise<void>;
    } = $props();
    const submitLabel = $derived(props.submitLabel ?? "Save");

    // svelte-ignore state_referenced_locally
    const form = createForm<Worker>({
        initial: props.initial,
        validate(value) {
            const errors: Record<string, string> = {};
            if (!value.name.family.trim()) errors.family = "Required";
            if (!value.name.given.length) errors.given = "At least one given name";
            if (value.birth_date && Date.parse(value.birth_date) > Date.now()) {
                errors.birth_date = "Birth date cannot be in the future";
            }
            return errors;
        },
        onSubmit: (value) => props.onsubmit(value),
    });

    const genders: Gender[] = ["male", "female", "other", "unknown"];

    function handleSubmit(e: SubmitEvent) {
        e.preventDefault();
        void form.submit();
    }
</script>

<form onsubmit={handleSubmit} class="stack">
    <HumanNameInput bind:name={form.value.name} errors={form.errors} />

    <FieldRow>
        <LabeledField label="Birth date" for="dob" error={form.errors.birth_date}>
            <input id="dob" type="date" bind:value={form.value.birth_date} />
        </LabeledField>
        <LabeledField label="Gender" for="gender">
            <select id="gender" bind:value={form.value.gender}>
                {#each genders as g}
                    <option value={g}>{g}</option>
                {/each}
            </select>
        </LabeledField>
        <LabeledField label="Tax ID" for="tax_id" hint="SSN, CPF, TIN">
            <input id="tax_id" bind:value={form.value.tax_id} />
        </LabeledField>
    </FieldRow>

    {#if form.submitError}
        <div class="banner error">{form.submitError}</div>
    {/if}

    <div class="row">
        <button type="submit" class="button primary" disabled={form.submitting}>
            {form.submitting ? "Saving…" : submitLabel}
        </button>
        <button type="button" class="button" onclick={() => form.reset()} disabled={form.submitting}>
            Reset
        </button>
    </div>
</form>
