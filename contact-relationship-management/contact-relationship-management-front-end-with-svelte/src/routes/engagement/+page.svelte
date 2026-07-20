<!--
  Engagement area (`/engagement`): relationship-cadence aging
  (untouched contacts/accounts, no-next-touch coverage), the
  engagement workload (kinds + recorded sentiment — unrecorded stays
  unrecorded), and member-account health with the silent list. All
  server-derived; derivations shown verbatim.
-->
<script lang="ts">
  import { cadence, engagementWorkload, membersHealth } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";

  type Cadence = Awaited<ReturnType<typeof cadence>>;
  type Workload = Awaited<ReturnType<typeof engagementWorkload>>;
  type Members = Awaited<ReturnType<typeof membersHealth>>;

  let aging = $state<Cadence | null>(null);
  let workload = $state<Workload | null>(null);
  let members = $state<Members | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        aging = await cadence();
        workload = await engagementWorkload();
        members = await membersHealth();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<svelte:head><title>{t("nav.engagement")} — CRM</title></svelte:head>

<h1>{t("nav.engagement")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if aging}
  <h2>Relationship cadence</h2>
  <p class="muted">{aging.derivation} · threshold {aging.threshold_days}d</p>
  <p data-testid="cadence-coverage">
    <strong>{aging.contacts_without_next_touch}</strong> contacts with no planned next touch
  </p>
  <h3>Untouched contacts</h3>
  <table data-testid="cadence-contacts">
    <thead><tr><th>Contact</th><th>Role</th><th>Days since touch</th><th>Next touch?</th></tr></thead>
    <tbody>
      {#each aging.untouched_contacts as row (row.pid)}
        <tr>
          <td>{row.display_name}</td>
          <td>{row.stakeholder_role ?? "—"}</td>
          <td>{row.days_since_touch}</td>
          <td>{row.has_next_touch ? "yes" : "no"}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">Everyone touched within the threshold.</td></tr>
      {/each}
    </tbody>
  </table>
  <h3>Untouched accounts</h3>
  <table data-testid="cadence-accounts">
    <thead><tr><th>Account</th><th>Role</th><th>Days since touch</th></tr></thead>
    <tbody>
      {#each aging.untouched_accounts as row (row.pid)}
        <tr>
          <td>{row.display_name}</td>
          <td>{row.stakeholder_role ?? "—"}</td>
          <td>{row.days_since_touch}</td>
        </tr>
      {:else}
        <tr><td colspan="3" class="muted">Every account touched within the threshold.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if workload}
  <h2>Workload ({workload.window_days}d)</h2>
  <p class="muted">{workload.note}</p>
  <p data-testid="workload-touches"><strong>{workload.touches}</strong> touches</p>
  <div class="cols">
    <table data-testid="workload-kinds">
      <thead><tr><th>Kind</th><th>Touches</th></tr></thead>
      <tbody>
        {#each Object.entries(workload.per_kind) as [kind, count] (kind)}
          <tr><td>{kind}</td><td>{count}</td></tr>
        {/each}
      </tbody>
    </table>
    <table data-testid="workload-sentiment">
      <thead><tr><th>Sentiment</th><th>Touches</th></tr></thead>
      <tbody>
        {#each Object.entries(workload.sentiment) as [key, count] (key)}
          <tr><td>{key}</td><td>{count}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

{#if members}
  <h2>Member health</h2>
  <p class="muted">
    {members.derivation} · {members.silent_accounts} silent (over {members.threshold_days}d)
  </p>
  <table data-testid="member-health">
    <thead>
      <tr>
        <th>Account</th><th>Membership</th><th>Contacts</th>
        <th>Days since touch</th><th>Open follow-ups</th><th>Open tickets</th>
      </tr>
    </thead>
    <tbody>
      {#each members.accounts as row (row.pid)}
        <tr class:silent={row.silent}>
          <td>{row.display_name}</td>
          <td>{row.membership ? `${row.membership.status} since ${row.membership.joined_on}` : "—"}</td>
          <td>{row.contacts}</td>
          <td>{row.days_since_touch}</td>
          <td>{row.open_followups}</td>
          <td>{row.open_tickets}</td>
        </tr>
      {:else}
        <tr><td colspan="6" class="muted">No accounts yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .cols { display: flex; gap: 2rem; flex-wrap: wrap; }
  tr.silent td { color: #b45309; }
</style>
