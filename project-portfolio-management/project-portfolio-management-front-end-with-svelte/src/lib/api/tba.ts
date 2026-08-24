// Time-based-analysis client: typed wrappers over the service's TBA
// endpoints, and the single source of TBA paths for the UI.
//
// Every endpoint here is a GET. Transitions are written by the existing
// task create / board-move calls (see PpmClient.moveTask), so moving a
// card is what produces the measurement — there is nothing extra for an
// operator to keep up to date, and nothing here can edit the log.
//
// See the entity spec's cross-cutting `time-based-analysis.md`.
//
// English-first, like the other PPM views.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";

// ---- wire shapes (mirroring the service responses) ----

/** A ratio reported with the two figures it came from, so a consumer can re-aggregate without trusting our rounding. */
export interface Ratio {
  /** The ratio in [0, 1], or `null` when undefined (a non-positive denominator). */
  value: number | null;
  /** The numerator, milliseconds. */
  numerator_ms: number;
  /** The denominator, milliseconds. */
  denominator_ms: number;
}

/** Time spent in one board status. These partition the lead time. */
export interface StatusShare {
  status: string;
  /** The VSM category it was classified as, under the map in force. */
  category: string;
  /** The VSM waste type, where the status represents one. */
  waste: string | null;
  ms: number;
  days: number;
  /** Share of lead time. */
  share: number | null;
  /** How many times the item entered this status. */
  entries: number;
}

/** Time in one VSM category. These partition the lead time. */
export interface CategoryShare {
  category: string;
  ms: number;
  days: number;
  share: number | null;
}

/** A long-tailed duration distribution, reported by percentile. */
export interface Distribution {
  n: number;
  min_ms: number;
  p50_ms: number;
  p75_ms: number;
  p85_ms: number;
  p95_ms: number;
  max_ms: number;
  /** Reported, but skew-sensitive and describing no actual item. */
  mean_ms: number;
  p50_days: number;
  p85_days: number;
  method: string;
}

/** The per-task analysis. */
export interface TaskAnalysis {
  /** Created → finished. What the requester waits. */
  lead_time_ms: number;
  lead_time_days: number;
  /** First started → finished. What the team controls. `null` until it starts. */
  cycle_time_ms: number | null;
  cycle_time_days: number | null;
  cycle_time_reason: string | null;
  work_time_ms: number;
  process_time_ms: number;
  wait_time_ms: number;
  blocked_time_ms: number;
  /** Time in `todo` — the backlog dwell. */
  queue_time_ms: number;
  /** Work over cycle time. The headline 5–15% ratio. */
  flow_efficiency: Ratio;
  by_status: StatusShare[];
  by_category: CategoryShare[];
  transitions: number;
  /** How many transitions were synthesised by the migration rather than observed. */
  backfilled: number;
  rework_count: number;
  first_pass: boolean;
  distinct_assignees: number;
  handoffs: number;
  finished: boolean;
  /** For an open item: how long since it started. The actionable one. */
  age_ms: number | null;
  age_days: number | null;
}

/** The status → VSM category map a figure was computed with. */
export interface Classification {
  classes: Record<string, string>;
  overridden: boolean;
  source: string;
}

/** "p% of items finish within N days", derived from the plan's own history. */
export interface ServiceLevelExpectation {
  percentile: number;
  within_ms: number | null;
  within_days: number | null;
  /** Finished items it was computed from. */
  sample: number;
  /** Why it is null — below the minimum sample, it is refused rather than computed from noise. */
  reason: string | null;
  target_days: number | null;
  target_achieved_ratio: number | null;
  target_met: boolean | null;
}

/** The plan-level rollup. */
export interface PlanAnalysis {
  tasks: number;
  finished: number;
  work_in_progress: number;
  not_started: number;
  cycle_time: Distribution | null;
  /** Always present beside the cycle time, so the flattering number cannot travel alone. */
  lead_time: Distribution | null;
  aggregate_flow_efficiency: Ratio;
  median_flow_efficiency: number | null;
  /** `concentrated` when aggregate and median diverge — the waste sits in a minority of items. */
  waste_shape: "concentrated" | "uniform" | "insufficient_data";
  /** The share of finished items that never moved backwards. */
  rolled_first_pass_yield: number | null;
  rework_count: number;
  by_status: StatusShare[];
  backfilled_ratio: number | null;
}

/** One disclosed constraint finding, ordered by time recoverable. */
export interface Finding {
  rule: string;
  subject: string;
  detail: string;
  recoverable_ms: number;
  recoverable_days: number;
}

/** One open item scored against the service level expectation. */
export interface AgingRow {
  task: {
    pid: string;
    title: string;
    status: string;
    assignee_ref: string | null;
  };
  status: string;
  aging: {
    age_ms: number;
    age_days: number;
    past_sle: boolean;
    /** Age as a fraction of the expectation; `null` when there is no expectation yet. */
    sle_ratio: number | null;
  };
  blocked_time_ms: number;
  rework_count: number;
}

/** Queueing-theory flow over a window. */
export interface Flow {
  window_days: number;
  arrivals: number;
  completions: number;
  arrival_rate_per_day: number | null;
  throughput_per_day: number | null;
  utilisation: number | null;
  utilisation_reason: string | null;
  work_in_progress: number;
  implied_cycle_time_days: number | null;
  observed_p50_cycle_time_days: number | null;
  interpretation:
    | "wip_growing"
    | "steady_state"
    | "queue_draining"
    | "insufficient_data";
  detail: string;
}

/** One column's occupancy against its configured WIP cap. */
export interface ColumnOccupancy {
  status: string;
  count: number;
  limit: number | null;
  over_limit: boolean;
}

/** One sample of the board: how many tasks stood in each status at an instant. */
export interface FlowSample {
  at_ms: number;
  /** Status → count. Every band is present, including at zero. */
  counts: Record<string, number>;
  total: number;
  done: number;
  work_in_progress: number;
}

// ---- response envelopes ----

/** `GET /api/plans/{pid}/time-analysis` */
export interface PlanTimeAnalysis {
  as_of: string;
  plan: { pid: string; name: string };
  note: string;
  classification: Classification;
  service_level_expectation: ServiceLevelExpectation;
  plan_analysis: PlanAnalysis;
}

/** `GET /api/plans/{pid}/tasks/{t}/time-analysis` */
export interface TaskTimeAnalysis {
  as_of: string;
  task: {
    pid: string;
    title: string;
    status: string;
    assignee_ref: string | null;
  };
  note: string;
  classification: Classification;
  analysis: TaskAnalysis;
}

/** `GET /api/plans/{pid}/constraints` */
export interface Constraints {
  as_of: string;
  plan: { pid: string; name: string };
  note: string;
  classification: Classification;
  tasks: number;
  findings: Finding[];
}

/** `GET /api/plans/{pid}/aging-wip` */
export interface AgingWip {
  as_of: string;
  plan: { pid: string; name: string };
  note: string;
  classification: Classification;
  service_level_expectation: ServiceLevelExpectation;
  aging: AgingRow[];
}

/** `GET /api/plans/{pid}/flow` */
export interface PlanFlow {
  as_of: string;
  plan: { pid: string; name: string };
  window_since: string;
  note: string;
  flow: Flow;
  columns: ColumnOccupancy[];
}

/** `GET /api/plans/{pid}/cumulative-flow` */
export interface CumulativeFlow {
  as_of: string;
  plan: { pid: string; name: string };
  days: number;
  note: string;
  classification: Classification;
  samples: FlowSample[];
}

/** One recorded status change. */
export interface TransitionRow {
  pid: string;
  task_pid: string;
  from_status: string | null;
  to_status: string;
  at: string;
  actor_ref: string | null;
  assignee_ref: string | null;
  /** Synthesised by the migration rather than observed. */
  backfilled: boolean;
}

/** `GET /api/plans/{pid}/tasks/{t}/transitions` */
export interface TransitionLog {
  task: {
    pid: string;
    title: string;
    status: string;
    assignee_ref: string | null;
  };
  note: string;
  transitions: TransitionRow[];
}

/** `GET /api/flow-classes` */
export interface FlowClasses {
  note: string;
  classification: Classification;
  default: Record<string, string>;
  categories: string[];
  board_order: string[];
  finished_status: string;
  backlog_status: string;
  blocked_status: string;
  minimum_sle_sample: number;
}

// ---- client ----

/**
 * Typed access to the time-based-analysis endpoints. Read-only by
 * design: the transition log is append-only and is written by the task
 * endpoints, so there is deliberately no write method here.
 */
export class TbaClient {
  constructor(private readonly http: ApiClient) {}

  /** Wire an {@link ApiClient} at {@link API_BASE_URL}. */
  static withFetch(fetchFn?: typeof fetch): TbaClient {
    return new TbaClient(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  planTimeAnalysis(
    pid: string,
    options: {
      slePercentile?: number;
      targetDays?: number;
      sprint?: string;
    } = {},
  ): Promise<PlanTimeAnalysis> {
    const query = new URLSearchParams();
    if (options.slePercentile !== undefined)
      query.set("sle_percentile", String(options.slePercentile));
    if (options.targetDays !== undefined)
      query.set("target_days", String(options.targetDays));
    if (options.sprint) query.set("sprint", options.sprint);
    const suffix = query.size > 0 ? `?${query}` : "";
    return this.http.get(`/api/plans/${pid}/time-analysis${suffix}`);
  }

  taskTimeAnalysis(pid: string, taskPid: string): Promise<TaskTimeAnalysis> {
    return this.http.get(`/api/plans/${pid}/tasks/${taskPid}/time-analysis`);
  }

  transitions(pid: string, taskPid: string): Promise<TransitionLog> {
    return this.http.get(`/api/plans/${pid}/tasks/${taskPid}/transitions`);
  }

  constraints(pid: string, sprint?: string): Promise<Constraints> {
    return this.http.get(
      `/api/plans/${pid}/constraints${sprint ? `?sprint=${sprint}` : ""}`,
    );
  }

  agingWip(pid: string, slePercentile?: number): Promise<AgingWip> {
    return this.http.get(
      `/api/plans/${pid}/aging-wip${slePercentile === undefined ? "" : `?sle_percentile=${slePercentile}`}`,
    );
  }

  flow(pid: string, windowDays?: number): Promise<PlanFlow> {
    return this.http.get(
      `/api/plans/${pid}/flow${windowDays === undefined ? "" : `?window_days=${windowDays}`}`,
    );
  }

  cumulativeFlow(pid: string, days?: number): Promise<CumulativeFlow> {
    return this.http.get(
      `/api/plans/${pid}/cumulative-flow${days === undefined ? "" : `?days=${days}`}`,
    );
  }

  flowClasses(): Promise<FlowClasses> {
    return this.http.get("/api/flow-classes");
  }
}

// ---- presentation helpers (pure; unit-tested) ----

/**
 * A ratio as a percentage string, or an em-dash when the service
 * returned `null`.
 *
 * A null ratio means undefined, not zero — an item that never started
 * has no flow efficiency, and rendering that as "0%" would read as
 * catastrophically inefficient rather than as "not applicable yet".
 */
export function percent(value: number | null | undefined, digits = 0): string {
  if (value === null || value === undefined || !Number.isFinite(value))
    return "—";
  return `${(value * 100).toFixed(digits)}%`;
}

/** A duration in days, or an em-dash when there is none. */
export function days(value: number | null | undefined, digits = 1): string {
  if (value === null || value === undefined || !Number.isFinite(value))
    return "—";
  return `${value.toFixed(digits)}d`;
}

/** Milliseconds as a days string. */
export function msAsDays(ms: number | null | undefined, digits = 1): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return "—";
  return days(ms / 86_400_000, digits);
}

/**
 * How a flow-efficiency figure reads against the field's own
 * benchmark. Knowledge-work flow efficiency typically measures 5–15%,
 * so a low number is **normal, not alarming** — and a very high one
 * usually means the board is not being kept up to date rather than that
 * the team is exceptional.
 */
export function flowEfficiencyBand(
  value: number | null,
): "unknown" | "typical" | "strong" | "suspicious" {
  if (value === null || !Number.isFinite(value)) return "unknown";
  if (value > 0.6) return "suspicious";
  if (value >= 0.15) return "strong";
  return "typical";
}

/** The human label for a Little's-Law interpretation. */
export function interpretationLabel(
  interpretation: Flow["interpretation"],
): string {
  switch (interpretation) {
    case "wip_growing":
      return "Work in progress is growing";
    case "steady_state":
      return "Steady state";
    case "queue_draining":
      return "Queue draining";
    default:
      return "Not enough data";
  }
}
