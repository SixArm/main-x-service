<!--
  Partners area (`/partners`): the declared-stakeholder register and
  power–interest grid (declared only — undeclared counted, never
  placed), the innovation-partnership register (forward-only
  lifecycle), and membership renewals + the lapsed list.
-->
<script lang="ts">
  import {
    membershipsView,
    partnershipsRegister,
    stakeholdersView,
  } from "$lib/api/crm";
  import { t } from "$lib/i18n.svelte";

  type Stakeholders = Awaited<ReturnType<typeof stakeholdersView>>;
  type Partnerships = Awaited<ReturnType<typeof partnershipsRegister>>;
  type Memberships = Awaited<ReturnType<typeof membershipsView>>;

  let stakeholders = $state<Stakeholders | null>(null);
  let partnerships = $state<Partnerships | null>(null);
  let memberships = $state<Memberships | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        stakeholders = await stakeholdersView();
        partnerships = await partnershipsRegister();
        memberships = await membershipsView();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });

  const SCALE = [1, 2, 3, 4, 5] as const;
  const cell = (p: number, i: number) => stakeholders?.grid[`p${p}i${i}`] ?? 0;
</script>

<svelte:head><title>{t("nav.partners")} — CRM</title></svelte:head>

<h1>{t("nav.partners")}</h1>
{#if error}<p class="error" data-testid="error">{error}</p>{/if}

{#if stakeholders}
  <h2>Stakeholder register</h2>
  <p class="muted">{stakeholders.note}</p>
  <div class="cols" data-testid="stakeholder-register">
    {#each Object.entries(stakeholders.by_role) as [role, rows] (role)}
      {#if rows.length > 0}
        <section>
          <h3>{role} ({rows.length})</h3>
          <ul>
            {#each rows as row (row.pid)}
              <li>
                {row.display_name}
                <span class="muted">
                  {row.days_since_touch}d ·
                  {row.influence !== null && row.interest !== null
                    ? `P${row.influence}/I${row.interest}`
                    : "no grid scores"}
                </span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/each}
  </div>
  <p class="muted">
    {stakeholders.undeclared_contacts} contacts undeclared ·
    {stakeholders.stakeholders_without_grid_scores} declared without grid scores
  </p>

  <h3>Power–interest grid</h3>
  <table class="matrix" data-testid="stakeholder-grid">
    <thead>
      <tr><th>influence ↓ / interest →</th>{#each SCALE as interest (interest)}<th>{interest}</th>{/each}</tr>
    </thead>
    <tbody>
      {#each [...SCALE].reverse() as influence (influence)}
        <tr>
          <th>{influence}</th>
          {#each SCALE as interest (interest)}
            <td class:hot={influence >= 4 && interest >= 4}>{cell(influence, interest) || ""}</td>
          {/each}
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if partnerships}
  <h2>Innovation partnerships</h2>
  <p class="muted">
    {#each Object.entries(partnerships.by_stage) as [stage, count] (stage)}
      {stage}: {count}&nbsp;
    {/each}
  </p>
  <table data-testid="partnership-register">
    <thead><tr><th>Account</th><th>Kind</th><th>Stage</th><th>Summary</th></tr></thead>
    <tbody>
      {#each partnerships.register as row (row.pid)}
        <tr>
          <td>{row.account ?? row.account_pid}</td>
          <td>{row.kind}</td>
          <td>{row.stage}</td>
          <td>{row.summary}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No partnership records.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if memberships}
  <h2>Memberships</h2>
  <p class="muted">{memberships.memberships} recorded · window {memberships.window_days}d</p>
  <h3>Renewals due</h3>
  <table data-testid="membership-renewals">
    <thead><tr><th>Account</th><th>Status</th><th>Renewal</th></tr></thead>
    <tbody>
      {#each memberships.renewals_due as row (row.pid)}
        <tr><td>{row.account}</td><td>{row.status}</td><td>{row.renewal_on}</td></tr>
      {:else}
        <tr><td colspan="3" class="muted">No renewals due in the window.</td></tr>
      {/each}
    </tbody>
  </table>
  {#if memberships.lapsed.length > 0}
    <h3>Lapsed</h3>
    <ul data-testid="membership-lapsed">
      {#each memberships.lapsed as row (row.pid)}
        <li>{row.account}</li>
      {/each}
    </ul>
  {/if}
{/if}

<style>
  .cols { display: flex; gap: 2rem; flex-wrap: wrap; }
  .cols section { min-width: 12rem; }
  .cols ul { list-style: none; padding: 0; margin: 0; }
  .matrix td { text-align: center; min-width: 2.5rem; }
  .matrix td.hot { background: color-mix(in srgb, #b91c1c 25%, transparent); }
</style>
