// Time-based-analysis client: typed wrappers over the care-pathway
// service's TBA endpoints, and the single source of TBA paths for the
// UI.
//
// Time-based analysis measures a patient journey against elapsed
// calendar time: of the days a subject spent on a pathway, how many
// were care? Published NHS journeys measure 8–14%. See the entity
// spec's cross-cutting `time-based-analysis.md`.
//
// Reads are GETs. The recording endpoints (segments, the clock) are
// wrapped too, because unlike the portfolio sibling — where a board
// move already produces the data — a clinical journey has to be mapped
// by hand, so the UI needs a way to record one.

import { API_BASE_URL } from "$lib/config";
import { ApiClient } from "./client";

// ---- vocabularies (mirroring the service's closed sets) ----

/** Journey stages. */
export const STAGES = [
  "referral",
  "triage",
  "diagnostics",
  "treatment",
  "follow_up",
  "discharge",
  "other",
] as const;

/** The value-stream-mapping categories. */
export const CATEGORIES = [
  "value_adding",
  "necessary_non_value_adding",
  "unnecessary_non_value_adding",
] as const;

/** The eight VSM wastes. */
export const WASTES = [
  "waiting",
  "transportation",
  "motion",
  "over_processing",
  "defects",
  "inventory",
  "overproduction",
  "underutilised_people",
] as const;

export type Stage = (typeof STAGES)[number];
export type Category = (typeof CATEGORIES)[number];
export type Waste = (typeof WASTES)[number];

// ---- wire shapes ----

/** A ratio reported with the two figures it came from. */
export interface Ratio {
  /** The ratio in [0, 1], or `null` when undefined. */
  value: number | null;
  numerator_ms: number;
  denominator_ms: number;
}

/** The instance clock: the window every ratio is measured against. */
export interface Clock {
  start_ms: number;
  stop_ms: number;
  /** `clock_start_at` (measured) or `enrolled_on` (day resolution). */
  start_source: string;
  /** `clock_stop_at`, `closed_on`, or `as_of` while it runs. */
  stop_source: string;
  running: boolean;
}

/** One bucket of the clock partition. The four sum to the lead time. */
export interface CategoryShare {
  /** A {@link Category}, or `unrecorded`. */
  category: string;
  ms: number;
  days: number;
  share: number | null;
}

/** Per-stage time. Stages may overlap, so shares need not sum to 1. */
export interface StageShare {
  stage: string;
  ms: number;
  non_value_adding_ms: number;
  share: number | null;
  segments: number;
}

/** Per-waste-type time, over non-value-adding segments only. */
export interface WasteShare {
  waste: string;
  ms: number;
  segments: number;
}

/** A stretch of clock covered by no segment. */
export interface Gap {
  start_ms: number;
  end_ms: number;
  duration_ms: number;
  days: number;
  /** The segment it follows, if any. */
  after: string | null;
  /** The segment it precedes, if any. */
  before: string | null;
  /** The stage it is charged to — what the patient was waiting to reach. */
  stage: string | null;
  /** Whether it sits at a change of clinician or location. */
  at_handoff: boolean;
}

/** Handoff counts and their time cost. */
export interface Handoffs {
  actor_changes: number;
  location_changes: number;
  total: number;
  distinct_actors: number;
  distinct_locations: number;
  gap_ms_at_handoffs: number;
}

/** The per-instance analysis. */
export interface InstanceAnalysis {
  clock: Clock;
  lead_time_ms: number;
  lead_time_days: number;
  /** Union of value-adding segments (VT). */
  value_time_ms: number;
  /** Union of value-adding + necessary segments (PT). */
  process_time_ms: number;
  waste_time_ms: number;
  /** The raw sum; may exceed lead time when care was concurrent. */
  touch_time_ms: number;
  wait_time_ms: number;
  unrecorded_ms: number;
  /** The Barker headline: value time over elapsed calendar time. */
  value_adding_ratio: Ratio;
  activity_ratio: Ratio;
  /** How much of the journey was mapped at all. */
  coverage_ratio: Ratio;
  /** `unmapped` | `partial` | `mapped`. */
  confidence: string;
  segments: number;
  by_category: CategoryShare[];
  by_stage: StageShare[];
  by_waste: WasteShare[];
  handoffs: Handoffs;
  /** Longest first. */
  gaps: Gap[];
  /** Why the analysis is null, when the clock is unmeasurable. */
  reason: string | null;
}

/** One entry of the timeline wall: a recorded segment or a gap. */
export interface WallEntry {
  kind: "segment" | "gap";
  pid?: string;
  label: string;
  stage: string | null;
  category?: string;
  waste?: string | null;
  started_at?: string;
  ended_at?: string | null;
  open?: boolean;
  actor_ref?: string | null;
  location_ref?: string | null;
  duration_ms: number;
  duration_days: number;
  at_handoff?: boolean;
}

/** `GET /api/instances/{pid}/timeline` */
export interface Timeline {
  as_of: string;
  instance: { pid: string; status: string };
  clock: Clock;
  note: string;
  totals: {
    lead_time_ms: number;
    lead_time_days: number;
    value_adding_ratio: Ratio;
    coverage_ratio: Ratio;
    confidence: string;
  };
  wall: WallEntry[];
}

/** `GET /api/instances/{pid}/time-analysis` */
export interface InstanceTimeAnalysis {
  as_of: string;
  instance: { pid: string; status: string };
  note: string;
  analysis: InstanceAnalysis;
}

/** A right-skewed duration distribution. */
export interface Distribution {
  n: number;
  min_ms: number;
  p50_ms: number;
  p75_ms: number;
  p90_ms: number;
  p95_ms: number;
  max_ms: number;
  /** Reported, but skew-sensitive and describing no actual patient. */
  mean_ms: number;
  p50_days: number;
  p90_days: number;
  method: string;
}

/** The cohort rollup. */
export interface CohortAnalysis {
  instances: number;
  /** `null` when suppressed for a small cohort. */
  lead_time: Distribution | null;
  aggregate_value_adding_ratio: Ratio;
  median_value_adding_ratio: number | null;
  /** `concentrated` when the waste sits in a minority of journeys. */
  waste_shape: "concentrated" | "uniform" | "insufficient_data";
  coverage_ratio: Ratio;
  by_stage: StageShare[];
  by_waste: WasteShare[];
}

/** How a cohort scored against an access standard. */
export interface Compliance {
  standard: string;
  threshold_ms: number;
  threshold_days: number;
  within: number;
  breached: number;
  achieved_ratio: number | null;
  target_ratio: number | null;
  target_met: boolean | null;
  as_of: string | null;
}

/** `GET /api/care-pathways/{pathway}/time-analysis` */
export interface CohortTimeAnalysis {
  as_of: string;
  pathway: { pid: string; name: string };
  note: string;
  /** True when the cohort is too small to disclose percentile detail. */
  suppressed: boolean;
  suppression_note: string | null;
  cohort: CohortAnalysis;
  compliance: Compliance | null;
}

/** One disclosed constraint finding. */
export interface Finding {
  rule: string;
  subject: string;
  detail: string;
  recoverable_ms: number;
  recoverable_days: number;
}

/** `GET /api/care-pathways/{pathway}/constraints` */
export interface Constraints {
  as_of: string;
  pathway: { pid: string; name: string };
  note: string;
  instances: number;
  findings: Finding[];
}

/** A named access standard: a threshold on lead time plus its target. */
export interface Standard {
  id: string;
  label: string;
  threshold_ms: number;
  target_ratio: number;
  authority: string;
  /** When the entry was last checked — targets move. */
  as_of: string;
  note: string;
}

/** `GET /api/instances/time-standards` */
export interface Standards {
  note: string;
  standards: Standard[];
  vocabularies: { stages: string[]; categories: string[]; wastes: string[] };
}

/** `GET /api/instances/flow` */
export interface Flow {
  as_of: string;
  window_since: string;
  note: string;
  instances_considered: number;
  flow: {
    window_days: number;
    arrivals: number;
    closures: number;
    arrival_rate_per_day: number | null;
    service_rate_per_day: number | null;
    utilisation: number | null;
    utilisation_reason: string | null;
    work_in_progress: number;
    implied_lead_time_days: number | null;
    observed_p50_lead_time_days: number | null;
    interpretation:
      | "backlog_growing"
      | "steady_state"
      | "queue_draining"
      | "insufficient_data";
    detail: string;
  };
}

/** A recorded journey segment, as stored. */
export interface Segment {
  pid: string;
  instance_pid: string;
  label: string;
  stage: string;
  category: string;
  waste: string | null;
  started_at: string;
  ended_at: string | null;
  actor_ref: string | null;
  location_ref: string | null;
  note: string | null;
  position: number;
}

/** The body for recording a segment. */
export interface SegmentPayload {
  label: string;
  stage: Stage;
  category: Category;
  /** Refused on a value-adding segment; required on an unnecessary one. */
  waste?: Waste | null;
  started_at: string;
  /** Omit to open a running segment; only one may be open per instance. */
  ended_at?: string | null;
  actor_ref?: string | null;
  location_ref?: string | null;
  note?: string | null;
}

// ---- client ----

/** Typed access to the care-pathway time-based-analysis endpoints. */
export class TbaRepository {
  constructor(private readonly http: ApiClient) {}

  /** Wire an {@link ApiClient} at {@link API_BASE_URL}. */
  static withFetch(fetchFn?: typeof fetch): TbaRepository {
    return new TbaRepository(
      new ApiClient({ baseUrl: API_BASE_URL, fetch: fetchFn }),
    );
  }

  /** `GET /api/instances/{pid}/timeline` — the segment/gap wall. */
  timeline(pid: string): Promise<Timeline> {
    return this.http.get<Timeline>(
      `/api/instances/${encodeURIComponent(pid)}/timeline`,
    );
  }

  /** `GET /api/instances/{pid}/time-analysis`. */
  instanceAnalysis(pid: string): Promise<InstanceTimeAnalysis> {
    return this.http.get<InstanceTimeAnalysis>(
      `/api/instances/${encodeURIComponent(pid)}/time-analysis`,
    );
  }

  /** `GET /api/instances/{pid}/segments`. */
  segments(pid: string): Promise<Segment[]> {
    return this.http.get<Segment[]>(
      `/api/instances/${encodeURIComponent(pid)}/segments`,
    );
  }

  /** `POST /api/instances/{pid}/segments` — record a mapped segment. */
  recordSegment(pid: string, body: SegmentPayload): Promise<Segment> {
    return this.http.post<Segment>(
      `/api/instances/${encodeURIComponent(pid)}/segments`,
      { body },
    );
  }

  /** `POST /api/instances/{pid}/clock` — set the clock start or stop. */
  setClock(
    pid: string,
    event: "start" | "stop",
    at?: string,
  ): Promise<unknown> {
    return this.http.post<unknown>(
      `/api/instances/${encodeURIComponent(pid)}/clock`,
      { body: at === undefined ? { event } : { event, at } },
    );
  }

  /** `GET /api/care-pathways/{pathway}/time-analysis`. */
  cohort(
    pathwayPid: string,
    options: { standard?: string; targetDays?: number; status?: string } = {},
  ): Promise<CohortTimeAnalysis> {
    const query = new URLSearchParams();
    if (options.standard) query.set("standard", options.standard);
    if (options.targetDays !== undefined)
      query.set("target_days", String(options.targetDays));
    if (options.status) query.set("status", options.status);
    const suffix = query.size > 0 ? `?${query}` : "";
    return this.http.get<CohortTimeAnalysis>(
      `/api/care-pathways/${encodeURIComponent(pathwayPid)}/time-analysis${suffix}`,
    );
  }

  /** `GET /api/care-pathways/{pathway}/constraints`. */
  constraints(pathwayPid: string, status?: string): Promise<Constraints> {
    return this.http.get<Constraints>(
      `/api/care-pathways/${encodeURIComponent(pathwayPid)}/constraints${status ? `?status=${status}` : ""}`,
    );
  }

  /** `GET /api/instances/time-standards` — the access-standard catalogue. */
  standards(): Promise<Standards> {
    return this.http.get<Standards>("/api/instances/time-standards");
  }

  /** `GET /api/instances/flow` — Little's Law over a window. */
  flow(windowDays?: number, pathway?: string): Promise<Flow> {
    const query = new URLSearchParams();
    if (windowDays !== undefined) query.set("window_days", String(windowDays));
    if (pathway) query.set("pathway", pathway);
    const suffix = query.size > 0 ? `?${query}` : "";
    return this.http.get<Flow>(`/api/instances/flow${suffix}`);
  }
}

// ---- presentation helpers (pure; unit-tested) ----

/**
 * A ratio as a percentage, or an em-dash when the service returned
 * `null`.
 *
 * A null ratio means undefined, not zero. An unmeasurable clock has no
 * value-adding ratio, and rendering that as "0%" would read as a
 * catastrophically wasteful journey rather than as "we cannot tell".
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
 * What a coverage-derived confidence label means for the reader.
 *
 * This is the guard against the method's worst misreading: a journey
 * nobody mapped reports a value-adding ratio near zero, which looks
 * identical to a catastrophically wasteful one. The service labels it
 * `unmapped`; this turns that into a sentence.
 */
export function confidenceNote(confidence: string): string {
  switch (confidence) {
    case "mapped":
      return "Most of this journey is accounted for, so the non-value-adding figure is evidenced rather than inferred.";
    case "partial":
      return "Part of this journey is unmapped, so the ratio is a floor: the real value-adding share cannot be higher, and may be lower.";
    default:
      return "This journey is essentially unmapped. The ratio is not a measurement — record the segments before reading anything into it.";
  }
}

/**
 * How a value-adding ratio reads against the published NHS benchmark.
 *
 * Barker's tracked journeys measure 8–14%, so a single-digit figure is
 * the norm the method predicts, not an outlier — and an implausibly
 * high one almost always means an unmapped journey rather than an
 * efficient one.
 */
export function valueAddingBand(
  ratio: number | null,
  confidence: string,
): "unknown" | "typical" | "better" | "suspicious" {
  if (ratio === null || !Number.isFinite(ratio)) return "unknown";
  if (confidence === "unmapped") return "unknown";
  if (ratio > 0.5) return "suspicious";
  if (ratio > 0.14) return "better";
  return "typical";
}

/** The human label for a Little's-Law interpretation. */
export function interpretationLabel(
  interpretation: Flow["flow"]["interpretation"],
): string {
  switch (interpretation) {
    case "backlog_growing":
      return "The backlog is growing";
    case "steady_state":
      return "Steady state";
    case "queue_draining":
      return "Queue draining";
    default:
      return "Not enough data";
  }
}
