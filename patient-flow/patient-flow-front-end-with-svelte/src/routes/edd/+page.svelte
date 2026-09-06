<!--
  EDD calendar route (`/edd`) — every occupied bed's expected
  discharge date across the estate in the SVAR Calendar (month view).
  Read-only: EDD changes stay with the stay-update endpoint; this is
  the discharge-planning overview, honest about its `as_of` moment.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { Calendar, Willow } from "@svar-ui/svelte-calendar";
  import type { CalendarInstanceApi } from "@svar-ui/svelte-calendar";
  import { getWards, getWhiteboard } from "$lib/api/flow";
  import type { BedCard } from "$lib/api/types";

  type EddEntry = { stayPid: string; card: BedCard; ward: string };

  let entries = $state<EddEntry[]>([]);
  let asOf = $state<string>("");
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const wards = await getWards();
      const boards = await Promise.all(wards.map((w) => getWhiteboard(w.pid)));
      const next: EddEntry[] = [];
      boards.forEach((board, index) => {
        for (const card of board.cards) {
          if (card.stay_pid && card.edd) {
            next.push({
              stayPid: card.stay_pid,
              card,
              ward: wards[index]?.code ?? "",
            });
          }
        }
        asOf = board.as_of ?? asOf;
      });
      entries = next;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  });

  // One all-day calendar event per expected discharge. Names may
  // arrive masked from the service; the calendar renders whatever the
  // read surface allowed.
  //
  // `@svar-ui/calendar-store` requires an all-day event's `end` to be
  // strictly *after* `start` (`!(e.end > start)` silently drops the
  // event — confirmed against the compiled library source). `end: day`
  // — the same Date as `start` — therefore filtered out every
  // expected-discharge event this calendar was ever asked to show (the
  // same bug worker-front-end's and person-front-end's `/expiry`
  // calendars had). Fixed with the following calendar day, the minimum
  // exclusive span a one-day all-day event needs.
  const events = $derived(
    entries.map((e) => {
      const day = new Date(e.card.edd ?? "");
      const nextDay = new Date(
        day.getFullYear(),
        day.getMonth(),
        day.getDate() + 1,
      );
      return {
        id: e.stayPid,
        start: day,
        end: nextDay,
        allDay: true,
        text: `${e.ward} ${e.card.number} — ${e.card.display_name ?? e.card.number}`,
      };
    }),
  );

  function init(api: CalendarInstanceApi) {
    api.on("select-event", (raw) => {
      const ev = raw as { id: string | number | null };
      if (ev.id) void goto(`/stays/${ev.id}`);
    });
  }
</script>

<svelte:head><title>EDD calendar — Patient Flow</title></svelte:head>

<h1>Expected discharges</h1>
{#if asOf}
  <p class="muted small">as of {new Date(asOf).toLocaleString()}</p>
{/if}

{#if loading}
  <p>Loading…</p>
{:else if error}
  <p class="error">{error}</p>
{:else}
  <div class="calendar-wrap" data-testid="edd-calendar">
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
