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
    import { t } from '$lib/i18n.svelte';

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
            nhsError = t('folderNew.invalidNhs');
        }
        if (!title.trim()) {
            titleError = t('folderNew.titleRequired');
        }
        if (nhsError || titleError) return;

        try {
            const folder = await cache.addFolder({
                nhsNumber: formatted,
                patientName: patientName.trim() || undefined,
                dateOfBirth: dateOfBirth || undefined,
                title: title.trim(),
                cabinetId: cabinetId || null,
                notes: notes.trim() || undefined,
            });
            await goto(`/folders/${folder.id}`);
        } catch (e) {
            // Map server validation (snake_case keys) onto the form fields;
            // fall back to a banner if none matched.
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as {
                    errors?: Record<string, string>;
                } | null;
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

<BackLink href="/folders">{t('folderNew.backToFolders')}</BackLink>

<h2>{t('folderNew.heading')}</h2>
<p>{t('folderNew.intro')}</p>

{#if submitError}
    <Alert type="error" heading={t('folderNew.cannotSave')}>{submitError}</Alert
    >
{/if}

<Form label={t('folderNew.formLabel')} onsubmit={handleSubmit}>
    <Field
        label={t('common.nhsNumber')}
        required
        error={nhsError}
        description={t('folderNew.nhsDescription')}
    >
        <UnitedKingdomNationalHealthServiceNumberInput
            label={t('common.nhsNumber')}
            bind:value={nhsNumber}
            required
        />
    </Field>
    <Field
        label={t('folderNew.titleLabel')}
        required
        error={titleError}
        description={t('folderNew.titleDescription')}
    >
        <input bind:value={title} required />
    </Field>

    <Field
        label={t('folderNew.patientName')}
        error={nameError}
        description={t('folderNew.patientNameDescription')}
    >
        <input bind:value={patientName} />
    </Field>
    <Field
        label={t('common.dateOfBirth')}
        error={dobError}
        description={t('folderNew.dobDescription')}
    >
        <input type="date" bind:value={dateOfBirth} />
    </Field>

    <Field
        label={t('folderNew.initialCabinet')}
        description={t('folderNew.initialCabinetDescription')}
    >
        <select bind:value={cabinetId}>
            <option value="">{t('common.inTransitOption')}</option>
            {#each cache.cabinets as c (c.id)}
                <option value={c.id}>{c.label} ({c.containerPath})</option>
            {/each}
        </select>
    </Field>
    <Field label={t('common.notes')}>
        <textarea bind:value={notes} rows="2"></textarea>
    </Field>
    <div class="actions">
        <a href="/folders" class="button secondary">{t('common.cancel')}</a>
        <Button type="submit">{t('folderNew.saveFolder')}</Button>
    </div>
</Form>
