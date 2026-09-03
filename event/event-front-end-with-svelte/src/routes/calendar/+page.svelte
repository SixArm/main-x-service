<!--
  Calendar route (`/calendar`) — Event records' time windows in the
  SVAR Calendar (month/week/day). Dragging an event to a new slot
  writes the change back through the normal update endpoint, so the
  calendar is a scheduling surface, not a copy of the truth.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { Calendar, Willow } from "@svar-ui/svelte-calendar";
    import type { CalendarInstanceApi } from "@svar-ui/svelte-calendar";
    import { EventRepository } from "$lib/api/events";
    import type { Event as DomainEvent } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte";

    const repo = EventRepository.withFetch();

    let domainEvents = $state<DomainEvent[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    async function load() {
        try {
            const res = await repo.search({ q: "*", limit: 200 });
            domainEvents = res.items;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    }

    onMount(load);

    // Map domain events to SVAR calendar events. An event with no end
    // date renders as a one-hour slot; all-day events pass the flag
    // through.
    const calendarEvents = $derived(
        domainEvents
            .filter((e) => e.id && e.start_date)
            .map((e) => {
                const start = new Date(e.start_date);
                const end = e.end_date
                    ? new Date(e.end_date)
                    : new Date(start.getTime() + 60 * 60 * 1000);
                return {
                    id: e.id ?? "",
                    start,
                    end,
                    text: e.name,
                    allDay: e.all_day ?? false,
                };
            }),
    );

    // Drag-to-reschedule: write the new window back via the normal
    // update endpoint (full-DTO PUT), then reload the truth. Failures
    // reload too, so the board never drifts from the service.
    function init(api: CalendarInstanceApi) {
        api.on("update-event", (raw) => {
            const ev = raw as {
                id: string | number;
                event: { start?: Date; end?: Date };
            };
            const found = domainEvents.find((d) => d.id === String(ev.id));
            if (!found || !ev.event.start) return;
            const updated: DomainEvent = {
                ...found,
                start_date: ev.event.start.toISOString(),
                end_date: ev.event.end
                    ? ev.event.end.toISOString()
                    : found.end_date,
            };
            void repo
                .update(String(ev.id), updated)
                .catch((err) => {
                    error = err instanceof Error ? err.message : String(err);
                })
                .finally(load);
        });
        api.on("select-event", (raw) => {
            const ev = raw as { id: string | number | null };
            if (ev.id) void goto(`/events/${ev.id}`);
        });
    }
</script>

<svelte:head><title>{t("nav.calendar")} — Main X</title></svelte:head>

<h1>{t("nav.calendar")}</h1>

{#if loading}
    <p>{t("detail.loading")}</p>
{:else if error}
    <p class="error" role="alert">{error}</p>
{:else}
    <div class="calendar-wrap" data-testid="event-calendar">
        <Willow>
            <Calendar events={calendarEvents} view="month" {init} />
        </Willow>
    </div>
{/if}

<style>
    .calendar-wrap {
        height: 640px;
    }
</style>
