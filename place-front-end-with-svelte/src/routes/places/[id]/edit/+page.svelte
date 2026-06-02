<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import PlaceForm from "$lib/components/PlaceForm.svelte";
    import { PlaceRepository } from "$lib/api/places.js";
    import type { Place } from "$lib/api/types.js";

    const repo = PlaceRepository.withFetch();
    let place = $state<Place | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            place = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    async function handleSubmit(value: Place) {
        await repo.update(id, value);
        goto(`/places/${id}`);
    }
</script>

<svelte:head><title>Edit place · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Edit place</h1>
    <a href={`/places/${id}`} class="button">Cancel</a>
</header>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if place}
    <section class="surface stack">
        <PlaceForm initial={place} submitLabel="Save changes" onsubmit={handleSubmit} />
    </section>
{/if}
