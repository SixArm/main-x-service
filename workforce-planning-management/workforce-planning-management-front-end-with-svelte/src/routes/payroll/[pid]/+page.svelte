<script lang="ts">
  import { page } from "$app/state";
  import { getRun, money, runAction, runPayslips } from "$lib/api/wpm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Payslip, PayrollRun } from "$lib/api/types";

  /** The action(s) each run status offers (WPM-D5: derive, approve, pay). */
  const ACTIONS: Record<string, ("calculate" | "approve" | "pay" | "reopen")[]> = {
    draft: ["calculate"],
    calculated: ["approve", "reopen"],
    approved: ["pay"],
    paid: [],
  };

  let run = $state<PayrollRun | null>(null);
  let payslips = $state<Payslip[]>([]);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  const pid = $derived(page.params.pid ?? "");

  async function load() {
    try {
      run = await getRun(pid);
      payslips = run.status === "draft" ? [] : await runPayslips(pid);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    if (pid) void load();
  });

  async function act(action: "calculate" | "approve" | "pay" | "reopen") {
    actionError = null;
    try {
      await runAction(pid, action);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if run === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1>
    {run.period_start} → {run.period_end}
    <span class="chip" data-testid="run-status">{run.status}</span>
  </h1>
  <div class="panel">
    {#each ACTIONS[run.status] ?? [] as action (action)}
      <button data-testid={`action-${action}`} onclick={() => void act(action)}>
        {action === "calculate" ? t("pay.calculate") : action === "pay" ? t("pay.pay") : action}
      </button>
    {/each}
    {#if actionError}
      <p class="error" data-testid="action-error">{actionError}</p>
    {/if}
  </div>

  <h2>{t("emp.payslips")}</h2>
  <table data-testid="payslips">
    <thead>
      <tr>
        <th>{t("nav.employees")}</th>
        <th>{t("pay.gross")}</th>
        <th>{t("pay.deductions")}</th>
        <th>{t("pay.net")}</th>
      </tr>
    </thead>
    <tbody>
      {#each payslips as slip (slip.pid)}
        <tr>
          <td><a href={`/employees/${slip.employee_pid}`}>{slip.employee_pid.slice(0, 8)}</a></td>
          <td>{money(slip.gross_minor, slip.currency, i18n.locale)}</td>
          <td>
            {#each slip.deductions as deduction (deduction.label)}
              <span class="chip">{deduction.label}: {money(deduction.amount_minor, slip.currency, i18n.locale)}</span>
            {/each}
          </td>
          <td>{money(slip.net_minor, slip.currency, i18n.locale)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
