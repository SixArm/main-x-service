<!--
  Identity-document expiry calendar (`/expiry`) — every worker's
  identity documents with an expiry date, as all-day events in the
  SVAR Calendar (month view, read-only). Selecting an entry opens the
  worker's detail page. Loads via the search endpoint (capped), so
  the view is a window, not a promise of completeness.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { Calendar, Willow } from "@svar-ui/svelte-calendar";
    import type { CalendarInstanceApi } from "@svar-ui/svelte-calendar";
    import { WorkerRepository } from "$lib/api/workers";
    import type { Worker } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte.js";

    const repo = WorkerRepository.withFetch();

    let records = $state<Worker[]>([]);
    let loading = $state(true);
    let error = $state<string | null>(null);

    onMount(async () => {
        try {
            const res = await repo.search({ q: "*", limit: 200 });
            records = res.items;
        } catch (err) {
            error = err instanceof Error ? err.message : String(err);
        } finally {
            loading = false;
        }
    });

    // One all-day event per document expiry date; the id keeps the
    // owning record so selection can navigate to it.
    //
    // T-28 (found while writing its Playwright test, not by inspection):
    // `end` must be *exclusive* for an all-day event — `@svar-ui/
    // calendar-store` filters out any event whose `end` is not strictly
    // after its `start` (`!(e.end > start)`), so `end: day` (the same
    // Date as `start`) made every expiry event vanish silently: this
    // calendar has never actually shown a document on it. `end` is now
    // the following day, the minimum exclusive span a single calendar
    // day needs.
    const events = $derived(
        records.flatMap((record) =>
            (record.documents ?? [])
                .filter((doc) => doc.expiry_date)
                .map((doc, index) => {
                    const day = new Date(doc.expiry_date ?? "");
                    const nextDay = new Date(
                        day.getFullYear(),
                        day.getMonth(),
                        day.getDate() + 1,
                    );
                    return {
                        id: `${record.id ?? ""}::${index}`,
                        start: day,
                        end: nextDay,
                        allDay: true,
                        text: `${record.name.family} — ${doc.document_type}`,
                    };
                }),
        ),
    );

    function init(api: CalendarInstanceApi) {
        api.on("select-event", (raw) => {
            const ev = raw as { id: string | number | null };
            const recordId = String(ev.id ?? "").split("::")[0];
            if (recordId) void goto(`/workers/${recordId}`);
        });
    }
</script>

<svelte:head><title>{t("nav.expiry")} — Main X</title></svelte:head>

<h1>{t("nav.expiry")}</h1>

{#if loading}
    <p>{t("detail.loading")}</p>
{:else if error}
    <p class="error" role="alert">{error}</p>
{:else}
    <div class="calendar-wrap" data-testid="expiry-calendar">
        <Willow>
            <Calendar {events} view="month" readonly {init} />
        </Willow>
    </div>
{/if}

<style>
    .calendar-wrap {
        height: 640px;
    }
</style>
