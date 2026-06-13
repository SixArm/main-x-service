<script lang="ts">
    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { ApiError } from '$lib/api/client';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';

    let label = $state('');
    let roomId = $state<string>(cache.rooms[0]?.id ?? '');
    let capacity = $state<number | string>(80);
    let description = $state('');

    let labelError = $state('');
    let roomError = $state('');
    let submitError = $state('');

    const roomOptions = $derived(
        cache.rooms.map((r) => ({
            id: r.id,
            label: `${cache.buildingById(r.buildingId)?.name ?? '?'} — ${r.name}`
        }))
    );

    async function handleSubmit() {
        labelError = '';
        roomError = '';
        submitError = '';
        if (!label.trim()) labelError = 'Cabinet label is required.';
        if (!roomId) roomError = 'Select a room.';
        if (labelError || roomError) return;
        try {
            await cache.addCabinet({
                label: label.trim(),
                roomId,
                capacity: capacity === '' ? null : Math.max(1, Number(capacity) || 1),
                description: description.trim() || undefined
            });
            await goto('/cabinets');
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                labelError = body?.errors?.name ?? '';
                if (!labelError) submitError = e.message;
            } else {
                submitError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/cabinets">Back to cabinets</BackLink>

<h2>Add a file cabinet</h2>
<p>A cabinet lives inside a room (which lives inside a building).</p>

{#if cache.rooms.length === 0}
    <Alert type="warning" heading="No rooms exist yet">
        Create a <a href="/buildings/new">building</a> first, then add a room
        from the building's page.
    </Alert>
{/if}

{#if submitError}
    <Alert type="error" heading="Cannot save cabinet">{submitError}</Alert>
{/if}

<Form label="Add cabinet" onsubmit={handleSubmit}>
    <Field label="Cabinet label" required error={labelError}>
        <input bind:value={label} required placeholder="e.g. Cabinet D3" />
    </Field>
    <Field label="Room" required error={roomError}>
        <select bind:value={roomId} required>
            <option value="">— Select a room —</option>
            {#each roomOptions as r (r.id)}
                <option value={r.id}>{r.label}</option>
            {/each}
        </select>
    </Field>
    <Field label="Capacity" description="Approximate number of folders the cabinet holds. Leave blank for unknown.">
        <input type="number" min="1" bind:value={capacity} />
    </Field>
    <Field label="Description">
        <textarea bind:value={description} rows="2"></textarea>
    </Field>
    <div class="actions">
        <a href="/cabinets" class="button secondary">Cancel</a>
        <Button type="submit" disabled={cache.rooms.length === 0}>Save cabinet</Button>
    </div>
</Form>
