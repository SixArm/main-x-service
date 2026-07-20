<!--
  Follow-ups (`/followups`): the overdue list (server-derived aging)
  and the next 30 days on a SVAR Calendar. The server's note is shown
  verbatim (actor_ref is the recorder, not necessarily an assignee).
-->
<script lang="ts">
  import { Calendar, Willow } from "@svar-ui/svelte-calendar";
  import { followups, type Followup } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";

  let overdue = $state<Followup[] | null>(null);
  let upcoming = $state<Followup[]>([]);
  let note = $state("");
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const view = await followups();
        overdue = view.overdue;
        upcoming = view.upcoming_30d;
        note = view.note;
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const events = $derived(
    upcoming.map((item) => {
      const day = new Date(`${item.due_on}T00:00:00`);
      return {
        id: item.pid,
        start: day,
        end: day,
        allDay: true,
        text: `${item.kind}: ${item.summary}`,
      };
    }),
  );
</script>

<svelte:head><title>{t("nav.followups")} — CRM</title></svelte:head>

<h1>{t("nav.followups")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}
{#if note}<p class="muted">{note}</p>{/if}

{#if overdue !== null}
  <h2>Overdue</h2>
  <table data-testid="followups-overdue">
    <thead>
      <tr><th>Due</th><th>Overdue (days)</th><th>Kind</th><th>Summary</th><th>Recorder</th></tr>
    </thead>
    <tbody>
      {#each overdue as item (item.pid)}
        <tr>
          <td>{item.due_on}</td>
          <td>{item.overdue_days}</td>
          <td>{item.kind}</td>
          <td>{item.summary}</td>
          <td>{item.actor_ref ?? "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted">Nothing overdue.</td></tr>
      {/each}
    </tbody>
  </table>

  <h2>Next 30 days</h2>
  <div class="calendar-wrap" data-testid="followups-calendar">
    <Willow>
      <Calendar {events} view="month" readonly />
    </Willow>
  </div>
{/if}

<style>
  .calendar-wrap {
    height: 560px;
    overflow-x: auto;
  }
</style>
