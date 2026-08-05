// Capability client: collaborative review, assignee management,
// workflow automation, the set-and-forget scheduler, the Smart Score,
// and bird's-eye lifecycle visibility. One class, one method per
// endpoint — the single source of these paths for the UI, mirroring
// the service's `collaboration` / `automation` / `prioritisation`
// controllers.
//
// These views ship English-first, like the rest of the PPM catalogue;
// extending the 13-locale catalogues to them is a documented follow-up.

import { API_BASE_URL } from "$lib/config";
import { ApiClient, type Page, type PageRequest } from "./client";

// ---- collaborative review ----

/** What a review invitation may hang off. */
export type ReviewSubjectKind = "idea" | "proposal" | "plan";

/** Invitation lifecycle; `invited` / `accepted` still owe a verdict. */
export type ReviewStatus =
  | "invited"
  | "accepted"
  | "declined"
  | "submitted"
  | "expired"
  | "withdrawn";

/** Inside the organisation, or an outside expert. */
export type ReviewerScope = "internal" | "external";

/** The verdict a submitted review carries. */
export type Recommendation = "advance" | "hold" | "reject";

/** One delegation of one subject to one expert. */
export interface Review {
  pid: string;
  subject_kind: ReviewSubjectKind;
  subject_pid: string;
  /** EntityRef URN (`person:` / `worker:` / `organization:`). */
  reviewer_ref: string;
  reviewer_scope: ReviewerScope;
  /** Why this expert — the specialism the delegation is for. */
  expertise: string | null;
  status: ReviewStatus;
  due_on: string | null;
  score: number | null;
  recommendation: Recommendation | null;
  comment: string | null;
  invited_by: string | null;
  responded_at: string | null;
  submitted_at: string | null;
}

/**
 * The aggregate verdict on one subject. `majority` is a **strict**
 * majority: a tie or a mere plurality reports `null`, and `mean_score`
 * is `null` until somebody actually scored.
 */
export interface Consensus {
  invited: number;
  submitted: number;
  outstanding: number;
  declined: number;
  mean_score: number | null;
  recommendations: Record<string, number>;
  majority: Recommendation | null;
  external_submitted: number;
  complete: boolean;
}

// ---- assignees + notifications ----

/** One assignee's open pile (`unassigned` is a reserved key). */
export interface AssigneeLoad {
  assignee_ref: string;
  open: number;
  blocked: number;
  in_progress: number;
  open_points: number;
  unestimated: number;
}

/** In-app notification. This service sends no email and no push. */
export interface Notification {
  pid: string;
  recipient_ref: string;
  subject_kind: string;
  subject_pid: string;
  kind: string;
  message: string;
  read_at: string | null;
  created_at: string;
}

// ---- workflow automation ----

/** What can fire a rule. */
export type TriggerKind =
  | "task_moved"
  | "review_submitted"
  | "plan_stage_changed";

/** What a rule may do. */
export type ActionKind =
  | "assign"
  | "add_label"
  | "notify"
  | "schedule_action"
  | "set_task_status";

/** One configured rule. */
export interface Automation {
  pid: string;
  /** Scoped to one plan's board, or `null` for every plan. */
  plan_pid: string | null;
  name: string;
  trigger_kind: TriggerKind;
  from_status: string | null;
  to_status: string | null;
  action_kind: ActionKind;
  action_value: Record<string, unknown>;
  enabled: boolean;
}

/** What one firing actually did. */
export interface AutomationRun {
  pid: string;
  automation_pid: string;
  subject_kind: string;
  subject_pid: string;
  outcome: "applied" | "skipped" | "failed";
  detail: Record<string, unknown>;
  created_at: string;
}

/** A deadline configured once and held until it comes due. */
export interface ScheduledAction {
  pid: string;
  subject_kind: string;
  subject_pid: string;
  action_kind: "notify" | "expire_review";
  payload: Record<string, unknown>;
  due_at: string;
  status: "pending" | "fired" | "cancelled";
  source_automation_pid: string | null;
  created_by: string | null;
  fired_at: string | null;
  outcome: string | null;
}

/** What one sweep did. */
export interface SweepResult {
  fired: number;
  skipped_already_claimed: number;
  capped: boolean;
  cap: number;
  as_of: string;
}

// ---- Smart Score ----

/** One component's contribution to a Smart Score. */
export interface ScoreComponent {
  name: string;
  /** Renormalised share of the score, 0–1. */
  weight: number;
  /** Normalised component value, 0–1. */
  raw: number;
  /** Points of the final score. */
  contribution: number;
}

/**
 * A plan's Smart Score. `score` is `null` when there is no evidence at
 * all — deliberately not zero, which would read as "low value".
 * `missing` names the components with no evidence and `coverage` is the
 * share of configured weight that was actually backed by data.
 */
export interface SmartScore {
  score: number | null;
  band: "high" | "medium" | "low" | "unscored";
  components: ScoreComponent[];
  missing: string[];
  coverage: number;
}

/** One plan's line in the ranked queue. */
export interface RankedPlan {
  pid: string;
  name: string;
  kind: string | null;
  stage: string | null;
  score: number | null;
  band: SmartScore["band"];
  coverage: number;
  missing_evidence: string[];
}

/** The ranked prioritisation queue. */
export interface Prioritisation {
  plans: RankedPlan[];
  scored_of: number;
  as_of: string;
}

/** One plan's score plus its identity. */
export interface PlanSmartScore {
  pid: string;
  name: string;
  smart_score: SmartScore;
  as_of: string;
}

// ---- bird's-eye lifecycle ----

/** One phase of the challenge funnel. */
export interface PhaseRow {
  phase: string;
  live: number;
  stalled: number;
}

/** The funnel across every phase (empty phases are still reported). */
export interface LifecycleFunnel {
  phases: PhaseRow[];
  unknown_phase: number;
  stall_days: number;
  as_of: string;
}

/** One readiness check, with the count behind it. */
export interface ReadinessCheck {
  name: string;
  ok: boolean;
  detail: string;
}

/** Where a plan sits and what stands between it and the next gate. */
export interface PlanLifecycle {
  pid: string;
  name: string;
  phase: string;
  readiness: {
    stage: string | null;
    next_gate: string | null;
    ready: boolean;
    checks: ReadinessCheck[];
    blockers: string[];
  };
  review_consensus: Consensus;
  as_of: string;
}

/** Typed wrappers over the capability endpoints. */
export class CapabilityClient {
  constructor(private readonly http: ApiClient) {}

  /** Wire an {@link ApiClient} at {@link API_BASE_URL}. */
  static withFetch(fetchFn?: typeof fetch): CapabilityClient {
    return new CapabilityClient(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  // ---- collaborative review ----
  listReviews(
    params: {
      subject_kind?: ReviewSubjectKind;
      subject_pid?: string;
      reviewer?: string;
      status?: ReviewStatus;
    } = {},
  ): Promise<Review[]> {
    return this.http.get(`/api/reviews${query(params)}`);
  }
  /**
   * As {@link listReviews}, but paginated (`?limit=&offset=`) and
   * returning the service's `X-Total-Count`/`X-Limit`/`X-Offset`
   * alongside the page — see {@link ApiClient.getPage}.
   */
  listReviewsPage(
    params: {
      subject_kind?: ReviewSubjectKind;
      subject_pid?: string;
      reviewer?: string;
      status?: ReviewStatus;
    } = {},
    page: PageRequest = {},
  ): Promise<Page<Review>> {
    return this.http.getPage(`/api/reviews${query(params)}`, page);
  }
  invite(body: {
    subject_kind: ReviewSubjectKind;
    subject_pid: string;
    reviewer_ref: string;
    reviewer_scope?: ReviewerScope;
    expertise?: string;
    due_on?: string;
  }): Promise<Review> {
    return this.http.post("/api/reviews", { body });
  }
  respond(pid: string, response: "accept" | "decline"): Promise<Review> {
    return this.http.post(`/api/reviews/${pid}/respond`, {
      body: { response },
    });
  }
  submitVerdict(
    pid: string,
    body: { score?: number; recommendation: Recommendation; comment?: string },
  ): Promise<Review> {
    return this.http.post(`/api/reviews/${pid}/submit`, { body });
  }
  withdraw(pid: string): Promise<void> {
    return this.http.delete(`/api/reviews/${pid}`);
  }
  consensus(
    subject_kind: ReviewSubjectKind,
    subject_pid: string,
  ): Promise<Consensus> {
    return this.http.get(
      `/api/reviews/consensus${query({ subject_kind, subject_pid })}`,
    );
  }

  // ---- assignees ----
  workload(
    plan?: string,
  ): Promise<{ assignees: AssigneeLoad[]; as_of: string }> {
    return this.http.get(`/api/assignees/workload${query({ plan })}`);
  }
  assign(
    planPid: string,
    taskPid: string,
    assignee_ref: string | null,
  ): Promise<unknown> {
    return this.http.post(`/api/plans/${planPid}/tasks/${taskPid}/assign`, {
      body: { assignee_ref },
    });
  }

  // ---- notifications ----
  inbox(recipient: string, unread = false): Promise<Notification[]> {
    return this.http.get(
      `/api/notifications${query({ recipient, unread: unread || undefined })}`,
    );
  }
  /**
   * As {@link inbox}, but paginated (`?limit=&offset=`) — see
   * {@link ApiClient.getPage}. No route calls this yet (nor {@link inbox}
   * itself — the notifications inbox has no consuming UI; see
   * `spec/index.md` §13), but the API-client layer carries the same
   * pagination contract as the other four sub-resource lists for when
   * one is built.
   */
  inboxPage(
    recipient: string,
    unread = false,
    page: PageRequest = {},
  ): Promise<Page<Notification>> {
    return this.http.getPage(
      `/api/notifications${query({ recipient, unread: unread || undefined })}`,
      page,
    );
  }
  markRead(pid: string): Promise<Notification> {
    return this.http.post(`/api/notifications/${pid}/read`, { body: {} });
  }

  // ---- workflow automation ----
  listAutomations(
    params: {
      plan?: string;
      trigger?: TriggerKind;
      enabled?: boolean;
    } = {},
  ): Promise<Automation[]> {
    return this.http.get(`/api/automations${query(params)}`);
  }
  /**
   * As {@link listAutomations}, but paginated (`?limit=&offset=`) — see
   * {@link ApiClient.getPage}.
   */
  listAutomationsPage(
    params: {
      plan?: string;
      trigger?: TriggerKind;
      enabled?: boolean;
    } = {},
    page: PageRequest = {},
  ): Promise<Page<Automation>> {
    return this.http.getPage(`/api/automations${query(params)}`, page);
  }
  createAutomation(body: {
    plan_pid?: string;
    name: string;
    trigger_kind: TriggerKind;
    from_status?: string;
    to_status?: string;
    action_kind: ActionKind;
    action_value: Record<string, unknown>;
  }): Promise<Automation> {
    return this.http.post("/api/automations", { body });
  }
  setAutomationEnabled(pid: string, enabled: boolean): Promise<Automation> {
    return this.http.post(
      `/api/automations/${pid}/${enabled ? "enable" : "disable"}`,
      { body: {} },
    );
  }
  deleteAutomation(pid: string): Promise<void> {
    return this.http.delete(`/api/automations/${pid}`);
  }
  runs(
    params: { automation?: string; subject?: string; outcome?: string } = {},
  ): Promise<AutomationRun[]> {
    return this.http.get(`/api/automations/runs${query(params)}`);
  }
  /**
   * As {@link runs}, but paginated (`?limit=&offset=`) — see
   * {@link ApiClient.getPage}.
   */
  runsPage(
    params: { automation?: string; subject?: string; outcome?: string } = {},
    page: PageRequest = {},
  ): Promise<Page<AutomationRun>> {
    return this.http.getPage(`/api/automations/runs${query(params)}`, page);
  }

  // ---- set and forget ----
  listScheduled(
    params: {
      status?: string;
      subject?: string;
      overdue?: boolean;
    } = {},
  ): Promise<ScheduledAction[]> {
    return this.http.get(`/api/scheduled-actions${query(params)}`);
  }
  /**
   * As {@link listScheduled}, but paginated (`?limit=&offset=`) — see
   * {@link ApiClient.getPage}. The deadline queue stays soonest-first
   * (`due_at` ascending) under paging: a page is a contiguous slice of
   * that order, never a reshuffled one.
   */
  listScheduledPage(
    params: {
      status?: string;
      subject?: string;
      overdue?: boolean;
    } = {},
    page: PageRequest = {},
  ): Promise<Page<ScheduledAction>> {
    return this.http.getPage(`/api/scheduled-actions${query(params)}`, page);
  }
  schedule(body: {
    subject_kind: string;
    subject_pid: string;
    action_kind: "notify" | "expire_review";
    in_days: number;
    recipient_ref?: string;
    message?: string;
  }): Promise<ScheduledAction> {
    return this.http.post("/api/scheduled-actions", { body });
  }
  cancelScheduled(pid: string): Promise<void> {
    return this.http.delete(`/api/scheduled-actions/${pid}`);
  }
  sweep(): Promise<SweepResult> {
    return this.http.post("/api/scheduled-actions/sweep", { body: {} });
  }

  // ---- prioritisation + lifecycle ----
  prioritisation(
    params: { limit?: number; band?: string } = {},
  ): Promise<Prioritisation> {
    return this.http.get(`/api/prioritisation${query(params)}`);
  }
  smartScore(planPid: string): Promise<PlanSmartScore> {
    return this.http.get(`/api/plans/${planPid}/smart-score`);
  }
  lifecycle(): Promise<LifecycleFunnel> {
    return this.http.get("/api/lifecycle");
  }
  planLifecycle(planPid: string): Promise<PlanLifecycle> {
    return this.http.get(`/api/plans/${planPid}/lifecycle`);
  }
}

/** Build a query string from the set parameters (`?a=1&b=2`, or ``). */
function query(
  params: Record<string, string | number | boolean | undefined>,
): string {
  const pairs = Object.entries(params).filter(
    ([, value]) => value !== undefined && value !== "",
  );
  if (pairs.length === 0) return "";
  const search = new URLSearchParams(
    pairs.map(([key, value]) => [key, String(value)]),
  );
  return `?${search.toString()}`;
}
