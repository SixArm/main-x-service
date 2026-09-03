<!--
  Edit-event page (route "/events/[id]/edit") — loads the event by id,
  prefills the EventForm, and saves changes via PUT before returning to the
  detail page.

  State ($state): the loaded event, error, and loading flag.
  Derived ($derived): `id` read from the route param.
-->
<script lang="ts">
    import { page } from "$app/state";
    import { goto } from "$app/navigation";
    import { onMount } from "svelte";
    import EventForm from "$lib/components/EventForm.svelte";
    import { EventRepository } from "$lib/api/events.js";
    import { t } from "$lib/i18n.svelte.js";
    import type { Event } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();
    let event = $state<Event | null>(null);
    let error = $state<string | null>(null);
    let loading = $state(true);

    // Route param drives which event we load/save (reactive to navigation).
    const id = $derived(page.params.id as string);

    // Load the existing record to prefill the form.
    onMount(async () => {
        try {
            event = await repo.get(id);
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // Persist edits, then navigate back to the detail view.
    async function handleSubmit(value: Event) {
        await repo.update(id, value);
        goto(`/events/${id}`);
    }
</script>

<svelte:head><title>Edit event · {id}</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>{t("edit.title")}</h1>
    <a href={`/events/${id}`} class="button">{t("edit.cancel")}</a>
</header>

{#if loading}
    <p class="muted">{t("edit.loading")}</p>
{:else if error}
    <div class="banner error">{error}</div>
{:else if event}
    <section class="surface stack">
        <EventForm
            initial={event}
            submitLabel={t("edit.save")}
            onsubmit={handleSubmit}
        />
    </section>
{/if}
