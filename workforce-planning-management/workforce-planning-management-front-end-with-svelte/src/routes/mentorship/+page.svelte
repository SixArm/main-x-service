<!--
  Mentorship area (`/mentorship`): the coaching overview — active
  pairings, mentor load (active mentees per mentor), unmatched active
  employees, and stale mentorships (no session within the window).
  All server-derived.
-->
<script lang="ts">
  import { mentorshipOverview } from "$lib/api/hcm";
  import { t } from "$lib/i18n.svelte";

  type Overview = Awaited<ReturnType<typeof mentorshipOverview>>;
  let overview = $state<Overview | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        overview = await mentorshipOverview();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<svelte:head><title>{t("nav.mentorship")} — HCM</title></svelte:head>

<h1>{t("nav.mentorship")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if overview}
  <section class="tiles" data-testid="mentorship-tiles">
    <div class="tile"><strong>{overview.active_pairings}</strong><span>active pairings</span></div>
    <div class="tile"><strong>{overview.unmatched_employees.length}</strong><span>unmatched</span></div>
    <div class="tile">
      <strong>{overview.stale_mentorships.length}</strong>
      <span>stale (over {overview.stale_days}d)</span>
    </div>
  </section>

  <h2>Mentor load</h2>
  <table data-testid="mentor-load">
    <thead><tr><th>Mentor</th><th>Active mentees</th></tr></thead>
    <tbody>
      {#each overview.mentor_load as row (row.mentor_pid)}
        <tr><td>{row.mentor ?? row.mentor_pid}</td><td>{row.active_mentees}</td></tr>
      {:else}
        <tr><td colspan="2" class="muted">No active mentorships.</td></tr>
      {/each}
    </tbody>
  </table>

  {#if overview.stale_mentorships.length > 0}
    <h2>Stale mentorships</h2>
    <table data-testid="stale-mentorships">
      <thead><tr><th>Mentor</th><th>Mentee</th><th>Last session</th></tr></thead>
      <tbody>
        {#each overview.stale_mentorships as row (row.pid)}
          <tr>
            <td>{row.mentor ?? "—"}</td>
            <td>{row.mentee ?? "—"}</td>
            <td>{row.last_session ?? "never"}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  <h2>Unmatched employees</h2>
  <ul data-testid="unmatched">
    {#each overview.unmatched_employees as employee (employee.pid)}
      <li>{employee.display_name} <span class="muted">({employee.department})</span></li>
    {:else}
      <li class="muted">Everyone active is in a mentorship.</li>
    {/each}
  </ul>
{/if}
