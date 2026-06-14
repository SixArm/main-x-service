<script lang="ts">
    // New folder (`/folders/new`) — create a folder for a patient.
    //
    // Validates the NHS Number client-side (Modulus-11) before submitting
    // so an obviously-wrong number is caught without a round-trip. On
    // success, routes to the new folder's detail page. Server-side 422
    // field errors are mapped back onto the matching form fields.
    //
    // State: the form fields (nhsNumber/patientName/dateOfBirth/title/
    // cabinetId/notes) and a per-field error string for each, plus a
    // catch-all submitError.

    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { ApiError } from '$lib/api/client';
    import { formatNhsNumber, isValidNhsNumber } from '$lib/store/nhs';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import UnitedKingdomNationalHealthServiceNumberInput from '$lib/components/UnitedKingdomNationalHealthServiceNumberInput/UnitedKingdomNationalHealthServiceNumberInput.svelte';

    let nhsNumber = $state('');
    let patientName = $state('');
    let dateOfBirth = $state('');
    let title = $state('');
    let cabinetId = $state<string>('');
    let notes = $state('');

    let nhsError = $state('');
    let nameError = $state('');
    let dobError = $state('');
    let titleError = $state('');
    let submitError = $state('');

    // Validate locally, create via the cache, then navigate to the folder.
    async function handleSubmit() {
        nhsError = '';
        nameError = '';
        dobError = '';
        titleError = '';
        submitError = '';

        // Client-side Modulus-11 gate before we bother the server.
        const formatted = formatNhsNumber(nhsNumber);
        if (!isValidNhsNumber(formatted)) {
            nhsError = 'Enter a valid 10-digit NHS Number (Modulus 11 check failed).';
        }
        if (!title.trim()) {
            titleError = 'Folder title is required.';
        }
        if (nhsError || titleError) return;

        try {
            const folder = await cache.addFolder({
                nhsNumber: formatted,
                patientName: patientName.trim() || undefined,
                dateOfBirth: dateOfBirth || undefined,
                title: title.trim(),
                cabinetId: cabinetId || null,
                notes: notes.trim() || undefined
            });
            await goto(`/folders/${folder.id}`);
        } catch (e) {
            // Map server validation (snake_case keys) onto the form fields;
            // fall back to a banner if none matched.
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                const errs = body?.errors ?? {};
                if (errs.nhs_number) nhsError = errs.nhs_number;
                if (errs.patient_name) nameError = errs.patient_name;
                if (errs.date_of_birth) dobError = errs.date_of_birth;
                if (errs.title) titleError = errs.title;
                if (!nhsError && !nameError && !dobError && !titleError) {
                    submitError = e.message;
                }
            } else {
                submitError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/folders">Back to folders</BackLink>

<h2>Add a new folder</h2>
<p>
    A folder belongs to one patient. If the patient is not yet registered with
    the Main Patient Service, we'll create them; otherwise the new folder is
    attached to the existing patient record.
</p>

{#if submitError}
    <Alert type="error" heading="Cannot save folder">{submitError}</Alert>
{/if}

<Form label="Add folder" onsubmit={handleSubmit}>
    <Field label="NHS Number" required error={nhsError} description="10 digits, formatted XXX XXX XXXX.">
        <UnitedKingdomNationalHealthServiceNumberInput
            label="NHS Number"
            bind:value={nhsNumber}
            required
        />
    </Field>
    <Field label="Folder title" required error={titleError} description="e.g. Volume 1, Cardiology 2023">
        <input bind:value={title} required />
    </Field>

    <Field label="Patient name" error={nameError} description="Only needed for a new patient.">
        <input bind:value={patientName} />
    </Field>
    <Field label="Date of birth" error={dobError} description="Only needed for a new patient.">
        <input type="date" bind:value={dateOfBirth} />
    </Field>

    <Field label="Initial cabinet" description="Leave blank if the folder is in transit.">
        <select bind:value={cabinetId}>
            <option value="">— In transit —</option>
            {#each cache.cabinets as c (c.id)}
                <option value={c.id}>{c.label} ({c.containerPath})</option>
            {/each}
        </select>
    </Field>
    <Field label="Notes">
        <textarea bind:value={notes} rows="2"></textarea>
    </Field>
    <div class="actions">
        <a href="/folders" class="button secondary">Cancel</a>
        <Button type="submit">Save folder</Button>
    </div>
</Form>
