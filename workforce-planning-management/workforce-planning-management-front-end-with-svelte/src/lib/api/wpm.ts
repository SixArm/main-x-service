// Typed WPM API surface over the BFF proxy, plus the family `money()`
// formatter. Paths mirror the service routes one-to-one — the
// Playwright suite stubs these exact paths, so drift fails loudly.

import { api } from "$lib/api/client";
import type {
  Application,
  Benchmark,
  ComparisonRow,
  Employee,
  LeaveEntitlement,
  LeaveRequest,
  OnboardingItem,
  OrgNode,
  Payslip,
  PayrollRun,
  Requisition,
  Review,
  SuccessionEntry,
  TrainingEnrollment,
} from "$lib/api/types";

type FetchLike = { fetch?: typeof fetch };

/**
 * Format minor units + ISO-4217 as a locale-aware money string.
 * `null` renders as an em dash (the masked/absent state) — never 0,
 * which would be a lie.
 */
export function money(
  minor: number | null | undefined,
  currency: string | null | undefined,
  locale?: string,
): string {
  if (minor === null || minor === undefined || !currency) return "—";
  return new Intl.NumberFormat(locale, {
    style: "currency",
    currency,
  }).format(minor / 100);
}

/** Employees list (optionally filtered). */
export function listEmployees(
  filters?: { department?: string; status?: string },
  init?: FetchLike,
): Promise<Employee[]> {
  const params = new URLSearchParams();
  if (filters?.department) params.set("department", filters.department);
  if (filters?.status) params.set("status", filters.status);
  const qs = params.size ? `?${params}` : "";
  return api(`/employees${qs}`, init);
}

/** One employee. */
export function getEmployee(pid: string, init?: FetchLike): Promise<Employee> {
  return api(`/employees/${pid}`, init);
}

/** One employee lifecycle transition. */
export function changeStatus(pid: string, to: string): Promise<Employee> {
  return api(`/employees/${pid}/status`, { method: "POST", body: { to } });
}

/** The manager forest for one organization. */
export function orgChart(organization: string, init?: FetchLike): Promise<OrgNode[]> {
  return api(`/org-chart?organization=${encodeURIComponent(organization)}`, init);
}

/** Requisitions (optionally by status). */
export function listRequisitions(status?: string, init?: FetchLike): Promise<Requisition[]> {
  return api(`/requisitions${status ? `?status=${status}` : ""}`, init);
}

/** One requisition. */
export function getRequisition(pid: string, init?: FetchLike): Promise<Requisition> {
  return api(`/requisitions/${pid}`, init);
}

/** One requisition transition. */
export function requisitionStatus(pid: string, to: string): Promise<Requisition> {
  return api(`/requisitions/${pid}/status`, { method: "POST", body: { to } });
}

/** A requisition's applications. */
export function listApplications(pid: string, init?: FetchLike): Promise<Application[]> {
  return api(`/requisitions/${pid}/applications`, init);
}

/** One application stage transition. */
export function applicationStage(
  pid: string,
  body: { to: string; employee_number?: string; salary_minor?: number; salary_currency?: string },
): Promise<{ pid: string; stage: string; employee_pid: string | null }> {
  return api(`/applications/${pid}/stage`, { method: "POST", body });
}

/** An employee's onboarding checklist. */
export function listOnboarding(pid: string, init?: FetchLike): Promise<OnboardingItem[]> {
  return api(`/employees/${pid}/onboarding`, init);
}

/** Complete one checklist item. */
export function completeItem(pid: string): Promise<OnboardingItem> {
  return api(`/onboarding-items/${pid}/complete`, { method: "POST" });
}

/** An employee's leave balances. */
export function listEntitlements(pid: string, init?: FetchLike): Promise<LeaveEntitlement[]> {
  return api(`/employees/${pid}/leave-entitlements`, init);
}

/** An employee's leave requests. */
export function listLeaveRequests(pid: string, init?: FetchLike): Promise<LeaveRequest[]> {
  return api(`/employees/${pid}/leave-requests`, init);
}

/** Decide one leave request. */
export function decideLeave(
  pid: string,
  decision: "approve" | "reject" | "cancel",
): Promise<LeaveRequest> {
  return api(`/leave-requests/${pid}/${decision}`, { method: "POST" });
}

/** The rota (shifts + assignments). */
export function listShifts(
  filters?: { department?: string; date?: string },
  init?: FetchLike,
): Promise<{ shift: { pid: string; department: string; starts_at: string; ends_at: string; required_headcount: number }; assignments: { pid: string; employee_pid: string }[] }[]> {
  const params = new URLSearchParams();
  if (filters?.department) params.set("department", filters.department);
  if (filters?.date) params.set("date", filters.date);
  const qs = params.size ? `?${params}` : "";
  return api(`/shifts${qs}`, init);
}

/** An employee's reviews. */
export function listReviews(pid: string, init?: FetchLike): Promise<Review[]> {
  return api(`/employees/${pid}/reviews`, init);
}

/** An employee's training enrolments. */
export function listTraining(pid: string, init?: FetchLike): Promise<TrainingEnrollment[]> {
  return api(`/employees/${pid}/training-enrollments`, init);
}

/** Certificates expiring within the window. */
export function expiringTraining(
  withinDays: number,
  init?: FetchLike,
): Promise<{ as_of: string; horizon: string; expiring: TrainingEnrollment[] }> {
  return api(`/training/expiring?within_days=${withinDays}`, init);
}

/** Succession plans with ranked candidates. */
export function listSuccession(init?: FetchLike): Promise<SuccessionEntry[]> {
  return api("/succession-plans", init);
}

/** The succession gap report. */
export function successionGaps(init?: FetchLike): Promise<{ gaps: SuccessionEntry["plan"][] }> {
  return api("/succession-plans/gaps", init);
}

/** Payroll runs. */
export function listRuns(init?: FetchLike): Promise<PayrollRun[]> {
  return api("/payroll-runs", init);
}

/** One payroll run. */
export function getRun(pid: string, init?: FetchLike): Promise<PayrollRun> {
  return api(`/payroll-runs/${pid}`, init);
}

/** One run action (calculate / approve / pay / reopen). */
export function runAction(
  pid: string,
  action: "calculate" | "approve" | "pay" | "reopen",
): Promise<PayrollRun> {
  return api(`/payroll-runs/${pid}/${action}`, { method: "POST" });
}

/** A run's payslips. */
export function runPayslips(pid: string, init?: FetchLike): Promise<Payslip[]> {
  return api(`/payroll-runs/${pid}/payslips`, init);
}

/** One employee's payslips (self-service). */
export function employeePayslips(pid: string, init?: FetchLike): Promise<Payslip[]> {
  return api(`/employees/${pid}/payslips`, init);
}

/** Benchmarks. */
export function listBenchmarks(init?: FetchLike): Promise<Benchmark[]> {
  return api("/benchmarks", init);
}

/** The benchmark comparison (flags only). */
export function benchmarkComparison(
  organization: string,
  init?: FetchLike,
): Promise<{ organization: string; rows: ComparisonRow[] }> {
  return api(`/benchmarks/comparison?organization=${encodeURIComponent(organization)}`, init);
}

// ─── Learning & development ─────────────────────────────────────────

/** The skills catalog. */
export function listSkills(init?: FetchLike): Promise<
  Array<{ pid: string; name: string; category: string }>
> {
  return api("/skills", init);
}

/** Declare (upsert) an employee's proficiency in a skill (1-5). */
export function declareSkill(
  employeePid: string,
  body: { skill_pid: string; proficiency: number; target?: number },
): Promise<unknown> {
  return api(`/employees/${employeePid}/skills`, { method: "PUT", body });
}

/** The per-department skills matrix + gaps. */
export function skillsMatrix(init?: FetchLike): Promise<{
  as_of: string;
  note: string;
  matrix: Array<{
    department: string;
    skill: string | null;
    employees: number;
    average_proficiency: number;
    below_target: number;
  }>;
  gaps: Array<{
    employee_pid: string;
    department: string;
    skill: string | null;
    proficiency: number;
    target: number | null;
  }>;
}> {
  return api("/learning/skills-matrix", init);
}

/** Per-department training analytics. */
export function trainingAnalytics(init?: FetchLike): Promise<{
  as_of: string;
  horizon: string;
  note: string;
  departments: Array<{
    department: string;
    by_status: Record<string, number>;
    completion_rate: { numerator: number; denominator: number; value: number | null };
    certs_expiring: number;
  }>;
}> {
  return api("/learning/training-analytics", init);
}

/** Learning paths with step counts. */
export function listPaths(init?: FetchLike): Promise<
  Array<{ pid: string; name: string; summary: string | null; steps: number }>
> {
  return api("/learning-paths", init);
}

/** One path's per-member honest progress. */
export function pathProgress(pathPid: string, init?: FetchLike): Promise<{
  as_of: string;
  path: { pid: string; name: string };
  steps: Array<{ course_ref: string; title: string; position: number }>;
  derivation: string;
  members: Array<{
    employee_pid: string;
    display_name: string | null;
    completed_steps: number;
    total_steps: number;
  }>;
}> {
  return api(`/learning-paths/${pathPid}/progress`, init);
}

/** The mentorship overview (active pairs, load, unmatched, stale). */
export function mentorshipOverview(days?: number, init?: FetchLike): Promise<{
  as_of: string;
  active_pairings: number;
  mentor_load: Array<{ mentor_pid: string; mentor: string | null; active_mentees: number }>;
  unmatched_employees: Array<{ pid: string; display_name: string; department: string }>;
  stale_days: number;
  stale_mentorships: Array<{
    pid: string;
    mentor: string | null;
    mentee: string | null;
    last_session: string | null;
  }>;
}> {
  return api(`/learning/mentorship-overview${days ? `?days=${days}` : ""}`, init);
}

/** Change a mentorship's lifecycle status. */
export function mentorshipStatus(pid: string, to: string): Promise<unknown> {
  return api(`/mentorships/${pid}/status`, { method: "POST", body: { to } });
}

/** The advisory working-time guardrail signals (WPM-R27; flags only). */
export function workingTime(
  department?: string,
  init?: FetchLike,
): Promise<{
  as_of: string;
  reference_weeks: number;
  rest_window_days: number;
  employees_checked: number;
  derivation: string;
  flagged: Array<{
    employee_pid: string;
    display_name: string;
    department: string;
    average_weekly: {
      numerator_minutes: number;
      denominator_weeks: number;
      value_minutes_per_week: number | null;
    };
    over_48h: boolean;
    rest_breaches: Array<{ prev_end: string; next_start: string; gap_minutes: number }>;
  }>;
}> {
  const qs = department ? `?department=${encodeURIComponent(department)}` : "";
  return api(`/workforce/working-time${qs}`, init);
}

// ─── Ergonomic (DSE) assessments (WPM-R32) ──────────────────────────

/** One DSE checklist item (equipment note only — WPM-D24). */
export interface ErgonomicItem {
  pid: string;
  name: string;
  ok: boolean | null;
  note: string | null;
}

/** One workstation assessment with its items. */
export interface ErgonomicAssessment {
  pid: string;
  workstation: string;
  status: "open" | "completed";
  assessed_on: string | null;
  open_issues: number;
  items: ErgonomicItem[];
}

/** The employee's DSE assessments. */
export function listErgonomicAssessments(
  pid: string,
  init?: FetchLike,
): Promise<ErgonomicAssessment[]> {
  return api(`/employees/${pid}/ergonomic-assessments`, init);
}

/** Open an assessment (default DSE checklist when items omitted). */
export function createErgonomicAssessment(
  pid: string,
  workstation: string,
  items?: string[],
): Promise<{ pid: string }> {
  return api(`/employees/${pid}/ergonomic-assessments`, {
    method: "POST",
    body: { workstation, items: items ?? [] },
  });
}

/** Answer one checklist item (ok | issue + equipment note). */
export function answerErgonomicItem(
  pid: string,
  ok: boolean,
  note?: string,
): Promise<ErgonomicItem> {
  return api(`/ergonomic-items/${pid}`, { method: "PUT", body: { ok, note } });
}

/** Complete an assessment (every item must be answered). */
export function completeErgonomicAssessment(pid: string): Promise<ErgonomicAssessment> {
  return api(`/ergonomic-assessments/${pid}/complete`, { method: "POST" });
}

/** Issue-flagged items by department (rota-tier visibility). */
export function ergonomicIssues(init?: FetchLike): Promise<{
  as_of: string;
  by_department: Record<string, number>;
  issues: Array<{
    department: string;
    employee_pid: string;
    display_name: string;
    workstation: string;
    item: string;
    note: string | null;
    assessment_status: string;
  }>;
  derivation: string;
}> {
  return api("/ergonomics/issues", init);
}

// ─── Notifications (WPM-R31) ────────────────────────────────────────

/** One in-app notification (reference-only body, WPM-D23). */
export interface Notification {
  pid: string;
  kind: string;
  body: string;
  data: Record<string, unknown>;
  created_at: string;
  read_at: string | null;
}

/** The employee's notifications, unread first ($sub-owned). */
export function listNotifications(pid: string, init?: FetchLike): Promise<Notification[]> {
  return api(`/employees/${pid}/notifications`, init);
}

/** Mark one notification read (owner-only). */
export function markNotificationRead(pid: string): Promise<Notification> {
  return api(`/notifications/${pid}/read`, { method: "POST" });
}

// ─── Subject rights & retention (WPM-R30) ───────────────────────────

/** The retention report: what the next sweep would remove. */
export function retentionReport(init?: FetchLike): Promise<{
  as_of: string;
  horizon_days: number;
  soft_deleted_past_horizon: Record<string, number>;
  expired_consent_candidates: number;
  derivation: string;
}> {
  return api("/retention", init);
}

/** Run the retention sweep (destructive; admin under enforcement). */
export function retentionSweep(): Promise<{
  horizon_days: number;
  deleted: Record<string, number>;
  rows_deleted: number;
  candidates_scrubbed: number;
}> {
  return api("/retention/sweep", { method: "POST" });
}

/** Erase (anonymise) a terminated/retired employee (destructive). */
export function eraseEmployee(pid: string): Promise<{ erased: string; note: string }> {
  return api(`/employees/${pid}/erase`, { method: "POST" });
}

// ─── 360° appraisals (WPM-R29) ──────────────────────────────────────

/** One appraisal summary row (counts, never content). */
export interface AppraisalSummary {
  pid: string;
  status: "draft" | "collecting" | "shared";
  competencies: string[];
  shared_on: string | null;
  nominated: number;
  responded: number;
}

/** One nomination on the detail view: who (and whether they responded). */
export interface AppraisalNomination {
  pid: string;
  rater_pid: string;
  display_name: string | null;
  group: "self" | "manager" | "peer" | "report";
  responded: boolean;
}

/** One group block on the report: withheld, or disclosed aggregates. */
export interface AppraisalGroup {
  group: string;
  withheld: boolean;
  responses?: number;
  competencies?: Record<string, { count: number; mean: number }>;
  comments?: string[];
}

/** The subject's appraisals. */
export function listAppraisals(pid: string, init?: FetchLike): Promise<AppraisalSummary[]> {
  return api(`/employees/${pid}/appraisals`, init);
}

/** Open a draft 360 (self nomination is automatic). */
export function createAppraisal(
  pid: string,
  competencies: string[],
): Promise<{ pid: string }> {
  return api(`/employees/${pid}/appraisals`, { method: "POST", body: { competencies } });
}

/** One appraisal + its nominations (who responded, never what). */
export function getAppraisal(pid: string, init?: FetchLike): Promise<{
  pid: string;
  employee_pid: string;
  status: string;
  competencies: string[];
  shared_on: string | null;
  nominations: AppraisalNomination[];
}> {
  return api(`/appraisals/${pid}`, init);
}

/** Invite a rater (draft only). */
export function nominateRater(
  appraisalPid: string,
  raterPid: string,
  group: "manager" | "peer" | "report",
): Promise<{ pid: string }> {
  return api(`/appraisals/${appraisalPid}/nominations`, {
    method: "POST",
    body: { rater_pid: raterPid, group },
  });
}

/** Move the appraisal lifecycle (draft → collecting → shared). */
export function appraisalStatus(pid: string, to: string): Promise<unknown> {
  return api(`/appraisals/${pid}/status`, { method: "POST", body: { to } });
}

/** Submit one rater's response (every declared competency, 1–5). */
export function respondAppraisal(
  appraisalPid: string,
  raterPid: string,
  scores: Record<string, number>,
  comment?: string,
): Promise<{ submitted: boolean }> {
  return api(`/appraisals/${appraisalPid}/responses`, {
    method: "POST",
    body: { rater_pid: raterPid, scores, comment },
  });
}

/** One pending 360 request for a rater. */
export interface AppraisalRequest {
  appraisal_pid: string;
  subject_pid: string;
  subject: string | null;
  group: string;
  competencies: string[];
}

/** The rater's own pending 360 requests ($sub-owned). */
export function appraisalRequests(pid: string, init?: FetchLike): Promise<AppraisalRequest[]> {
  return api(`/employees/${pid}/appraisal-requests`, init);
}

/** The group-floored report (shared appraisals only). */
export function appraisalReport(pid: string, init?: FetchLike): Promise<{
  appraisal: { pid: string; employee_pid: string; competencies: string[]; shared_on: string | null };
  groups: AppraisalGroup[];
  derivation: string;
}> {
  return api(`/appraisals/${pid}/report`, init);
}

// ─── Wellbeing (health entitlements, WPM-R25) ───────────────────────

/** One configurable entitlement rule (non-clinical predicates only). */
export interface WellbeingEntitlement {
  pid: string;
  name: string;
  kind: "health" | "benefit";
  benefit_plan_pid: string | null;
  description: string;
  info_url: string | null;
  min_age: number | null;
  max_age: number | null;
  departments: string[];
  job_titles: string[];
  doses: number;
  active_from: string | null;
  active_until: string | null;
}

/** One live prompt (or the one multi-dose reminder) for an employee. */
export interface WellbeingPrompt {
  kind: "prompt" | "reminder";
  entitlement_kind: "health" | "benefit";
  benefit_plan_pid: string | null;
  entitlement_pid: string;
  name: string;
  description: string;
  info_url: string | null;
  doses: number;
  response: string | null;
}

/** The configured entitlement rules. */
export function listWellbeingEntitlements(init?: FetchLike): Promise<WellbeingEntitlement[]> {
  return api("/wellbeing-entitlements", init);
}

/** Add an entitlement rule (HR configuration). */
export function createWellbeingEntitlement(
  body: Partial<Omit<WellbeingEntitlement, "pid">> & { name: string; description: string },
): Promise<{ pid: string }> {
  return api("/wellbeing-entitlements", { method: "POST", body });
}

/** Soft-close an entitlement rule. */
export function deleteWellbeingEntitlement(pid: string): Promise<unknown> {
  return api(`/wellbeing-entitlements/${pid}`, { method: "DELETE" });
}

/** An employee's live prompts (self-service). */
export function employeeWellbeingPrompts(
  pid: string,
  init?: FetchLike,
): Promise<{ as_of: string; age_known: boolean; derivation: string; prompts: WellbeingPrompt[] }> {
  return api(`/employees/${pid}/wellbeing-prompts`, init);
}

/** Acknowledge a prompt (booked | done | declined | dismissed). */
export function acknowledgeWellbeing(
  employeePid: string,
  entitlementPid: string,
  response: "booked" | "done" | "declined" | "dismissed",
): Promise<unknown> {
  return api(`/employees/${employeePid}/wellbeing-acknowledgements`, {
    method: "POST",
    body: { entitlement_pid: entitlementPid, response },
  });
}

/** One anonymous pulse survey. */
export interface PulseSurvey {
  pid: string;
  name: string;
  question: string;
  active_from: string | null;
  active_until: string | null;
  open: boolean;
}

/** One k-floored result cell: suppressed, or disclosed with stats. */
export interface PulseCell {
  suppressed: boolean;
  count?: number;
  distribution?: number[];
  mean?: number;
}

/** The pulse surveys with their open state. */
export function listPulseSurveys(init?: FetchLike): Promise<PulseSurvey[]> {
  return api("/pulse-surveys", init);
}

/** Submit one anonymous 1–5 score (no handle comes back). */
export function submitPulse(
  surveyPid: string,
  employeePid: string,
  score: number,
): Promise<{ submitted: boolean }> {
  return api(`/pulse-surveys/${surveyPid}/responses`, {
    method: "POST",
    body: { employee_pid: employeePid, score },
  });
}

/** The k-floored aggregate results for one survey. */
export function pulseResults(
  surveyPid: string,
  init?: FetchLike,
): Promise<{
  as_of: string;
  survey: { pid: string; name: string; question: string };
  overall: PulseCell;
  departments: Array<PulseCell & { department: string }>;
  derivation: string;
}> {
  return api(`/pulse-surveys/${surveyPid}/results`, init);
}

/** HR aggregate uptake: counts only, no individuals. */
export function wellbeingUptake(init?: FetchLike): Promise<{
  as_of: string;
  derivation: string;
  entitlements: Array<{
    entitlement_pid: string;
    name: string;
    kind: "health" | "benefit";
    by_response: Record<string, number>;
    uptake_rate: { numerator: number; denominator: number; value: number | null };
    enrolment_conversion: {
      numerator: number;
      denominator: number;
      value: number | null;
    } | null;
  }>;
}> {
  return api("/wellbeing/uptake", init);
}
