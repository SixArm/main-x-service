<script lang="ts">
  import { listRuns } from "$lib/api/hcm";
  import { t } from "$lib/i18n.svelte";
  import type { PayrollRun } from "$lib/api/types";

  let runs = $state<PayrollRun[] | null>(null);
  let error = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        runs = await listRuns();
      } catch (cause) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    })();
  });
</script>

<h1>{t("nav.payroll")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if runs === null}
  <p>{t("common.loading")}</p>
{:else}
  <table data-testid="runs">
    <thead>
      <tr><th></th><th>{t("common.status")}</th></tr>
    </thead>
    <tbody>
      {#each runs as run (run.pid)}
        <tr>
          <td><a href={`/payroll/${run.pid}`}>{run.period_start} → {run.period_end}</a></td>
          <td><span class="chip">{run.status}</span></td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
