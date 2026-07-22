<!--
  CFO financial area (`/financials`): budget variance (by collection,
  portfolio, and category) and per-currency estate exposure. All
  numbers are server-derived minor units formatted with `money()`;
  currencies are never merged and never converted (the server's note
  is displayed verbatim). English-first, like the other PPM views.
-->
<script lang="ts">
  import { t } from "$lib/i18n.svelte";
  import { onMount } from "svelte";
  import {
    PpmClient,
    money,
    type FinancialExposure,
    type FinancialVariance,
    type VarianceRow,
  } from "$lib/api/ppm";

  const ppm = PpmClient.withFetch();
  let variance = $state<FinancialVariance | null>(null);
  let exposure = $state<FinancialExposure | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      variance = await ppm.financialVariance();
      exposure = await ppm.financialExposure();
    } catch (err) {
      error = err instanceof Error ? err.message : t("ppm.common.loadFailed");
    }
  });
</script>

<svelte:head><title>{t("ppm.nav.financials")} — PPM</title></svelte:head>

<h1>{t("ppm.nav.financials")}</h1>
{#if error}<p class="banner" role="alert">{error}</p>{/if}

{#if exposure}
  <h2>Currency exposure</h2>
  <p class="muted">as of {exposure.as_of} · {exposure.note}</p>
  <table data-testid="fin-exposure">
    <thead>
      <tr>
        <th>Currency</th><th>Planned</th><th>Actual</th><th>Remaining</th>
        <th>Lines</th><th>Work items</th>
      </tr>
    </thead>
    <tbody>
      {#each exposure.currencies as row (row.currency)}
        <tr class:overrun={row.overrun}>
          <td><strong>{row.currency}</strong></td>
          <td>{money(row.planned_minor, row.currency)}</td>
          <td>{money(row.actual_minor, row.currency)}</td>
          <td>{money(row.remaining_minor, row.currency)}</td>
          <td>{row.line_count}</td>
          <td>{row.plans}</td>
        </tr>
      {:else}
        <tr><td colspan="6" class="muted">No budget lines recorded yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

{#snippet varianceTable(label: string, rows: Array<{ key: string; variance: VarianceRow[] }>, testid: string)}
  <h2>{label}</h2>
  <table data-testid={testid}>
    <thead>
      <tr>
        <th>{label}</th><th>Currency</th><th>Planned</th><th>Actual</th>
        <th>Remaining</th><th>Lines</th>
      </tr>
    </thead>
    <tbody>
      {#each rows as group (group.key)}
        {#each group.variance as row, index (row.currency)}
          <tr class:overrun={row.overrun}>
            <td>{#if index === 0}<strong>{group.key}</strong>{/if}</td>
            <td>{row.currency}</td>
            <td>{money(row.planned_minor, row.currency)}</td>
            <td>{money(row.actual_minor, row.currency)}</td>
            <td>{money(row.remaining_minor, row.currency)}</td>
            <td>{row.line_count}</td>
          </tr>
        {/each}
      {:else}
        <tr><td colspan="6" class="muted">No budget lines recorded yet.</td></tr>
      {/each}
    </tbody>
  </table>
{/snippet}

{#if variance}
  {@render varianceTable(
    "By category",
    variance.by_category.map((g) => ({ key: g.category, variance: g.variance })),
    "fin-by-category",
  )}
  {@render varianceTable(
    "By collection",
    variance.by_collection.map((g) => ({ key: g.collection, variance: g.variance })),
    "fin-by-collection",
  )}
  {@render varianceTable(
    "By portfolio",
    variance.by_portfolio.map((g) => ({
      key: g.portfolio?.name ?? "(unassigned)",
      variance: g.variance,
    })),
    "fin-by-portfolio",
  )}
{/if}

<style>
  tr.overrun td { color: #b91c1c; }
</style>
