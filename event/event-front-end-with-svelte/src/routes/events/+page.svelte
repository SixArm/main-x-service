<!--
  Events list page (route "/events") — search box + facet filters feeding
  the SVAR EventGrid; selecting a row navigates to the detail page.

  State ($state): query text, results (events/total), loading/error flags,
  and the date/status/type filter values.
-->
<script lang="ts">
    import { goto } from "$app/navigation";
    import SearchBox from "$lib/components/SearchBox.svelte";
    import EventGrid from "$lib/components/EventGrid.svelte";
    import { EventRepository } from "$lib/api/events.js";
    import { EVENT_STATUSES, EVENT_TYPES } from "$lib/api/types.js";
    import type { Event, EventStatus, EventType } from "$lib/api/types.js";

    let query = $state("");
    let events = $state<Event[]>([]);
    let total = $state(0);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let dateFrom = $state("");
    let dateTo = $state("");
    let statusFilter = $state<EventStatus | "">("");
    let typeFilter = $state<EventType | "">("");

    // One repository per page (no global HTTP store, per project rules).
    const repo = EventRepository.withFetch();

    // Run a search with the current filters; empty query becomes "*" (all).
    // Empty filter strings are sent as undefined so they are omitted.
    async function runSearch(q: string) {
        loading = true;
        error = null;
        try {
            const res = await repo.search({
                q: q || "*",
                limit: 50,
                date_from: dateFrom || undefined,
                date_to: dateTo || undefined,
                event_status: statusFilter || undefined,
                event_type: typeFilter || undefined,
            });
            events = res.items;
            total = res.total;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
            events = [];
            total = 0;
        } finally {
            loading = false;
        }
    }

    // Grid row selection → navigate to that event's detail page.
    function openEvent(event: Event) {
        if (event.id) goto(`/events/${event.id}`);
    }

    // Run an initial unfiltered search once the component is set up.
    $effect(() => {
        void runSearch("");
    });
</script>

<svelte:head><title>Events · Event Service</title></svelte:head>

<header class="row" style="justify-content: space-between">
    <h1>Events</h1>
    <a href="/events/new" class="button primary">New event</a>
</header>

<section class="surface stack">
    <SearchBox bind:value={query} placeholder="Search by name, organizer, identifier…" onsearch={runSearch} />
    <div class="row small" style="flex-wrap: wrap; gap: 0.75rem">
        <label class="row small">From <input type="date" bind:value={dateFrom} /></label>
        <label class="row small">To <input type="date" bind:value={dateTo} /></label>
        <label class="row small">Status
            <select bind:value={statusFilter}>
                <option value="">any</option>
                {#each EVENT_STATUSES as s}<option value={s}>{s}</option>{/each}
            </select>
        </label>
        <label class="row small">Type
            <select bind:value={typeFilter}>
                <option value="">any</option>
                {#each EVENT_TYPES as t}<option value={t}>{t}</option>{/each}
            </select>
        </label>
        <button type="button" class="button" onclick={() => runSearch(query)}>Apply filters</button>
        <span class="muted" style="margin-left: auto">
            {loading ? "Loading…" : `${total} event${total === 1 ? "" : "s"}`}
        </span>
    </div>
    {#if error}<div class="banner error">{error}</div>{/if}
    <EventGrid {events} onselect={openEvent} />
</section>
