<script lang="ts">
    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { ApiError } from '$lib/api/client';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';

    let name = $state('');
    let description = $state('');
    let nameError = $state('');
    let submitError = $state('');

    async function handleSubmit() {
        nameError = '';
        submitError = '';
        if (!name.trim()) {
            nameError = 'Building name is required.';
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

<BackLink href="/buildings">Back to buildings</BackLink>

<h2>Add a building</h2>
<p>One building can have many rooms; each room can hold many cabinets.</p>

{#if submitError}
    <Alert type="error" heading="Cannot save building">{submitError}</Alert>
{/if}

<Form label="Add building" onsubmit={handleSubmit}>
    <Field label="Building name" required error={nameError}>
        <input bind:value={name} required />
    </Field>
    <Field label="Description">
        <textarea bind:value={description} rows="2"></textarea>
    </Field>
    <div class="actions">
        <a href="/buildings" class="button secondary">Cancel</a>
        <Button type="submit">Save building</Button>
    </div>
</Form>
