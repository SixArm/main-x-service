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
    import { t } from '$lib/i18n.svelte';

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
            nhsError = t('volumeNew.invalidNhs');
        }
        if (!title.trim()) {
            titleError = t('volumeNew.titleRequired');
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

<BackLink href="/volumes">{t('volumeNew.backToVolumes')}</BackLink>

<h2>{t('volumeNew.heading')}</h2>
<p>{t('volumeNew.intro')}</p>

{#if submitError}
    <Alert type="error" heading={t('volumeNew.cannotCreate')}>{submitError}</Alert>
{/if}

<Form label={t('volumeNew.formLabel')} onsubmit={handleSubmit}>
    <Field label={t('volumeNew.patientNhs')} required error={nhsError} description={t('volumeNew.nhsDescription')}>
        <UnitedKingdomNationalHealthServiceNumberInput
            label={t('common.nhsNumber')}
            bind:value={nhsNumber}
            required
        />
    </Field>
    <Field label={t('volumeNew.titleLabel')} required error={titleError} description={t('volumeNew.titleDescription')}>
        <input bind:value={title} required />
    </Field>
    <Field label={t('volumeNew.initialCabinet')} description={t('volumeNew.initialCabinetDescription')}>
        <select bind:value={cabinetId}>
            <option value="">{t('common.inTransitOption')}</option>
            {#each cache.cabinets as c (c.id)}
                <option value={c.id}>{c.label} ({c.containerPath})</option>
            {/each}
        </select>
    </Field>
    <div class="actions">
        <a href="/volumes" class="button secondary">{t('common.cancel')}</a>
        <Button type="submit">{t('volumeNew.createVolume')}</Button>
    </div>
</Form>
