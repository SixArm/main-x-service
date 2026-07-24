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

// ─── Wellbeing (health entitlements, WPM-R25) ───────────────────────

/** One configurable health-entitlement rule (non-clinical predicates only). */
export interface WellbeingEntitlement {
  pid: string;
  name: string;
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

/** HR aggregate uptake: counts only, no individuals. */
export function wellbeingUptake(init?: FetchLike): Promise<{
  as_of: string;
  derivation: string;
  entitlements: Array<{
    entitlement_pid: string;
    name: string;
    by_response: Record<string, number>;
    uptake_rate: { numerator: number; denominator: number; value: number | null };
  }>;
}> {
  return api("/wellbeing/uptake", init);
}
