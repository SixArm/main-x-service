<!--
  Delivery calendar (`/calendar`) — the estate's milestones on a SVAR
  Calendar, filterable by kind (milestone / demo / release /
  checkpoint). Read-only: milestones are created and completed on their
  work item. English-first, like the other PPM views.
-->
<script lang="ts">
  import { Calendar, Willow } from "@svar-ui/svelte-calendar";
  import { PpmClient, type MilestoneCalendar } from "$lib/api/ppm";
  import { t } from "$lib/i18n.svelte";

  const ppm = PpmClient.withFetch();
  let calendar = $state<MilestoneCalendar | null>(null);
  let kind = $state("");
  let error = $state<string | null>(null);

  async function load() {
    try {
      calendar = await ppm.milestoneCalendar(kind || undefined);
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  }
  $effect(() => {
    void load();
  });

  const events = $derived(
    (calendar?.milestones ?? []).map((milestone) => {
      const day = new Date(`${milestone.due}T00:00:00`);
      return {
        id: milestone.pid,
        start: day,
        end: day,
        allDay: true,
        text: `${milestone.kind}: ${milestone.name}${milestone.done ? " ✓" : ""}`,
      };
    }),
  );
</script>

<svelte:head><title>{t("ppm.nav.calendar")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.calendar")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

<p>
  <label>
    Kind
    <select
      value={kind}
      onchange={(event) => {
        kind = event.currentTarget.value;
        void load();
      }}
    >
      <option value="">(all)</option>
      {#each calendar?.kinds ?? [] as option (option)}
        <option value={option}>{option}</option>
      {/each}
    </select>
  </label>
</p>

{#if calendar}
  <div class="calendar-wrap" data-testid="milestone-calendar">
    <Willow>
      <Calendar {events} view="month" readonly />
    </Willow>
  </div>
  <ul data-testid="milestone-list">
    {#each calendar.milestones as milestone (milestone.pid)}
      <li>
        <strong>{milestone.due}</strong> — {milestone.kind}: {milestone.name}
        <span class="muted">({milestone.item?.name ?? "—"}{milestone.done ? " · done" : ""})</span>
      </li>
    {:else}
      <li class="muted">No milestones match.</li>
    {/each}
  </ul>
{/if}

<style>
  .calendar-wrap { height: 560px; overflow-x: auto; margin-bottom: 1rem; }
</style>
