<script lang="ts">
  // Privacy operations (WPM-R30): the retention report — what the
  // next sweep would remove, per table, under the floored horizon —
  // and the sweep itself (destructive; admin-only under enforcement).
  // Erasure lives on the employee profile (it is per-person).
  import { retentionReport, retentionSweep } from "$lib/api/wpm";
  import { t } from "$lib/i18n.svelte";

  type Report = Awaited<ReturnType<typeof retentionReport>>;
  type SweepResult = Awaited<ReturnType<typeof retentionSweep>>;

  let report = $state<Report | null>(null);
  let sweepResult = $state<SweepResult | null>(null);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      report = await retentionReport();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function sweep() {
    actionError = null;
    try {
      sweepResult = await retentionSweep();
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

<h1>{t("nav.privacy")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if report === null}
  <p>{t("common.loading")}</p>
{:else}
  <h2>{t("pr.retention")}</h2>
  <div class="panel" data-testid="retention-report">
    <p>{t("pr.horizon")}: <strong>{report.horizon_days}</strong></p>
    <h3>{t("pr.pastHorizon")}</h3>
    {#if Object.keys(report.soft_deleted_past_horizon).length === 0}
      <p class="muted">—</p>
    {:else}
      <table>
        <tbody>
          {#each Object.entries(report.soft_deleted_past_horizon) as [table, count] (table)}
            <tr><td>{table}</td><td>{count}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
    <p>{t("pr.candidates")}: <strong>{report.expired_consent_candidates}</strong></p>
    <p class="muted">{report.derivation}</p>
    <button onclick={() => void sweep()} data-testid="run-sweep">{t("pr.sweep")}</button>
    {#if actionError}
      <p class="error" data-testid="action-error">{actionError}</p>
    {/if}
    {#if sweepResult}
      <p data-testid="sweep-result">
        {sweepResult.rows_deleted} {t("pr.swept")} ·
        {t("pr.candidates")}: {sweepResult.candidates_scrubbed}
      </p>
    {/if}
  </div>
{/if}
