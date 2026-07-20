<!--
  Auditor area (`/auditor`): the estate-wide audit-trail explorer
  (actor/action filters + integrity stats), segregation-of-duties
  findings (rules disclosed by the server), and the evidence-pack CSV
  download. English-first, like the other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import {
    PpmClient,
    type AuditTrail,
    type AuditorFindings,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let trail = $state<AuditTrail | null>(null);
  let findings = $state<AuditorFindings | null>(null);
  let error = $state<string | null>(null);
  let actorFilter = $state("");
  let actionFilter = $state("");

  async function loadTrail() {
    try {
      trail = await ppm.auditorTrail({
        actor: actorFilter.trim() || undefined,
        action: actionFilter.trim() || undefined,
      });
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  }
  onMount(async () => {
    await loadTrail();
    try {
      findings = await ppm.auditorFindings();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });
</script>

<svelte:head><title>{t("ppm.nav.auditor")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.auditor")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if findings}
  <h2>Findings</h2>
  <p class="muted">{findings.note}</p>
  <table data-testid="auditor-findings">
    <thead><tr><th>Rule</th><th>Detail</th></tr></thead>
    <tbody>
      {#each findings.findings as finding, index (index)}
        <tr><td><code>{finding.rule}</code></td><td>{finding.detail}</td></tr>
      {:else}
        <tr><td colspan="2" class="muted">No findings.</td></tr>
      {/each}
    </tbody>
  </table>
  <p data-testid="auditor-actorless">
    Actor-less recorded actions: <strong>{findings.actorless_actions}</strong>
    <span class="muted">(submitted without a verified token)</span>
  </p>
{/if}

<h2>Audit trail</h2>
<form
  class="filters"
  onsubmit={(event) => {
    event.preventDefault();
    void loadTrail();
  }}
>
  <label>Actor <input bind:value={actorFilter} placeholder="bearer sub" /></label>
  <label>Action <input bind:value={actionFilter} placeholder="e.g. merged" /></label>
  <button type="submit">Filter</button>
  <a href={ppm.evidencePackUrl()} download>Evidence pack (CSV)</a>
</form>

{#if trail}
  <p class="muted" data-testid="trail-stats">
    {trail.returned} rows · {trail.stats.distinct_actors} distinct actors ·
    {trail.stats.actorless} actor-less
  </p>
  <table data-testid="auditor-trail">
    <thead><tr><th>When</th><th>Actor</th><th>Action</th><th>Entity</th></tr></thead>
    <tbody>
      {#each trail.rows as row (row.created_at + row.action + row.entity_pid)}
        <tr>
          <td>{row.created_at}</td>
          <td>{row.actor ?? "—"}</td>
          <td><code>{row.action}</code></td>
          <td class="muted">{row.entity_pid.slice(0, 8)}</td>
        </tr>
      {:else}
        <tr><td colspan="4" class="muted">No audit rows match.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .filters { display: flex; gap: 1rem; align-items: end; flex-wrap: wrap; }
</style>
