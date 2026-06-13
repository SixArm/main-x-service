<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import EventForm from "$lib/components/EventForm.svelte";
    import { EventRepository } from "$lib/api/events.js";
    import type { Event } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();
    let event = $state<Event | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    const id = $derived(page.params.id as string);

    onMount(async () => {
        try {
            event = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    async function handleSubmit(value: Event) {
        await repo.update(id, value);
        goto(`/events/${id}`);
    }
</script>

<svelte:head><title>Edit event · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Edit event</h1>
    <a href={`/events/${id}`} class="button">Cancel</a>
</header>

{#if loading}
    <p class="muted">Loading…</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if event}
    <section class="surface stack">
        <EventForm initial={event} submitLabel="Save changes" onsubmit={handleSubmit} />
    </section>
{/if}
