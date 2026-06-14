<script lang="ts">
    // New volume (`/volumes/new`) — create an empty volume for a patient.
    //
    // Validates the NHS Number client-side (Modulus-11) and requires a
    // title. The patient must already exist (folders are added afterwards
    // on the detail page). On success, routes to the new volume. 422
    // field errors are mapped back onto the form.
    //
    // State: nhsNumber/title/cabinetId fields + per-field + submit errors.

    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { api, ApiError } from '$lib/api/client';
    import { formatNhsNumber, isValidNhsNumber } from '$lib/store/nhs';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import UnitedKingdomNationalHealthServiceNumberInput from '$lib/components/UnitedKingdomNationalHealthServiceNumberInput/UnitedKingdomNationalHealthServiceNumberInput.svelte';

    let nhsNumber = $state('');
    let title = $state('');
    let cabinetId = $state<string>('');

    let nhsError = $state('');
    let titleError = $state('');
    let submitError = $state('');

    async function handleSubmit() {
        nhsError = '';
        titleError = '';
        submitError = '';

        // Client-side Modulus-11 gate before contacting the server.
        const formatted = formatNhsNumber(nhsNumber);
        if (!isValidNhsNumber(formatted)) {
            nhsError = 'Enter a valid 10-digit NHS Number (Modulus 11 check failed).';
        }
        if (!title.trim()) {
            titleError = 'Volume title is required.';
        }
        if (nhsError || titleError) return;

        try {
            const volume = await api.volumes.create({
                nhsNumber: formatted,
                title: title.trim(),
                cabinetId: cabinetId || null
            });
            await goto(`/volumes/${volume.id}`);
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                const errs = body?.errors ?? {};
                if (errs.nhs_number) nhsError = errs.nhs_number;
                if (errs.title) titleError = errs.title;
                if (!nhsError && !titleError) submitError = e.message;
            } else {
                submitError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/volumes">Back to volumes</BackLink>

<h2>New volume</h2>
<p>
    A volume bundles folders for one patient. The patient must already be
    registered (create a folder for them first). You can add folders to the
    volume once it exists.
</p>

{#if submitError}
    <Alert type="error" heading="Cannot create volume">{submitError}</Alert>
{/if}

<Form label="New volume" onsubmit={handleSubmit}>
    <Field label="Patient NHS Number" required error={nhsError} description="10 digits, formatted XXX XXX XXXX.">
        <UnitedKingdomNationalHealthServiceNumberInput
            label="NHS Number"
            bind:value={nhsNumber}
            required
        />
    </Field>
    <Field label="Volume title" required error={titleError} description="e.g. Alice Johnson — Vol 1">
        <input bind:value={title} required />
    </Field>
    <Field label="Initial cabinet" description="Where the volume lives. Leave blank if in transit.">
        <select bind:value={cabinetId}>
            <option value="">— In transit —</option>
            {#each cache.cabinets as c (c.id)}
                <option value={c.id}>{c.label} ({c.containerPath})</option>
            {/each}
        </select>
    </Field>
    <div class="actions">
        <a href="/volumes" class="button secondary">Cancel</a>
        <Button type="submit">Create volume</Button>
    </div>
</Form>
