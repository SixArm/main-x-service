<script lang="ts">
  import { decideLeave, listEmployees, listLeaveRequests, listShifts } from "$lib/api/wpm";
  import { t } from "$lib/i18n.svelte";
  import type { Employee, LeaveRequest } from "$lib/api/types";

  type ShiftRow = Awaited<ReturnType<typeof listShifts>>[number];

  let pending = $state<(LeaveRequest & { employee?: Employee })[] | null>(null);
  let shifts = $state<ShiftRow[]>([]);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  async function load() {
    try {
      const employees = await listEmployees();
      const byPid = new Map(employees.map((e) => [e.pid, e]));
      const requestLists = await Promise.all(
        employees.map((e) => listLeaveRequests(e.pid)),
      );
      pending = requestLists
        .flat()
        .filter((r) => r.status === "requested")
        .map((r) => ({ ...r, employee: byPid.get(r.employee_pid) }));
      shifts = await listShifts();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });

  async function decide(request: LeaveRequest, decision: "approve" | "reject") {
    actionError = null;
    try {
      await decideLeave(request.pid, decision);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

<h1>{t("nav.workforce")}</h1>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if pending === null}
  <p>{t("common.loading")}</p>
{:else}
  <h2>{t("wf.leaveRequests")}</h2>
  {#if actionError}
    <p class="error" data-testid="action-error">{actionError}</p>
  {/if}
  <table data-testid="pending-leave">
    <tbody>
      {#each pending as request (request.pid)}
        <tr>
          <td>{request.employee?.display_name ?? request.employee_pid.slice(0, 8)}</td>
          <td>{request.kind}</td>
          <td>{request.start_on} → {request.end_on} ({request.days} {t("common.days")})</td>
          <td>
            <button onclick={() => void decide(request, "approve")}>{t("wf.approve")}</button>
            <button onclick={() => void decide(request, "reject")}>{t("wf.reject")}</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("wf.rota")}</h2>
  <table data-testid="rota">
    <tbody>
      {#each shifts as row (row.shift.pid)}
        <tr>
          <td>{row.shift.department}</td>
          <td>{new Date(row.shift.starts_at).toLocaleString()} → {new Date(row.shift.ends_at).toLocaleTimeString()}</td>
          <td>{row.assignments.length} / {row.shift.required_headcount}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
