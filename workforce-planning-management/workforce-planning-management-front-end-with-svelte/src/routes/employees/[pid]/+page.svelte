<script lang="ts">
  import { page } from "$app/state";
  import {
    acknowledgeWellbeing,
    changeStatus,
    completeItem,
    employeePayslips,
    employeeWellbeingPrompts,
    getEmployee,
    listEntitlements,
    listLeaveRequests,
    listOnboarding,
    listPulseSurveys,
    listReviews,
    listTraining,
    money,
    submitPulse,
    type PulseSurvey,
    type WellbeingPrompt,
  } from "$lib/api/wpm";
  import { i18n, t } from "$lib/i18n.svelte";
  import type {
    Employee,
    LeaveEntitlement,
    LeaveRequest,
    OnboardingItem,
    Payslip,
    Review,
    TrainingEnrollment,
  } from "$lib/api/types";

  let employee = $state<Employee | null>(null);
  let onboarding = $state<OnboardingItem[]>([]);
  let balances = $state<LeaveEntitlement[]>([]);
  let leave = $state<LeaveRequest[]>([]);
  let payslips = $state<Payslip[]>([]);
  let reviews = $state<Review[]>([]);
  let training = $state<TrainingEnrollment[]>([]);
  let wellbeing = $state<WellbeingPrompt[]>([]);
  let pulseSurveys = $state<PulseSurvey[]>([]);
  let pulseThanks = $state<Set<string>>(new Set());
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  const pid = $derived(page.params.pid ?? "");

  async function load() {
    try {
      employee = await getEmployee(pid);
      let prompts: { prompts: WellbeingPrompt[] };
      [onboarding, balances, leave, payslips, reviews, training, prompts] = await Promise.all([
        listOnboarding(pid),
        listEntitlements(pid),
        listLeaveRequests(pid),
        employeePayslips(pid),
        listReviews(pid),
        listTraining(pid),
        employeeWellbeingPrompts(pid),
      ]);
      wellbeing = prompts.prompts;
      pulseSurveys = (await listPulseSurveys()).filter((s) => s.open);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  $effect(() => {
    if (pid) void load();
  });

  async function transition(to: string) {
    actionError = null;
    try {
      await changeStatus(pid, to);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function complete(itemPid: string) {
    actionError = null;
    try {
      await completeItem(itemPid);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function respond(
    entitlementPid: string,
    response: "booked" | "done" | "declined" | "dismissed",
  ) {
    actionError = null;
    try {
      await acknowledgeWellbeing(pid, entitlementPid, response);
      await load();
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function sendPulse(surveyPid: string, score: number) {
    actionError = null;
    try {
      await submitPulse(surveyPid, pid, score);
      pulseThanks = new Set([...pulseThanks, surveyPid]);
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }
</script>

{#if error}
  <p class="error" data-testid="error">{t("common.error")}: {error}</p>
{:else if employee === null}
  <p>{t("common.loading")}</p>
{:else}
  <h1>{employee.display_name} <span class="muted">({employee.employee_number})</span></h1>
  <div class="panel" data-testid="facts">
    <p>
      {employee.job_title} · {employee.department} ·
      <span class={`chip status-${employee.status}`}>{employee.status}</span>
      · FTE {employee.fte_percent}%
    </p>
    <p>
      {t("emp.salary")}:
      {#if employee.salary_minor === null}
        <span class="muted" data-testid="salary-masked">{t("common.masked")}</span>
      {:else}
        <span data-testid="salary">{money(employee.salary_minor, employee.salary_currency, i18n.locale)}</span>
      {/if}
    </p>
    {#if employee.status === "onboarding"}
      <button onclick={() => void transition("active")}>{t("common.actions")}: → active</button>
    {/if}
    {#if actionError}
      <p class="error" data-testid="action-error">{actionError}</p>
    {/if}
  </div>

  {#if wellbeing.length}
    <h2>{t("wb.prompts")}</h2>
    <ul class="panel" data-testid="wellbeing-prompts">
      {#each wellbeing as prompt (prompt.entitlement_pid)}
        <li>
          <strong>{prompt.name}</strong>
          <span class="chip">{prompt.entitlement_kind === "benefit" ? t("wb.kind.benefit") : t("wb.kind.health")}</span>
          {#if prompt.kind === "reminder"}<span class="chip">{t("wb.reminder")}</span>{/if}
          <br />
          {prompt.description}
          {#if prompt.info_url}
            · <a href={prompt.info_url} target="_blank" rel="noreferrer">{t("wb.info")}</a>
          {/if}
          <br />
          <button onclick={() => void respond(prompt.entitlement_pid, "booked")}>{t("wb.booked")}</button>
          <button onclick={() => void respond(prompt.entitlement_pid, "done")}>{t("wb.done")}</button>
          <button onclick={() => void respond(prompt.entitlement_pid, "declined")}>{t("wb.declined")}</button>
          <button onclick={() => void respond(prompt.entitlement_pid, "dismissed")}>{t("wb.dismissed")}</button>
        </li>
      {/each}
    </ul>
  {/if}

  {#if pulseSurveys.length}
    <h2>{t("wb.pulse")}</h2>
    <ul class="panel" data-testid="pulse-surveys">
      {#each pulseSurveys as survey (survey.pid)}
        <li>
          <strong>{survey.name}</strong><br />
          {survey.question}<br />
          {#if pulseThanks.has(survey.pid)}
            <span class="muted">{t("wb.pulseThanks")}</span>
          {:else}
            {#each [1, 2, 3, 4, 5] as score (score)}
              <button onclick={() => void sendPulse(survey.pid, score)}>{score}</button>
            {/each}
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if onboarding.length}
    <h2>{t("emp.onboarding")}</h2>
    <ul class="panel">
      {#each onboarding as item (item.pid)}
        <li>
          {item.name}
          <span class="chip">{item.status}</span>
          {#if item.mandatory}<strong>·</strong>{/if}
          {#if item.status === "pending"}
            <button onclick={() => void complete(item.pid)}>✓</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <h2>{t("emp.balances")}</h2>
  <table>
    <tbody>
      {#each balances as balance (balance.pid)}
        <tr>
          <td>{balance.kind} {balance.year}</td>
          <td>{balance.entitled_days - balance.used_days} / {balance.entitled_days} {t("common.days")}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("wf.leaveRequests")}</h2>
  <table>
    <tbody>
      {#each leave as request (request.pid)}
        <tr>
          <td>{request.kind}</td>
          <td>{request.start_on} → {request.end_on} ({request.days} {t("common.days")})</td>
          <td><span class="chip">{request.status}</span></td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("emp.payslips")}</h2>
  <table>
    <tbody>
      {#each payslips as slip (slip.pid)}
        <tr>
          <td>{t("pay.gross")}: {money(slip.gross_minor, slip.currency, i18n.locale)}</td>
          <td>{t("pay.net")}: {money(slip.net_minor, slip.currency, i18n.locale)}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("emp.reviews")}</h2>
  <table>
    <tbody>
      {#each reviews as review (review.pid)}
        <tr>
          <td><span class="chip">{review.status}</span></td>
          <td>{review.rating ?? "—"}</td>
          <td>{review.content ?? ""}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  <h2>{t("emp.training")}</h2>
  <table>
    <tbody>
      {#each training as enrollment (enrollment.pid)}
        <tr>
          <td>{enrollment.course_ref}</td>
          <td><span class="chip">{enrollment.status}</span></td>
          <td>{enrollment.certificate_expires_on ?? ""}</td>
        </tr>
      {/each}
    </tbody>
  </table>
{/if}
