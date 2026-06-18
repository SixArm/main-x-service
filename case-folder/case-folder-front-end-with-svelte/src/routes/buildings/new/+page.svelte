<script lang="ts">
    // New building (`/buildings/new`) — create a top-level place.
    //
    // Buildings have no parent (a building contains rooms contain
    // cabinets). Requires a name; on success routes to the new building's
    // page (where rooms can be added). 422 maps the `name` error to the field.
    //
    // State: name/description fields + nameError + submitError.

    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { ApiError } from '$lib/api/client';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import { t } from '$lib/i18n.svelte';

    let name = $state('');
    let description = $state('');
    let nameError = $state('');
    let submitError = $state('');

    async function handleSubmit() {
        nameError = '';
        submitError = '';
        if (!name.trim()) {
            nameError = t('buildingNew.nameRequired');
            return;
        }
        try {
            const id = await cache.addBuilding({
                name: name.trim(),
                description: description.trim() || undefined
            });
            await goto(`/buildings/${id}`);
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                nameError = body?.errors?.name ?? '';
                if (!nameError) submitError = e.message;
            } else {
                submitError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/buildings">{t('buildingNew.backToBuildings')}</BackLink>

<h2>{t('buildingNew.heading')}</h2>
<p>{t('buildingNew.intro')}</p>

{#if submitError}
    <Alert type="error" heading={t('buildingNew.cannotSave')}>{submitError}</Alert>
{/if}

<Form label={t('buildingNew.formLabel')} onsubmit={handleSubmit}>
    <Field label={t('buildingNew.nameLabel')} required error={nameError}>
        <input bind:value={name} required />
    </Field>
    <Field label={t('common.description')}>
        <textarea bind:value={description} rows="2"></textarea>
    </Field>
    <div class="actions">
        <a href="/buildings" class="button secondary">{t('common.cancel')}</a>
        <Button type="submit">{t('buildingNew.saveBuilding')}</Button>
    </div>
</Form>
