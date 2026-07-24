<script lang="ts">
  import { benchmarkComparison, listBenchmarks, listEmployees, money } from "$lib/api/wpm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Benchmark, ComparisonRow } from "$lib/api/types";

  let benchmarks = $state<Benchmark[] | null>(null);
  let rows = $state<ComparisonRow[]>([]);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        benchmarks = await listBenchmarks();
        const employees = await listEmployees();
        const organization = employees[0]?.organization_ref;
        rows = organization ? (await benchmarkComparison(organization)).rows : [];
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<h1>{t("nav.benchmarks")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if benchmarks === null}
  <p>{t("common.loading")}</p>
{:else}
  <table data-testid="bands">
    <thead>
      <tr><th>{t("common.jobTitle")}</th><th>min</th><th>median</th><th>max</th></tr>
    </thead>
    <tbody>
      {#each benchmarks as band (band.pid)}
        <tr>
          <td>{band.job_title}</td>
          <td>{money(band.min_minor, band.currency, i18n.locale)}</td>
          <td>{money(band.median_minor, band.currency, i18n.locale)}</td>
          <td>{money(band.max_minor, band.currency, i18n.locale)}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("bench.flag")}</h2>
  <table data-testid="comparison">
    <tbody>
      {#each rows as row (row.employee_pid)}
        <tr>
          <td><a href={`/employees/${row.employee_pid}`}>{row.employee_pid.slice(0, 8)}</a></td>
          <td>{row.job_title}</td>
          <td>{row.department}</td>
          <td>
            {#if row.flag}
              <span class={`chip flag-${row.flag}`}>{row.flag}</span>
            {:else}
              <span class="muted">—</span>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .flag-below_min {
    color: var(--red-day, #c0392b);
  }
  .flag-above_max {
    color: var(--state-reserved, #b57e10);
  }
</style>
