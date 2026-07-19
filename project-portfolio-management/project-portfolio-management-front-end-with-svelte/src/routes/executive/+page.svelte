<!--
  CEO executive area (`/executive`): the portfolio-health briefing
  (server-derived RAG rollup — the derivation string is displayed, not
  re-computed here), the decision log, and benefits realization.
  All three views are ETag-conditional server derivations carrying
  `as_of`; the client formats, it does not compute. English-first
  (locale catalogues extend as a follow-up, matching the other PPM
  views).
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import {
    PpmClient,
    money,
    type DecisionEntry,
    type ExecutiveBenefits,
    type ExecutiveHealth,
    type AlignmentCoverage,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let health = $state<ExecutiveHealth | null>(null);
  let decisions = $state<DecisionEntry[] | null>(null);
  let benefits = $state<ExecutiveBenefits | null>(null);
  let alignment = $state<AlignmentCoverage | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      health = await ppm.executiveHealth();
      decisions = (await ppm.executiveDecisions(30)).decisions;
      benefits = await ppm.executiveBenefits();
      alignment = await ppm.executiveAlignment();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });

  const bucketName = (p: { name: string } | null) => p?.name ?? "(unassigned)";
</script>

<svelte:head><title>{t("ppm.nav.executive")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.executive")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if health}
  <h2>Portfolio health</h2>
  <p class="muted">as of {health.as_of} · {health.derivation}</p>
  <table data-testid="exec-health">
    <thead>
      <tr>
        <th>Portfolio</th><th>Status</th><th>Members</th>
        <th>{t("ppm.dashboard.red")}</th><th>{t("ppm.dashboard.amber")}</th><th>{t("ppm.dashboard.green")}</th>
        <th>Overdue milestones</th><th>Escalated risks</th><th>Open exposure</th>
        <th>Overrun currencies</th><th>Days since update</th>
      </tr>
    </thead>
    <tbody>
      {#each health.portfolios as row (row.portfolio?.pid ?? "unassigned")}
        <tr>
          <td><strong>{bucketName(row.portfolio)}</strong></td>
          <td><span class="rag rag-{row.status}">{row.status}</span></td>
          <td>{row.members}</td>
          <td>{row.rag.red}</td>
          <td>{row.rag.amber}</td>
          <td>{row.rag.green}</td>
          <td>{row.overdue_milestones}</td>
          <td>{row.escalated_risks}</td>
          <td>{row.open_risk_exposure}</td>
          <td>{row.overrun_currencies.join(", ") || "—"}</td>
          <td>{row.days_since_last_update}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

{#if benefits}
  <h2>Benefits realization</h2>
  <p class="muted">{benefits.note}</p>
  <table data-testid="exec-benefits">
    <thead>
      <tr>
        <th>Portfolio</th><th>Benefits</th><th>Currency</th>
        <th>Target</th><th>Realized</th><th>Realization</th>
      </tr>
    </thead>
    <tbody>
      {#each benefits.portfolios as row (row.portfolio?.pid ?? "unassigned")}
        {#if row.financial.length === 0}
          <tr>
            <td><strong>{bucketName(row.portfolio)}</strong></td>
            <td>{row.benefits}</td>
            <td colspan="4" class="muted">non-financial only ({row.non_financial})</td>
          </tr>
        {:else}
          {#each row.financial as line, index (line.currency)}
            <tr>
              <td>{#if index === 0}<strong>{bucketName(row.portfolio)}</strong>{/if}</td>
              <td>{#if index === 0}{row.benefits}{/if}</td>
              <td>{line.currency}</td>
              <td>{money(line.target_minor, line.currency)}</td>
              <td>{money(line.realized_minor, line.currency)}</td>
              <td>
                {#if line.realization_ratio === null}
                  <span class="muted">no target</span>
                {:else}
                  {(line.realization_ratio * 100).toFixed(0)}%
                {/if}
              </td>
            </tr>
          {/each}
        {/if}
      {/each}
    </tbody>
  </table>
{/if}

{#if alignment}
  <h2>Strategic alignment</h2>
  <p class="muted">{alignment.derivation}</p>
  <table data-testid="exec-alignment">
    <thead>
      <tr><th>Collection</th><th>Total</th><th>Aligned</th><th>Unaligned</th></tr>
    </thead>
    <tbody>
      {#each alignment.by_collection as row (row.collection)}
        <tr>
          <td><strong>{row.collection}</strong></td>
          <td>{row.total}</td>
          <td>{row.aligned}</td>
          <td class:warn={row.unaligned > 0}>{row.unaligned}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if alignment.unaligned_spend.length > 0}
    <h3>Unaligned spend</h3>
    <ul data-testid="exec-unaligned-spend">
      {#each alignment.unaligned_spend as row (row.currency)}
        <li><strong>{money(row.planned_minor, row.currency)}</strong> planned with no objective</li>
      {/each}
    </ul>
  {/if}
  {#if alignment.unaligned_items.length > 0}
    <h3>Largest unaligned items</h3>
    <ul data-testid="exec-unaligned-items">
      {#each alignment.unaligned_items as row (row.item.pid)}
        <li>
          {row.item.name} <span class="muted">({row.item.kind})</span>
          {#each row.planned as line (line.currency)}
            · {money(line.planned_minor, line.currency)}
          {/each}
        </li>
      {/each}
    </ul>
  {/if}
{/if}

{#if decisions}
  <h2>Decision log</h2>
  <table data-testid="exec-decisions">
    <thead>
      <tr><th>When</th><th>Kind</th><th>Decision</th><th>Subject</th><th>Actor</th></tr>
    </thead>
    <tbody>
      {#each decisions as entry (entry.kind + entry.at + entry.subject.pid)}
        <tr>
          <td>{entry.at}</td>
          <td>{entry.kind}{#if entry.gate}&nbsp;· {entry.gate}{/if}</td>
          <td>{entry.decision}</td>
          <td>{entry.subject.name ?? entry.subject.pid}</td>
          <td>{entry.actor ?? entry.sponsor ?? "—"}</td>
        </tr>
      {:else}
        <tr><td colspan="5" class="muted">No decisions recorded yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .rag { padding: 0.1rem 0.5rem; border-radius: 999px; color: #fff; }
  .rag-red { background: #b91c1c; }
  .rag-amber { background: #b45309; }
  .rag-green { background: #15803d; }
  td.warn { color: #b45309; font-weight: 600; }
</style>
