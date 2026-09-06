<script lang="ts">
    // New cabinet (`/cabinets/new`) — create a cabinet inside a room.
    //
    // A cabinet must sit in a room (which sits in a building); if no rooms
    // exist yet the form is disabled with guidance. Capacity is optional
    // (blank ⇒ uncapped). On success routes back to the cabinets list.
    //
    // State: label/roomId/capacity/description fields + per-field errors.
    // roomId defaults to the first cached room for convenience.

    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { ApiError } from '$lib/api/client';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import { t } from '$lib/i18n.svelte';

    let label = $state('');
    let roomId = $state<string>(cache.rooms[0]?.id ?? '');
    let capacity = $state<number | string>(80);
    let description = $state('');

    let labelError = $state('');
    let roomError = $state('');
    let submitError = $state('');

    // Label each room with its building so the picker is unambiguous when
    // two buildings have similarly-named rooms.
    const roomOptions = $derived(
        cache.rooms.map((r) => ({
            id: r.id,
            label: `${cache.buildingById(r.buildingId)?.name ?? '?'} — ${r.name}`,
        })),
    );

    async function handleSubmit() {
        labelError = '';
        roomError = '';
        submitError = '';
        if (!label.trim()) labelError = t('cabinetNew.labelRequired');
        if (!roomId) roomError = t('cabinetNew.roomRequired');
        if (labelError || roomError) return;
        try {
            await cache.addCabinet({
                label: label.trim(),
                roomId,
                // Blank ⇒ uncapped (null); otherwise coerce to a sane ≥1 integer.
                capacity:
                    capacity === '' ? null : Math.max(1, Number(capacity) || 1),
                description: description.trim() || undefined,
            });
            await goto('/cabinets');
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as {
                    errors?: Record<string, string>;
                } | null;
                labelError = body?.errors?.name ?? '';
                if (!labelError) submitError = e.message;
            } else {
                submitError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/cabinets">{t('cabinetNew.backToCabinets')}</BackLink>

<h2>{t('cabinetNew.heading')}</h2>
<p>{t('cabinetNew.intro')}</p>

{#if cache.rooms.length === 0}
    <Alert type="warning" heading={t('cabinetNew.noRoomsHeading')}>
        {t('cabinetNew.noRoomsCreate')}
        <a href="/buildings/new">{t('cabinetNew.noRoomsBuilding')}</a>
        {t('cabinetNew.noRoomsBody')}
    </Alert>
{/if}

{#if submitError}
    <Alert type="error" heading={t('cabinetNew.cannotSave')}
        >{submitError}</Alert
    >
{/if}

<Form label={t('cabinetNew.formLabel')} onsubmit={handleSubmit}>
    <Field label={t('cabinetNew.labelLabel')} required error={labelError}>
        <input
            bind:value={label}
            required
            placeholder={t('cabinetNew.labelPlaceholder')}
        />
    </Field>
    <Field label={t('cabinetNew.roomLabel')} required error={roomError}>
        <select bind:value={roomId} required>
            <option value="">{t('cabinetNew.selectRoomOption')}</option>
            {#each roomOptions as r (r.id)}
                <option value={r.id}>{r.label}</option>
            {/each}
        </select>
    </Field>
    <Field
        label={t('cabinetNew.capacityLabel')}
        description={t('cabinetNew.capacityDescription')}
    >
        <input type="number" min="1" bind:value={capacity} />
    </Field>
    <Field label={t('common.description')}>
        <textarea bind:value={description} rows="2"></textarea>
    </Field>
    <div class="actions">
        <a href="/cabinets" class="button secondary">{t('common.cancel')}</a>
        <Button type="submit" disabled={cache.rooms.length === 0}
            >{t('cabinetNew.saveCabinet')}</Button
        >
    </div>
</Form>
