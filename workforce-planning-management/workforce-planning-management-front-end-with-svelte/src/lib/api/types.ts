// Wire types mirroring the WPM service's JSON (the service spec is
// the contract; drift is a test failure in the stubbed e2e suite).

/** One employment relationship (salary fields null when masked). */
export interface Employee {
  pid: string;
  person_ref: string;
  worker_ref: string | null;
  organization_ref: string;
  employee_number: string;
  display_name: string;
  status: string;
  employment_type: string;
  fte_percent: number;
  department: string;
  job_title: string;
  manager_pid: string | null;
  salary_minor: number | null;
  salary_currency: string | null;
  hired_on: string;
  terminated_on: string | null;
}

/** One org-chart node (recursive). */
export interface OrgNode {
  pid: string;
  display_name: string;
  job_title: string;
  department: string;
  reports: OrgNode[];
}

/** One funded job opening. */
export interface Requisition {
  pid: string;
  organization_ref: string;
  department: string;
  job_title: string;
  headcount: number;
  salary_min_minor: number | null;
  salary_max_minor: number | null;
  salary_currency: string | null;
  status: string;
  opened_on: string | null;
}

/** One application row. */
export interface Application {
  pid: string;
  requisition_pid: string;
  candidate_pid: string;
  stage: string;
  notes: string | null;
}

/** One onboarding checklist item. */
export interface OnboardingItem {
  pid: string;
  employee_pid: string;
  name: string;
  mandatory: boolean;
  status: string;
  waived_reason: string | null;
}

/** One leave entitlement (balance) row. */
export interface LeaveEntitlement {
  pid: string;
  employee_pid: string;
  kind: string;
  year: number;
  entitled_days: number;
  used_days: number;
}

/** One leave request. */
export interface LeaveRequest {
  pid: string;
  employee_pid: string;
  kind: string;
  start_on: string;
  end_on: string;
  days: number;
  status: string;
  negative_balance: boolean;
}

/** One payroll run. */
export interface PayrollRun {
  pid: string;
  organization_ref: string;
  period_start: string;
  period_end: string;
  status: string;
}

/** One payslip (amounts zeroed when masked). */
export interface Payslip {
  pid: string;
  run_pid: string;
  employee_pid: string;
  currency: string;
  gross_minor: number;
  deductions: { label: string; amount_minor: number }[];
  net_minor: number;
}

/** One benchmark band. */
export interface Benchmark {
  pid: string;
  job_title: string;
  currency: string;
  min_minor: number;
  median_minor: number;
  max_minor: number;
  source: string;
  as_of: string;
}

/**
 * A ratio the service already computed — `numerator`/`denominator` plus
 * the derived `value`, or `null` when the denominator was zero. A zero
 * denominator must render as "no data", never as `0%`: "we measured and
 * it was zero" and "we had nothing to measure" are different claims, and
 * only the service knows which one is true. See `$lib/format.ts`.
 */
export interface Ratio {
  numerator: number;
  denominator: number;
  value: number | null;
}

/** One benchmark-comparison row (flags only, no amounts). */
export interface ComparisonRow {
  employee_pid: string;
  job_title: string;
  department: string;
  benchmark_pid: string | null;
  flag: "below_min" | "within" | "above_max" | null;
}

/** One review row. */
export interface Review {
  pid: string;
  cycle_pid: string;
  employee_pid: string;
  reviewer_ref: string;
  status: string;
  rating: number | null;
  content: string | null;
}

/** One training enrolment. */
export interface TrainingEnrollment {
  pid: string;
  employee_pid: string;
  course_ref: string;
  status: string;
  completed_on: string | null;
  certificate_expires_on: string | null;
}

/** One succession plan with candidates. */
export interface SuccessionEntry {
  plan: {
    pid: string;
    role_title: string;
    department: string;
    criticality: number;
  };
  candidates: {
    pid: string;
    employee_pid: string;
    readiness: string;
    rank: number;
  }[];
}
