<script lang="ts">
  import { page } from "$app/state";
  import {
    acknowledgeWellbeing,
    appraisalReport,
    appraisalRequests,
    appraisalStatus,
    changeStatus,
    completeItem,
    createAppraisal,
    employeePayslips,
    eraseEmployee,
    employeeWellbeingPrompts,
    getAppraisal,
    getEmployee,
    listAppraisals,
    listEmployees,
    listEntitlements,
    listLeaveRequests,
    listNotifications,
    listOnboarding,
    listPulseSurveys,
    listReviews,
    listTraining,
    markNotificationRead,
    money,
    nominateRater,
    respondAppraisal,
    submitPulse,
    type AppraisalRequest,
    type AppraisalSummary,
    type Notification,
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
  let appraisals = $state<AppraisalSummary[]>([]);
  let myRequests = $state<AppraisalRequest[]>([]);
  let notifications = $state<Notification[]>([]);
  let requestScores = $state<Record<string, number>>({});
  let requestComment = $state("");
  let openRequest = $state<string | null>(null);
  let openAppraisal = $state<Awaited<ReturnType<typeof getAppraisal>> | null>(null);
  let openReport = $state<Awaited<ReturnType<typeof appraisalReport>> | null>(null);
  let colleagues = $state<Employee[]>([]);
  let nomineePid = $state("");
  let nomineeGroup = $state<"manager" | "peer" | "report">("peer");
  let raterPid = $state("");
  let raterScores = $state<Record<string, number>>({});
  let raterComment = $state("");
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
      appraisals = await listAppraisals(pid);
      myRequests = await appraisalRequests(pid);
      notifications = await listNotifications(pid);
      if (openAppraisal) await toggleAppraisal(openAppraisal.pid, true);
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

  async function toggleAppraisal(appraisalPid: string, keepOpen = false) {
    actionError = null;
    if (!keepOpen && openAppraisal?.pid === appraisalPid) {
      openAppraisal = null;
      openReport = null;
      return;
    }
    try {
      openAppraisal = await getAppraisal(appraisalPid);
      openReport =
        openAppraisal.status === "shared" ? await appraisalReport(appraisalPid) : null;
      if (!colleagues.length) colleagues = await listEmployees();
      raterScores = Object.fromEntries(openAppraisal.competencies.map((c) => [c, 3]));
    } catch (cause) {
      actionError = cause instanceof Error ? cause.message : String(cause);
    }
  }

  async function act(action: () => Promise<unknown>) {
    actionError = null;
    try {
      await action();
      await load();
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
    <p>
      <a href={`/api/proxy/employees/${pid}/subject-access`} target="_blank" rel="noreferrer" data-testid="subject-access">
        {t("emp.subjectAccess")}
      </a>
      {#if employee.status === "terminated" || employee.status === "retired"}
        <button
          data-testid="erase"
          onclick={() => {
            if (window.confirm(t("emp.erase") + "?")) {
              void act(async () => {
                await eraseEmployee(pid);
                window.location.assign("/employees");
              });
            }
          }}
        >{t("emp.erase")}</button>
      {/if}
    </p>
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

  {#if notifications.length}
    <h2>{t("emp.notifications")}</h2>
    <ul class="panel" data-testid="notifications">
      {#each notifications as notification (notification.pid)}
        <li>
          {notification.body}
          {#if notification.read_at === null}
            <button onclick={() => void act(() => markNotificationRead(notification.pid))}>
              {t("notif.markRead")}
            </button>
          {:else}
            <span class="muted">✓</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  {#if myRequests.length}
    <h2>{t("ap.requests")}</h2>
    <ul class="panel" data-testid="appraisal-requests">
      {#each myRequests as request (request.appraisal_pid)}
        <li>
          <strong>{request.subject ?? request.subject_pid.slice(0, 8)}</strong>
          <span class="chip">{request.group}</span>
          {#if openRequest === request.appraisal_pid}
            {#each request.competencies as competency (competency)}
              <label>
                {competency}
                <select bind:value={requestScores[competency]}>
                  {#each [1, 2, 3, 4, 5] as score (score)}
                    <option value={score}>{score}</option>
                  {/each}
                </select>
              </label>
            {/each}
            <input placeholder="…" bind:value={requestComment} />
            <button
              onclick={() => void act(async () => {
                await respondAppraisal(request.appraisal_pid, pid, requestScores, requestComment.trim() || undefined);
                openRequest = null;
              })}
            >{t("ap.respond")}</button>
          {:else}
            <button
              onclick={() => {
                openRequest = request.appraisal_pid;
                requestScores = Object.fromEntries(request.competencies.map((c) => [c, 3]));
                requestComment = "";
              }}
            >{t("ap.respond")}</button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <h2>{t("ap.title")}</h2>
  <div class="panel" data-testid="appraisals">
    <button onclick={() => void act(() => createAppraisal(pid, ["communication", "collaboration", "delivery"]))}>
      {t("ap.new")}
    </button>
    {#each appraisals as appraisal (appraisal.pid)}
      <div>
        <button onclick={() => void toggleAppraisal(appraisal.pid)}>
          <span class="chip">{appraisal.status}</span>
          {appraisal.competencies.join(" · ")}
          ({appraisal.responded}/{appraisal.nominated})
        </button>
        {#if appraisal.status === "draft"}
          <button onclick={() => void act(() => appraisalStatus(appraisal.pid, "collecting"))}>{t("ap.start")}</button>
        {:else if appraisal.status === "collecting"}
          <button onclick={() => void act(() => appraisalStatus(appraisal.pid, "shared"))}>{t("ap.share")}</button>
        {/if}
        {#if openAppraisal?.pid === appraisal.pid}
          <ul>
            {#each openAppraisal.nominations as nomination (nomination.pid)}
              <li>
                {nomination.display_name ?? nomination.rater_pid.slice(0, 8)}
                <span class="chip">{nomination.group}</span>
                {#if nomination.responded}✓{/if}
              </li>
            {/each}
          </ul>
          {#if openAppraisal.status === "draft"}
            <select bind:value={nomineePid}>
              <option value="">—</option>
              {#each colleagues.filter((c) => c.pid !== pid) as colleague (colleague.pid)}
                <option value={colleague.pid}>{colleague.display_name}</option>
              {/each}
            </select>
            <select bind:value={nomineeGroup}>
              <option value="manager">manager</option>
              <option value="peer">peer</option>
              <option value="report">report</option>
            </select>
            <button
              disabled={!nomineePid}
              onclick={() => void act(() => nominateRater(appraisal.pid, nomineePid, nomineeGroup))}
            >{t("ap.nominate")}</button>
          {:else if openAppraisal.status === "collecting"}
            <select bind:value={raterPid}>
              <option value="">—</option>
              {#each openAppraisal.nominations.filter((n) => !n.responded) as nomination (nomination.pid)}
                <option value={nomination.rater_pid}>{nomination.display_name ?? nomination.rater_pid.slice(0, 8)}</option>
              {/each}
            </select>
            {#each openAppraisal.competencies as competency (competency)}
              <label>
                {competency}
                <select bind:value={raterScores[competency]}>
                  {#each [1, 2, 3, 4, 5] as score (score)}
                    <option value={score}>{score}</option>
                  {/each}
                </select>
              </label>
            {/each}
            <input placeholder="…" bind:value={raterComment} />
            <button
              disabled={!raterPid}
              onclick={() => void act(() => respondAppraisal(appraisal.pid, raterPid, raterScores, raterComment.trim() || undefined))}
            >{t("ap.respond")}</button>
          {/if}
          {#if openReport}
            <h3>{t("ap.report")}</h3>
            {#each openReport.groups as group (group.group)}
              <div>
                <strong>{group.group}</strong>
                {#if group.withheld}
                  <span class="muted">{t("ap.withheld")}</span>
                {:else}
                  {#each Object.entries(group.competencies ?? {}) as [competency, cell] (competency)}
                    <span class="chip">{competency}: {cell.mean.toFixed(1)} (n={cell.count})</span>
                  {/each}
                  {#each group.comments ?? [] as comment (comment)}
                    <p class="muted">“{comment}”</p>
                  {/each}
                {/if}
              </div>
            {/each}
            <p class="muted">{openReport.derivation}</p>
          {/if}
        {/if}
      </div>
    {/each}
  </div>

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
