<script lang="ts">
  import { listEmployees, money } from "$lib/api/hcm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type { Employee } from "$lib/api/types";

  let employees = $state<Employee[] | null>(null);
  let error = $state<string | null>(null);
  let department = $state("");

  async function load() {
    try {
      employees = await listEmployees(department ? { department } : undefined);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    void load();
  });
</script>

<h1>{t("nav.employees")}</h1>

<div class="panel">
  <label>
    {t("common.department")}
    <input bind:value={department} onchange={() => void load()} placeholder="engineering" />
  </label>
</div>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if employees === null}
  <p>{t("common.loading")}</p>
{:else}
  <table data-testid="employee-table">
    <thead>
      <tr>
        <th>{t("emp.number")}</th>
        <th>{t("common.name")}</th>
        <th>{t("common.jobTitle")}</th>
        <th>{t("common.department")}</th>
        <th>{t("common.status")}</th>
        <th>{t("emp.salary")}</th>
      </tr>
    </thead>
    <tbody>
      {#each employees as employee (employee.pid)}
        <tr>
          <td><a href={`/employees/${employee.pid}`}>{employee.employee_number}</a></td>
          <td>{employee.display_name}</td>
          <td>{employee.job_title}</td>
          <td>{employee.department}</td>
          <td><span class={`chip status-${employee.status}`}>{employee.status}</span></td>
          <td>
            {#if employee.salary_minor === null}
              <span class="muted">{t("common.masked")}</span>
            {:else}
              {money(employee.salary_minor, employee.salary_currency, i18n.locale)}
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
