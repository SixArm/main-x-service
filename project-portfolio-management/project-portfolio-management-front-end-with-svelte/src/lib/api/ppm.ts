// PPM catalogue client (Phases A–C): typed wrappers over the
// governance / visibility / strategy endpoints, mirroring the
// service's route table (spec 15-roadmap PPM-1..12). One class, one
// method per endpoint, the single source of PPM paths for the UI.
//
// PPM views ship English-first; extending the 13-locale catalogues to
// these strings is a documented follow-up.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";
import type { Collection } from "./types";

// ---- wire shapes (mirroring the service responses) ----

export interface Proposal {
  pid: string;
  title: string;
  summary: string | null;
  kind_target: Collection;
  sponsor_ref: string | null;
  strategic_rationale: string | null;
  requested_minor: number | null;
  currency: string | null;
  status: "draft" | "submitted" | "in_review" | "approved" | "rejected" | "promoted";
  promoted_work_item_pid: string | null;
}

export interface DemandHit {
  source: "work_item" | "proposal";
  pid: string;
  name: string;
  score: number;
}

export interface Idea {
  pid: string;
  title: string;
  pitch: string | null;
  tags: string[] | unknown;
  votes: number;
  status: "open" | "converted" | "dismissed";
  converted_proposal_pid: string | null;
}

export interface GateJourney {
  stage: string | null;
  next_gate: string | null;
  reviews: {
    pid: string;
    gate: string;
    decision: string;
    conditions: string | null;
    approver_ref: string | null;
    decided_at: string;
  }[];
}

export interface Risk {
  pid: string;
  title: string;
  status: string;
  probability: number;
  impact: number;
  exposure: number;
  owner_ref: string | null;
  mitigation: string | null;
  review_date: string | null;
  escalated_at: string | null;
}

export interface BudgetBoard {
  lines: {
    pid: string;
    category: string;
    description: string;
    currency: string;
    planned_minor: number;
    actual_minor: number;
  }[];
  totals: {
    currency: string;
    planned_minor: number;
    actual_minor: number;
    variance_minor: number;
  }[];
}

export interface GovernanceSummary {
  pid: string;
  name: string;
  stage: string | null;
  next_gate: string | null;
  gate_reviews: number;
  latest_review: { gate: string; decision: string; decided_at: string } | null;
  risks: {
    open: number;
    materialised: number;
    max_exposure: number | null;
    total_exposure: number;
  };
  budget: BudgetBoard["totals"];
}

export interface ScheduleView {
  portfolio_pid: string;
  items: {
    pid: string;
    kind: string;
    name: string;
    stage: string | null;
    start: string | null;
    end: string | null;
    on_critical_path: boolean;
  }[];
  edges: { pid: string; predecessor_pid: string; successor_pid: string; lag_days: number }[];
  violations: {
    edge_pid: string;
    predecessor: string;
    successor: string;
    earliest_start: string;
    actual_start: string;
  }[];
  critical_path: string[];
  undated: string[];
}

export interface Milestone {
  pid: string;
  name: string;
  due: string;
  done: boolean;
  overdue: boolean;
}

export interface Allocation {
  pid: string;
  person_ref: string;
  role: string | null;
  percent: number;
  start_date: string | null;
  end_date: string | null;
}

export interface CapacityView {
  from: string;
  to: string;
  people: {
    person_ref: string;
    allocated_percent: number;
    over_allocated: boolean;
    allocations: number;
  }[];
}

export interface ReportDefinition {
  pid: string;
  name: string;
  collection: Collection;
  filters: Record<string, unknown>;
  fields: string[] | unknown;
}

export interface ReportRun {
  name: string;
  collection: string;
  rows: number;
  data: Record<string, string>[];
}

export interface Dashboard {
  as_of: string;
  collections: {
    collection: string;
    total: number;
    rag: { red: number; amber: number; green: number };
    stages: Record<string, number>;
  }[];
  site_tiles: {
    work_items: number;
    proposals_open: number;
    materialised_risks: number;
    open_risk_exposure: number;
    schedule_violations: number;
    over_allocated_people: number;
  };
}

export interface Scenario {
  pid: string;
  name: string;
  description: string | null;
  status: "draft" | "committed";
  budget_cap_minor: number | null;
  currency: string | null;
}

export interface ScenarioEvaluation {
  pid: string;
  name: string;
  status: string;
  feasible: boolean;
  evaluation: {
    planned_by_currency: [string, number][];
    total_exposure: number;
    total_alignment: number;
    violations: string[];
  };
}

export interface Objective {
  pid: string;
  title: string;
  description: string | null;
  period: string | null;
}

export interface Alignment {
  objective_pid: string;
  title: string;
  period: string | null;
  items: { pid: string; kind: string; name: string; weight: number }[];
  weight_by_collection: Record<string, number>;
  total_weight: number;
}

export interface BenefitBoard {
  benefits: {
    pid: string;
    title: string;
    category: string;
    currency: string | null;
    target_minor: number | null;
    realized_minor: number;
    target_note: string | null;
    status: string;
    expected_on: string | null;
  }[];
  totals: {
    currency: string;
    target_minor: number;
    realized_minor: number;
    spend_minor: number;
    roi_basis_points: number | null;
  }[];
}

/** Format integer minor units as a major-unit string (e.g. 1_234_50 → "1,234.50"). */
export function money(minor: number, currency?: string | null): string {
  const sign = minor < 0 ? "-" : "";
  const abs = Math.abs(minor);
  const major = Math.floor(abs / 100).toLocaleString("en-GB");
  const cents = String(abs % 100).padStart(2, "0");
  return `${sign}${major}.${cents}${currency ? ` ${currency}` : ""}`;
}

/** The PPM endpoints, bound to the configured base URL. */
export class PpmClient {
  constructor(private readonly http: ApiClient) {}

  /** Wire an {@link ApiClient} at {@link API_BASE_URL}. */
  static withFetch(fetchFn?: typeof fetch): PpmClient {
    return new PpmClient(new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }));
  }

  // ---- dashboard ----
  dashboard(): Promise<Dashboard> {
    return this.http.get("/api/at-a-glance");
  }

  // ---- proposals (PPM-1) ----
  listProposals(status?: string): Promise<Proposal[]> {
    return this.http.get(`/api/proposals${status ? `?status=${status}` : ""}`);
  }
  createProposal(body: unknown): Promise<{ pid: string }> {
    return this.http.post("/api/proposals", { body });
  }
  proposalAction(pid: string, action: "submit" | "review" | "approve" | "reject"): Promise<Proposal> {
    return this.http.post(`/api/proposals/${pid}/${action}`, { body: {} });
  }
  promoteProposal(pid: string): Promise<{ work_item_pid: string; collection: Collection }> {
    return this.http.post(`/api/proposals/${pid}/promote`, { body: {} });
  }
  proposalDuplicates(pid: string): Promise<DemandHit[]> {
    return this.http.get(`/api/proposals/${pid}/duplicates`);
  }

  // ---- ideas (PPM-2) ----
  listIdeas(status?: string): Promise<Idea[]> {
    return this.http.get(`/api/ideas${status ? `?status=${status}` : ""}`);
  }
  createIdea(body: unknown): Promise<{ pid: string }> {
    return this.http.post("/api/ideas", { body });
  }
  voteIdea(pid: string): Promise<Idea> {
    return this.http.post(`/api/ideas/${pid}/vote`, { body: {} });
  }
  dismissIdea(pid: string): Promise<Idea> {
    return this.http.post(`/api/ideas/${pid}/dismiss`, { body: {} });
  }
  convertIdea(pid: string, kindTarget: Collection): Promise<{ proposal_pid: string }> {
    return this.http.post(`/api/ideas/${pid}/convert`, { body: { kind_target: kindTarget } });
  }

  // ---- governance panel (PPM-3/10/12 + benefits/objectives) ----
  governance(collection: Collection, pid: string): Promise<GovernanceSummary> {
    return this.http.get(`/api/${collection}/${pid}/governance`);
  }
  gateJourney(collection: Collection, pid: string): Promise<GateJourney> {
    return this.http.get(`/api/${collection}/${pid}/gate-reviews`);
  }
  reviewGate(collection: Collection, pid: string, body: unknown): Promise<{ stage: string | null }> {
    return this.http.post(`/api/${collection}/${pid}/gate-reviews`, { body });
  }
  listRisks(collection: Collection, pid: string): Promise<Risk[]> {
    return this.http.get(`/api/${collection}/${pid}/risks`);
  }
  createRisk(collection: Collection, pid: string, body: unknown): Promise<{ pid: string }> {
    return this.http.post(`/api/${collection}/${pid}/risks`, { body });
  }
  escalateRisk(collection: Collection, pid: string, riskPid: string): Promise<Risk> {
    return this.http.post(`/api/${collection}/${pid}/risks/${riskPid}/escalate`, { body: {} });
  }
  budget(collection: Collection, pid: string): Promise<BudgetBoard> {
    return this.http.get(`/api/${collection}/${pid}/budget-lines`);
  }
  createBudgetLine(collection: Collection, pid: string, body: unknown): Promise<{ pid: string }> {
    return this.http.post(`/api/${collection}/${pid}/budget-lines`, { body });
  }
  recordActual(collection: Collection, pid: string, linePid: string, amountMinor: number): Promise<unknown> {
    return this.http.post(`/api/${collection}/${pid}/budget-lines/${linePid}/actual`, {
      body: { amount_minor: amountMinor },
    });
  }
  benefits(collection: Collection, pid: string): Promise<BenefitBoard> {
    return this.http.get(`/api/${collection}/${pid}/benefits`);
  }
  createBenefit(collection: Collection, pid: string, body: unknown): Promise<{ pid: string }> {
    return this.http.post(`/api/${collection}/${pid}/benefits`, { body });
  }
  realizeBenefit(collection: Collection, pid: string, benefitPid: string, body: unknown): Promise<unknown> {
    return this.http.post(`/api/${collection}/${pid}/benefits/${benefitPid}/realize`, { body });
  }
  itemObjectives(collection: Collection, pid: string): Promise<{ objective_pid: string; title: string; weight: number }[]> {
    return this.http.get(`/api/${collection}/${pid}/objectives`);
  }
  linkObjective(collection: Collection, pid: string, objectivePid: string, weight: number): Promise<unknown> {
    return this.http.post(`/api/${collection}/${pid}/objectives`, {
      body: { objective_pid: objectivePid, weight },
    });
  }
  milestones(collection: Collection, pid: string): Promise<Milestone[]> {
    return this.http.get(`/api/${collection}/${pid}/milestones`);
  }
  createMilestone(collection: Collection, pid: string, body: unknown): Promise<{ pid: string }> {
    return this.http.post(`/api/${collection}/${pid}/milestones`, { body });
  }
  completeMilestone(collection: Collection, pid: string, milestonePid: string): Promise<unknown> {
    return this.http.post(`/api/${collection}/${pid}/milestones/${milestonePid}/complete`, { body: {} });
  }
  allocations(collection: Collection, pid: string): Promise<Allocation[]> {
    return this.http.get(`/api/${collection}/${pid}/allocations`);
  }
  createAllocation(collection: Collection, pid: string, body: unknown): Promise<{ pid: string }> {
    return this.http.post(`/api/${collection}/${pid}/allocations`, { body });
  }

  // ---- schedule (PPM-6) ----
  schedule(portfolioPid: string): Promise<ScheduleView> {
    return this.http.get(`/api/portfolios/${portfolioPid}/schedule`);
  }

  // ---- capacity (PPM-8) ----
  capacity(from?: string, to?: string): Promise<CapacityView> {
    const params = new URLSearchParams();
    if (from) params.set("from", from);
    if (to) params.set("to", to);
    const query = params.toString();
    return this.http.get(`/api/capacity${query ? `?${query}` : ""}`);
  }

  // ---- scenarios (PPM-4) ----
  listScenarios(): Promise<Scenario[]> {
    return this.http.get("/api/scenarios");
  }
  createScenario(body: unknown): Promise<{ pid: string }> {
    return this.http.post("/api/scenarios", { body });
  }
  evaluateScenario(pid: string): Promise<ScenarioEvaluation> {
    return this.http.get(`/api/scenarios/${pid}/evaluate`);
  }
  commitScenario(pid: string): Promise<Scenario> {
    return this.http.post(`/api/scenarios/${pid}/commit`, { body: {} });
  }

  // ---- objectives (PPM-5) ----
  listObjectives(): Promise<Objective[]> {
    return this.http.get("/api/objectives");
  }
  createObjective(body: unknown): Promise<{ pid: string }> {
    return this.http.post("/api/objectives", { body });
  }
  alignment(pid: string): Promise<Alignment> {
    return this.http.get(`/api/objectives/${pid}/alignment`);
  }

  // ---- reports (PPM-9) ----
  listReports(): Promise<ReportDefinition[]> {
    return this.http.get("/api/reports");
  }
  createReport(body: unknown): Promise<{ pid: string }> {
    return this.http.post("/api/reports", { body });
  }
  runReport(pid: string): Promise<ReportRun> {
    return this.http.get(`/api/reports/${pid}/run`);
  }
  /** The CSV download URL for a saved report (same-origin proxy). */
  reportCsvUrl(pid: string): string {
    return `${API_BASE_URL}/api/reports/${pid}/run?format=csv`;
  }
}
