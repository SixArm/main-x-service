<!--
  New-event page (route "/events/new") — renders a blank EventForm and
  creates the event on submit. If the service rejects with a 409 conflict,
  the duplicate candidates are surfaced for review instead of navigating.

  State ($state):
    - duplicates: candidate matches returned on a 409 conflict.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import EventForm from "$lib/components/EventForm.svelte";
    import MatchResultsList from "$lib/components/MatchResultsList.svelte";
    import { EventRepository } from "$lib/api/events.js";
    import { ApiError } from "$lib/api/client.js";
    import { t, translate } from "$lib/i18n.svelte.js";
    import type { Event, MatchResult } from "$lib/api/types.js";

    const repo = EventRepository.withFetch();
    let duplicates = $state<MatchResult[]>([]);

    // start_date is required; default to next top-of-hour for convenience.
    function defaultStart(): string {
        const d = new Date();
        d.setMinutes(0, 0, 0);
        d.setHours(d.getHours() + 1);
        return d.toISOString().slice(0, 16);
    }

    const blank: Event = {
        name: "",
        start_date: defaultStart(),
    };

    // Create handler passed to the form. On 409 with candidate details,
    // populate `duplicates` and re-throw a message (so the form shows it)
    // rather than navigating; other errors propagate unchanged.
    async function handleSubmit(value: Event) {
        duplicates = [];
        try {
            const created = await repo.create(value);
            if (created.id) goto(`/events/${created.id}`);
        } catch (err) {
            if (err instanceof ApiError && err.isConflict && Array.isArray(err.details)) {
                duplicates = err.details as MatchResult[];
                throw new Error(translate("new.duplicatesDetected").replace("{n}", String(duplicates.length)));
            }
            throw err;
        }
    }
</script>

<svelte:head><title>New event · Event Service</title></svelte:head>

<header><h1>{t("new.title")}</h1></header>

<section class="surface stack">
    <EventForm initial={blank} submitLabel={t("new.create")} onsubmit={handleSubmit} />
</section>

{#if duplicates.length > 0}
    <MatchResultsList results={duplicates} title={t("new.duplicatesTitle")} />
{/if}
