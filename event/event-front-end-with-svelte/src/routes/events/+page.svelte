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
    import { t, translate } from "$lib/i18n.svelte.js";
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
    let fuzzy = $state(false);

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
                fuzzy: fuzzy || undefined,
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
    <h1>{t("events.title")}</h1>
    <a href="/events/new" class="button primary">{t("events.new")}</a>
</header>

<section class="surface stack">
    <SearchBox bind:value={query} placeholder={t("events.searchPlaceholder")} onsearch={runSearch} />
    <div class="row small" style="flex-wrap: wrap; gap: 0.75rem">
        <label class="row small">{t("events.filter.from")} <input type="date" bind:value={dateFrom} /></label>
        <label class="row small">{t("events.filter.to")} <input type="date" bind:value={dateTo} /></label>
        <label class="row small">{t("events.filter.status")}
            <select bind:value={statusFilter}>
                <option value="">{t("events.filter.any")}</option>
                {#each EVENT_STATUSES as s}<option value={s}>{s}</option>{/each}
            </select>
        </label>
        <label class="row small">{t("events.filter.type")}
            <select bind:value={typeFilter}>
                <option value="">{t("events.filter.any")}</option>
                {#each EVENT_TYPES as et}<option value={et}>{et}</option>{/each}
            </select>
        </label>
        <label class="row small"><input type="checkbox" bind:checked={fuzzy} /> {t("events.filter.fuzzy")}</label>
        <button type="button" class="button" onclick={() => runSearch(query)}>{t("events.filter.apply")}</button>
        <span class="muted" style="margin-left: auto">
            {loading ? t("events.loading") : translate(total === 1 ? "events.count.one" : "events.count.other").replace("{n}", String(total))}
        </span>
    </div>
    {#if error}<div class="banner error">{error}</div>{/if}
    <EventGrid {events} onselect={openEvent} />
</section>
