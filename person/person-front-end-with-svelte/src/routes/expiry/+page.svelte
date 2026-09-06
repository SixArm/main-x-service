<!--
  Identity-document expiry calendar (`/expiry`) — every person's
  identity documents with an expiry date, as all-day events in the
  SVAR Calendar (month view, read-only). Selecting an entry opens the
  person's detail page. Loads via the search endpoint (capped), so
  the view is a window, not a promise of completeness.
-->
<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { Calendar, Willow } from "@svar-ui/svelte-calendar";
    import type { CalendarInstanceApi } from "@svar-ui/svelte-calendar";
    import { PersonRepository } from "$lib/api/persons";
    import type { Person } from "$lib/api/types";
    import { t } from "$lib/i18n.svelte.js";

    const repo = PersonRepository.withFetch();

    let records = $state<Person[]>([]);
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
    // `@svar-ui/calendar-store` requires an all-day event's `end` to be
    // strictly *after* `start` (`!(e.end > start)` silently drops the
    // event — confirmed against the compiled library source). `end: day`
    // — the same Date as `start` — therefore filtered out every expiry
    // event this calendar was ever asked to show, since the route
    // shipped (the same bug worker-front-end's `/expiry` calendar had,
    // fixed there first). The fix: the following calendar day, the
    // minimum exclusive span a one-day all-day event needs.
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
            if (recordId) void goto(`/persons/${recordId}`);
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
