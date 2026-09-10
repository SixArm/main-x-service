//! **Event-log and journey-feature export codecs** (spec `13-tasks.md`
//! T-14a) — pure row-shaping over the instance layer, so a bupaR / PM4Py
//! / ehrapy user can consume a cohort without a bespoke query.
//!
//! No I/O and no clock read: every row is built from values the caller
//! already loaded, exactly like [`crate::tba`], which this module is a
//! sibling of and depends on (`journey_feature_row` builds directly from
//! a [`crate::tba::InstanceAnalysis`]). That makes both codecs
//! unit-testable without a database, which is where the one invariant
//! that matters most is pinned (see `no_row_ever_carries_a_person_urn`
//! below): **a case is named by the instance `pid`, never
//! `subject_ref`, and a resource is a team *role*, never an actor
//! `person:`/`worker:` URN.** Both are bupaR/PM4Py event-log
//! conventions (`case_id`, `activity_id`, `lifecycle_id`, `timestamp`,
//! `resource_id`) that this module's column names deliberately echo.
//!
//! Three sibling tasks this module's acceptance criterion cites were not
//! yet built when this module landed — T-14c (journey variants), T-14d
//! (anchors/delays), T-14i (conformance) — so [`JourneyFeatureRow`]
//! reserved their columns from day one, `None` with a documented reason
//! rather than silently omitting them or blocking this task on theirs
//! (see each field's doc comment). T-14d has since landed and is wired:
//! `anchors_delays` is computed straight from the same
//! `tba::InstanceAnalysis` this row already builds from — no per-row
//! cohort context needed, unlike `variant` (T-14c, landed but still
//! unwired — a variant string needs the whole cohort's pipeline, not
//! one instance in isolation) and `conformance` (T-14i, not yet built).
//!
//! **Directly-follows process map** (spec T-14b) — [`ActivityStep`],
//! [`build_process_map`] — lives here too, alongside the event-log
//! codec rather than in `src/tba.rs`: both derive a shape from the same
//! segment/step inputs, and the family doc is explicit that a sequence
//! analysis (T-14b, T-14c journey variants, T-14i conformance) is not
//! an elapsed-time one and sits *beside* TBA rather than inside it
//! (`spec/time-based-analysis.md` §15). Suppression is deliberately
//! **not** applied here — that is a rendering decision the caller
//! makes per node/edge (`src/controllers/tba.rs`'s `process_map`),
//! exactly as `cohort_time_analysis`/`cohort_constraints` apply it
//! after calling the suppression-agnostic `tba::cohort`/`tba::constraints`.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::tba;

/// Everything about one pathway instance except its raw segment / step /
/// event / team rows, which are supplied separately (they may be capped
/// or paged independently of the case-level fields).
///
/// `case_id` is the instance `pid` — **never** `subject_ref` and never a
/// person URN. Building this from a `pathway_instances::Model` is the
/// caller's job precisely so this module never sees the subject field
/// at all, not merely "chooses not to use" it.
#[derive(Debug, Clone)]
pub struct CaseContext {
    /// The instance `pid`, stringified. The event log's `case_id`.
    pub case_id: String,
    /// The parent pathway template's `pid`, stringified.
    pub pathway_pid: String,
    /// The template's care setting, if declared.
    pub care_setting: Option<String>,
    /// The instance's urgency lens.
    pub urgency: String,
    /// The instance's lifecycle status (`active`, `on_hold`, …).
    pub status: String,
    /// The instance's recorded outcome, once closed.
    pub outcome: Option<String>,
}

/// One journey segment as the export codecs see it — a narrowed
/// [`tba::Segment`] with epoch-millisecond endpoints already resolved by
/// the caller (this module makes no clock decisions of its own).
#[derive(Debug, Clone)]
pub struct SegmentInput {
    /// One of [`tba::STAGES`].
    pub stage: String,
    /// One of [`tba::CATEGORIES`].
    pub category: String,
    /// One of [`tba::WASTES`], on non-value-adding segments only.
    pub waste: Option<String>,
    /// Interval start, epoch milliseconds.
    pub start_ms: i64,
    /// Interval end, epoch milliseconds; `None` while still running (no
    /// `complete` row is emitted for a running segment).
    pub end_ms: Option<i64>,
    /// Who — a `worker:` / `organization:` URN. Resolved to a team
    /// *role* before it ever reaches an [`EventLogRow`]; never carried
    /// through as the raw URN.
    pub actor_ref: Option<String>,
    /// Where — a `place:` / `organization:` URN, carried as-is (the
    /// spec's explicit column set includes it unmasked).
    pub location_ref: Option<String>,
}

/// One completed instance step. An undone step contributes no row — the
/// domain model records only *when a step was completed*, not when it
/// started, so there is nothing else to log.
#[derive(Debug, Clone)]
pub struct StepInput {
    /// The step's label.
    pub label: String,
    /// Epoch milliseconds of completion, or `None` if not yet done.
    pub done_at_ms: Option<i64>,
}

/// One instance event (a point-in-time occurrence — enrolment, review,
/// status change, …). Deliberately carries no actor: `instance_events.actor`
/// is the *operator* who recorded it (an audit identity), not a care-team
/// member, and T-14a's `resource` column is scoped to segments' `actor_ref`
/// only (see [`event_log_rows`]).
#[derive(Debug, Clone)]
pub struct EventInput {
    /// The event kind, e.g. `enrolled`, `reviewed`.
    pub kind: String,
    /// Epoch milliseconds the event occurred.
    pub occurred_at_ms: i64,
}

/// One row of the `event_log` export (spec `13-tasks.md` T-14a) — the
/// bupaR/PM4Py shape: `case_id`, `activity_id` (here `activity`),
/// `lifecycle_id` (here `lifecycle`), `timestamp`, `resource_id` (here
/// `resource`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventLogRow {
    /// The instance `pid` — never `subject_ref`.
    pub case_id: String,
    /// `stage:<stage>` for a segment, `step:<label>` for a completed
    /// step, `event:<kind>` for an instance event.
    pub activity: String,
    /// `start` or `complete` (bupaR lifecycle vocabulary; every activity
    /// here is a point event or an interval, never `schedule`/`suspend`).
    pub lifecycle: String,
    /// RFC 3339, millisecond precision, UTC.
    pub timestamp: String,
    /// One of [`tba::CATEGORIES`], on a segment row only.
    pub category: Option<String>,
    /// One of [`tba::WASTES`], on a non-value-adding segment row only.
    pub waste: Option<String>,
    /// The care-team **role** of the segment's `actor_ref` — never the
    /// URN, and `None` when the actor is not a recorded team member
    /// (never falls back to the raw ref).
    pub resource: Option<String>,
    /// The segment's `location_ref`, carried as-is.
    pub location_ref: Option<String>,
    /// Case attribute: the parent pathway template's `pid`.
    pub pathway_pid: String,
    /// Case attribute: the template's care setting.
    pub care_setting: Option<String>,
    /// Case attribute: the instance's urgency lens.
    pub urgency: String,
    /// Case attribute: the instance's lifecycle status.
    pub status: String,
    /// Case attribute: the instance's recorded outcome.
    pub outcome: Option<String>,
}

/// One row of the `journey_features` export (spec `13-tasks.md` T-14a) —
/// one row per instance, built directly from a [`tba::InstanceAnalysis`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JourneyFeatureRow {
    /// The instance `pid` — never `subject_ref`.
    pub case_id: String,
    /// Case attribute: the parent pathway template's `pid`.
    pub pathway_pid: String,
    /// Case attribute: the template's care setting.
    pub care_setting: Option<String>,
    /// Case attribute: the instance's urgency lens.
    pub urgency: String,
    /// Case attribute: the instance's lifecycle status.
    pub status: String,
    /// Case attribute: the instance's recorded outcome.
    pub outcome: Option<String>,
    /// Lead time (τ / LT), milliseconds.
    pub lead_time_ms: i64,
    /// Lead time, days.
    pub lead_time_days: f64,
    /// Value time (VT), milliseconds.
    pub value_time_ms: i64,
    /// Process time (PT), milliseconds.
    pub process_time_ms: i64,
    /// Waste time, milliseconds.
    pub waste_time_ms: i64,
    /// Touch time (φ), milliseconds — may exceed lead time.
    pub touch_time_ms: i64,
    /// Wait time (ω), milliseconds.
    pub wait_time_ms: i64,
    /// Unrecorded clock time, milliseconds.
    pub unrecorded_ms: i64,
    /// %VA, `None` when the clock is unmeasurable (never a sentinel 0).
    pub value_adding_ratio: Option<f64>,
    /// %A.
    pub activity_ratio: Option<f64>,
    /// Coverage — how much of the journey was mapped at all.
    pub coverage_ratio: Option<f64>,
    /// `unmapped` | `partial` | `mapped`.
    pub confidence: String,
    /// Segments considered.
    pub segments: usize,
    /// Handoff boundary count (actor or location change).
    pub handoffs_total: usize,
    /// Gap count (maximal uncovered stretches of the clock).
    pub gap_count: usize,
    /// The clock is still running — a right-censored observation
    /// (T-14e's raw signal; this column is the one part of T-14e
    /// buildable without that task's own machinery).
    pub censored: bool,
    /// Per-stage union duration, milliseconds, keyed by [`tba::STAGES`]
    /// name. A stage absent from this map carried no segment for this
    /// instance — treated as `0`, never as a missing column, so a CSV
    /// export is never ragged.
    pub by_stage_ms: BTreeMap<String, i64>,
    /// T-14c (journey variants) is not yet built. Always `None` — a
    /// documented gap, not a silently-empty string — until that task
    /// lands a variant string to carry here.
    pub variant: Option<String>,
    /// This instance's stage anchors and adjacent-pair delays (T-14d),
    /// JSON-encoded in one cell (`{"anchors": […], "delays": […]}`) per
    /// the family's CSV nested-value convention
    /// (`agents/share/bulk-import-export.md` §5) — `Some` for every row,
    /// since [`tba::anchors`]/[`tba::delays`] always return one entry
    /// per [`tba::STAGES`]/adjacent pair, `None` values and all.
    /// `None` only if the JSON encoding itself somehow fails, which no
    /// value these types can hold is expected to trigger.
    pub anchors_delays: Option<String>,
    /// T-14i (conformance to the enrolled template) is not yet built.
    /// Always `None`, for the same reason as `variant`.
    pub conformance: Option<String>,
}

/// `application/x-ndjson` — the JSONL export's content type.
pub const NDJSON_CONTENT_TYPE: &str = "application/x-ndjson";

/// `text/csv` — the CSV export's content type.
pub const CSV_CONTENT_TYPE: &str = "text/csv";

/// Epoch milliseconds → RFC 3339 (UTC, millisecond precision) — what a
/// bupaR/PM4Py/ehrapy reader expects an event-log `timestamp` column to
/// parse as. An out-of-range value (never expected from a stored clock,
/// but never trusted either) renders as the empty string rather than
/// panicking.
fn rfc3339_ms(ms: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

/// Build the `event_log` rows for one instance (spec T-14a): one row per
/// segment start (+ a second on close), one row per completed step, one
/// row per instance event — case attributes repeated on every row, and
/// every row's `resource` is a team role or `None`, never a URN.
///
/// Rows are sorted by timestamp (ties broken by activity name), so a
/// multi-instance export sorted case-by-case is still chronological
/// within each case.
#[must_use]
pub fn event_log_rows(
    ctx: &CaseContext,
    segments: &[SegmentInput],
    steps: &[StepInput],
    events: &[EventInput],
    team_roles: &BTreeMap<String, String>,
) -> Vec<EventLogRow> {
    let row = |activity: String,
               lifecycle: &str,
               ts_ms: i64,
               category: Option<String>,
               waste: Option<String>,
               resource: Option<String>,
               location_ref: Option<String>| EventLogRow {
        case_id: ctx.case_id.clone(),
        activity,
        lifecycle: lifecycle.to_string(),
        timestamp: rfc3339_ms(ts_ms),
        category,
        waste,
        resource,
        location_ref,
        pathway_pid: ctx.pathway_pid.clone(),
        care_setting: ctx.care_setting.clone(),
        urgency: ctx.urgency.clone(),
        status: ctx.status.clone(),
        outcome: ctx.outcome.clone(),
    };

    let mut rows = Vec::new();
    for seg in segments {
        let resource = seg
            .actor_ref
            .as_deref()
            .and_then(|actor| team_roles.get(actor))
            .cloned();
        rows.push(row(
            format!("stage:{}", seg.stage),
            "start",
            seg.start_ms,
            Some(seg.category.clone()),
            seg.waste.clone(),
            resource.clone(),
            seg.location_ref.clone(),
        ));
        if let Some(end_ms) = seg.end_ms {
            rows.push(row(
                format!("stage:{}", seg.stage),
                "complete",
                end_ms,
                Some(seg.category.clone()),
                seg.waste.clone(),
                resource,
                seg.location_ref.clone(),
            ));
        }
    }
    for step in steps {
        if let Some(done_ms) = step.done_at_ms {
            rows.push(row(
                format!("step:{}", step.label),
                "complete",
                done_ms,
                None,
                None,
                None,
                None,
            ));
        }
    }
    for event in events {
        rows.push(row(
            format!("event:{}", event.kind),
            "complete",
            event.occurred_at_ms,
            None,
            None,
            None,
            None,
        ));
    }
    rows.sort_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| a.activity.cmp(&b.activity))
    });
    rows
}

/// Build the `journey_features` row for one instance directly from its
/// [`tba::InstanceAnalysis`] (spec T-14a).
#[must_use]
pub fn journey_feature_row(
    ctx: &CaseContext,
    analysis: &tba::InstanceAnalysis,
) -> JourneyFeatureRow {
    let by_stage_ms = analysis
        .by_stage
        .iter()
        .map(|s| (s.stage.clone(), s.ms))
        .collect();
    JourneyFeatureRow {
        case_id: ctx.case_id.clone(),
        pathway_pid: ctx.pathway_pid.clone(),
        care_setting: ctx.care_setting.clone(),
        urgency: ctx.urgency.clone(),
        status: ctx.status.clone(),
        outcome: ctx.outcome.clone(),
        lead_time_ms: analysis.lead_time_ms,
        lead_time_days: analysis.lead_time_days,
        value_time_ms: analysis.value_time_ms,
        process_time_ms: analysis.process_time_ms,
        waste_time_ms: analysis.waste_time_ms,
        touch_time_ms: analysis.touch_time_ms,
        wait_time_ms: analysis.wait_time_ms,
        unrecorded_ms: analysis.unrecorded_ms,
        value_adding_ratio: analysis.value_adding_ratio.value,
        activity_ratio: analysis.activity_ratio.value,
        coverage_ratio: analysis.coverage_ratio.value,
        confidence: analysis.confidence.to_string(),
        segments: analysis.segments,
        handoffs_total: analysis.handoffs.total,
        gap_count: analysis.gaps.len(),
        censored: analysis.clock.running,
        by_stage_ms,
        variant: None,
        anchors_delays: serde_json::to_string(&serde_json::json!({
            "anchors": analysis.anchors,
            "delays": analysis.delays,
        }))
        .ok(),
        conformance: None,
    }
}

/// Quote a CSV field only when it needs it (contains a comma, quote, or
/// newline), doubling any internal quote — RFC 4180's minimal escaping.
fn csv_escape(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Join already-stringified fields into one escaped CSV line (no
/// trailing newline).
fn csv_line<S: AsRef<str>>(fields: &[S]) -> String {
    fields
        .iter()
        .map(|f| csv_escape(f.as_ref()))
        .collect::<Vec<_>>()
        .join(",")
}

/// `Some(x)` → `"x"` string; `None` → empty field — CSV's own convention
/// for absent, distinct from a genuine empty string (which this data
/// never produces, since every optional column is either a closed
/// vocabulary token or a URN-derived role).
fn opt_field<T: ToString>(value: Option<T>) -> String {
    value.map_or_else(String::new, |v| v.to_string())
}

const EVENT_LOG_HEADER: [&str; 13] = [
    "case_id",
    "activity",
    "lifecycle",
    "timestamp",
    "category",
    "waste",
    "resource",
    "location_ref",
    "pathway_pid",
    "care_setting",
    "urgency",
    "status",
    "outcome",
];

fn event_log_fields(row: &EventLogRow) -> [String; 13] {
    [
        row.case_id.clone(),
        row.activity.clone(),
        row.lifecycle.clone(),
        row.timestamp.clone(),
        opt_field(row.category.clone()),
        opt_field(row.waste.clone()),
        opt_field(row.resource.clone()),
        opt_field(row.location_ref.clone()),
        row.pathway_pid.clone(),
        opt_field(row.care_setting.clone()),
        row.urgency.clone(),
        row.status.clone(),
        opt_field(row.outcome.clone()),
    ]
}

/// Render `event_log` rows as CSV (header row + one row per entry).
#[must_use]
pub fn event_log_csv(rows: &[EventLogRow]) -> String {
    let mut out = csv_line(&EVENT_LOG_HEADER);
    out.push('\n');
    for row in rows {
        out.push_str(&csv_line(&event_log_fields(row)));
        out.push('\n');
    }
    out
}

/// Render `event_log` rows as JSONL (one JSON object per line).
#[must_use]
pub fn event_log_jsonl(rows: &[EventLogRow]) -> String {
    to_jsonl(rows)
}

/// Column order for the `journey_features` CSV export. The `by_stage_ms`
/// map is flattened into one `stage_<name>_ms` column per
/// [`tba::STAGES`] entry, in that module's declared order.
fn journey_feature_header() -> Vec<String> {
    let mut header: Vec<String> = [
        "case_id",
        "pathway_pid",
        "care_setting",
        "urgency",
        "status",
        "outcome",
        "lead_time_ms",
        "lead_time_days",
        "value_time_ms",
        "process_time_ms",
        "waste_time_ms",
        "touch_time_ms",
        "wait_time_ms",
        "unrecorded_ms",
        "value_adding_ratio",
        "activity_ratio",
        "coverage_ratio",
        "confidence",
        "segments",
        "handoffs_total",
        "gap_count",
        "censored",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    header.extend(tba::STAGES.iter().map(|stage| format!("stage_{stage}_ms")));
    header.extend(["variant", "anchors_delays", "conformance"].map(ToString::to_string));
    header
}

fn journey_feature_fields(row: &JourneyFeatureRow) -> Vec<String> {
    let mut fields = vec![
        row.case_id.clone(),
        row.pathway_pid.clone(),
        opt_field(row.care_setting.clone()),
        row.urgency.clone(),
        row.status.clone(),
        opt_field(row.outcome.clone()),
        row.lead_time_ms.to_string(),
        format!("{:.3}", row.lead_time_days),
        row.value_time_ms.to_string(),
        row.process_time_ms.to_string(),
        row.waste_time_ms.to_string(),
        row.touch_time_ms.to_string(),
        row.wait_time_ms.to_string(),
        row.unrecorded_ms.to_string(),
        opt_field(row.value_adding_ratio.map(|v| format!("{v:.4}"))),
        opt_field(row.activity_ratio.map(|v| format!("{v:.4}"))),
        opt_field(row.coverage_ratio.map(|v| format!("{v:.4}"))),
        row.confidence.clone(),
        row.segments.to_string(),
        row.handoffs_total.to_string(),
        row.gap_count.to_string(),
        row.censored.to_string(),
    ];
    fields.extend(tba::STAGES.iter().map(|stage| {
        row.by_stage_ms
            .get(*stage)
            .copied()
            .unwrap_or(0)
            .to_string()
    }));
    fields.push(opt_field(row.variant.clone()));
    fields.push(opt_field(row.anchors_delays.clone()));
    fields.push(opt_field(row.conformance.clone()));
    fields
}

/// Render `journey_features` rows as CSV (header row + one row per
/// instance).
#[must_use]
pub fn journey_features_csv(rows: &[JourneyFeatureRow]) -> String {
    let mut out = csv_line(&journey_feature_header());
    out.push('\n');
    for row in rows {
        out.push_str(&csv_line(&journey_feature_fields(row)));
        out.push('\n');
    }
    out
}

/// Render `journey_features` rows as JSONL (one JSON object per line).
#[must_use]
pub fn journey_features_jsonl(rows: &[JourneyFeatureRow]) -> String {
    to_jsonl(rows)
}

/// Serialize a slice as JSONL: one compact JSON object per line, in
/// order. A row that somehow fails to serialize (never expected — every
/// field is a plain scalar or option) is skipped rather than panicking,
/// since a partial export is recoverable and a panicked one is not.
fn to_jsonl<T: Serialize>(rows: &[T]) -> String {
    let mut out = String::new();
    for row in rows {
        if let Ok(line) = serde_json::to_string(row) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

/// The synthetic node every instance's sequence starts with — makes
/// entry variety visible as the set of edges leaving it.
pub const START_NODE: &str = "start";

/// The synthetic node every instance's sequence ends with — makes exit
/// variety visible as the set of edges arriving at it.
pub const END_NODE: &str = "end";

/// One activity in an instance's ordered sequence, for the directly-
/// follows process map (spec T-14b). `duration_days` is `Some` only
/// for a *closed* stage segment — a step is a point-in-time completion
/// with no duration, and neither pseudo-node has one either.
///
/// `end_ms` is deliberately separate from `start_ms`: the edge gap
/// leaving this activity is measured from *here* it ends, not from
/// when it started, so two activities that genuinely overlap or abut
/// contribute a near-zero gap rather than the activity's own duration
/// being counted as idle time. For a step or a pseudo-node, or a
/// segment that is still running, `end_ms == start_ms` — a point-in-time
/// activity has nothing else to measure, and a still-open segment's
/// true end is unknown, so its own start is the least-wrong stand-in.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivityStep {
    /// The stage name, the step label, or [`START_NODE`]/[`END_NODE`].
    pub activity: String,
    /// When this activity started — the ordering key.
    pub start_ms: i64,
    /// When this activity ended, or `start_ms` when it has no separate
    /// end (a step, a pseudo-node, or a still-open segment).
    pub end_ms: i64,
    /// The segment's duration, when it has closed.
    pub duration_days: Option<f64>,
}

/// Build one instance's activity sequence at **stage** level: every
/// segment (sorted by start), bookended by [`START_NODE`] at the
/// resolved clock start and [`END_NODE`] at the resolved clock stop.
/// Self-loops are never collapsed — two consecutive segments of the
/// same stage are two entries, producing a stage→stage edge.
#[must_use]
pub fn process_map_sequence_from_segments(
    clock_start_ms: i64,
    clock_stop_ms: i64,
    segments: &[SegmentInput],
) -> Vec<ActivityStep> {
    let mut real: Vec<ActivityStep> = segments
        .iter()
        .map(|segment| ActivityStep {
            activity: segment.stage.clone(),
            start_ms: segment.start_ms,
            end_ms: segment.end_ms.unwrap_or(segment.start_ms),
            duration_days: segment
                .end_ms
                .map(|end| duration_days(segment.start_ms, end)),
        })
        .collect();
    real.sort_by_key(|step| step.start_ms);
    bookend(real, clock_start_ms, clock_stop_ms)
}

/// Build one instance's activity sequence at **step** level: every
/// *completed* step (sorted by `done_on`), bookended the same way as
/// [`process_map_sequence_from_segments`]. An undone step contributes
/// nothing — same rule [`event_log_rows`] applies.
#[must_use]
pub fn process_map_sequence_from_steps(
    clock_start_ms: i64,
    clock_stop_ms: i64,
    steps: &[StepInput],
) -> Vec<ActivityStep> {
    let mut real: Vec<ActivityStep> = steps
        .iter()
        .filter_map(|step| {
            step.done_at_ms.map(|ms| ActivityStep {
                activity: step.label.clone(),
                start_ms: ms,
                end_ms: ms,
                duration_days: None,
            })
        })
        .collect();
    real.sort_by_key(|step| step.start_ms);
    bookend(real, clock_start_ms, clock_stop_ms)
}

fn bookend(real: Vec<ActivityStep>, clock_start_ms: i64, clock_stop_ms: i64) -> Vec<ActivityStep> {
    let mut out = Vec::with_capacity(real.len() + 2);
    out.push(ActivityStep {
        activity: START_NODE.to_string(),
        start_ms: clock_start_ms,
        end_ms: clock_start_ms,
        duration_days: None,
    });
    out.extend(real);
    out.push(ActivityStep {
        activity: END_NODE.to_string(),
        start_ms: clock_stop_ms,
        end_ms: clock_stop_ms,
        duration_days: None,
    });
    out
}

fn duration_days(start_ms: i64, end_ms: i64) -> f64 {
    to_days(end_ms - start_ms)
}

#[allow(clippy::cast_precision_loss)] // display-only day count
fn to_days(ms: i64) -> f64 {
    ms as f64 / tba::DAY_MS as f64
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // display days back to whole ms for percentile()
fn to_ms(days: f64) -> i64 {
    (days * tba::DAY_MS as f64) as i64
}

/// One node of a [`ProcessMap`]: an activity plus its cohort-wide
/// counts and (where the activity has one) median duration.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessMapNode {
    /// The stage name, the step label, or [`START_NODE`]/[`END_NODE`].
    pub activity: String,
    /// Distinct instances with at least one occurrence of this activity.
    pub instance_count: usize,
    /// Total occurrences across the cohort — at least `instance_count`,
    /// exceeding it exactly when some instance repeats the activity
    /// (a self-loop).
    pub occurrence_count: usize,
    /// Nearest-rank median duration, for a stage with closed segments.
    /// `None` for a step or a pseudo-node, or when every occurrence is
    /// still running.
    pub median_duration_days: Option<f64>,
}

/// One edge of a [`ProcessMap`]: a directly-follows transition plus its
/// cohort-wide counts and gap statistics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProcessMapEdge {
    /// The transition's source activity.
    pub from: String,
    /// The transition's destination activity.
    pub to: String,
    /// Distinct instances with at least one occurrence of this edge.
    pub instance_count: usize,
    /// Total occurrences across the cohort — what
    /// [`build_process_map`]'s edge-occurrence-count-sums-to-the-
    /// transition-count property is checked against.
    pub occurrence_count: usize,
    /// Nearest-rank median gap between the two activities, days.
    pub median_gap_days: f64,
    /// Nearest-rank p90 gap, days.
    pub p90_gap_days: f64,
}

/// A directly-follows process map: every activity as a node, every
/// observed transition as an edge (spec T-14b). Deliberately carries no
/// suppression decision — see this module's own doc comment.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct ProcessMap {
    /// Nodes, most-frequent first.
    pub nodes: Vec<ProcessMapNode>,
    /// Edges, most-frequent first.
    pub edges: Vec<ProcessMapEdge>,
}

/// Derive the process map from one cohort's already-built activity
/// sequences (one per instance, each already bookended with
/// [`START_NODE`]/[`END_NODE`]). Pure: no I/O, no suppression, no
/// clock read.
#[must_use]
pub fn build_process_map(sequences: &[Vec<ActivityStep>]) -> ProcessMap {
    let mut node_instances: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut node_occurrences: BTreeMap<String, usize> = BTreeMap::new();
    let mut node_durations_ms: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut edge_instances: BTreeMap<(String, String), BTreeSet<usize>> = BTreeMap::new();
    let mut edge_occurrences: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut edge_gaps_ms: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();

    for (index, sequence) in sequences.iter().enumerate() {
        for step in sequence {
            node_instances
                .entry(step.activity.clone())
                .or_default()
                .insert(index);
            *node_occurrences.entry(step.activity.clone()).or_default() += 1;
            if let Some(days) = step.duration_days {
                node_durations_ms
                    .entry(step.activity.clone())
                    .or_default()
                    .push(to_ms(days));
            }
        }
        for pair in sequence.windows(2) {
            let key = (pair[0].activity.clone(), pair[1].activity.clone());
            edge_instances.entry(key.clone()).or_default().insert(index);
            *edge_occurrences.entry(key.clone()).or_default() += 1;
            let gap_ms = (pair[1].start_ms - pair[0].end_ms).max(0);
            edge_gaps_ms.entry(key).or_default().push(gap_ms);
        }
    }

    let mut nodes: Vec<ProcessMapNode> = node_occurrences
        .keys()
        .map(|activity| {
            let mut durations = node_durations_ms.get(activity).cloned().unwrap_or_default();
            durations.sort_unstable();
            ProcessMapNode {
                activity: activity.clone(),
                instance_count: node_instances.get(activity).map_or(0, BTreeSet::len),
                occurrence_count: node_occurrences[activity],
                median_duration_days: tba::percentile(&durations, 0.5).map(to_days),
            }
        })
        .collect();
    nodes.sort_by(|a, b| {
        b.occurrence_count
            .cmp(&a.occurrence_count)
            .then_with(|| a.activity.cmp(&b.activity))
    });

    let mut edges: Vec<ProcessMapEdge> = edge_occurrences
        .keys()
        .map(|key| {
            let mut gaps = edge_gaps_ms.get(key).cloned().unwrap_or_default();
            gaps.sort_unstable();
            ProcessMapEdge {
                from: key.0.clone(),
                to: key.1.clone(),
                instance_count: edge_instances.get(key).map_or(0, BTreeSet::len),
                occurrence_count: edge_occurrences[key],
                median_gap_days: tba::percentile(&gaps, 0.5).map_or(0.0, to_days),
                p90_gap_days: tba::percentile(&gaps, 0.9).map_or(0.0, to_days),
            }
        })
        .collect();
    edges.sort_by(|a, b| {
        b.occurrence_count
            .cmp(&a.occurrence_count)
            .then_with(|| (&a.from, &a.to).cmp(&(&b.from, &b.to)))
    });

    ProcessMap { nodes, edges }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> CaseContext {
        CaseContext {
            case_id: "11111111-1111-1111-1111-111111111111".to_string(),
            pathway_pid: "22222222-2222-2222-2222-222222222222".to_string(),
            care_setting: Some("oncology".to_string()),
            urgency: "routine".to_string(),
            status: "active".to_string(),
            outcome: None,
        }
    }

    fn segment(
        stage: &str,
        start_ms: i64,
        end_ms: Option<i64>,
        actor_ref: Option<&str>,
    ) -> SegmentInput {
        SegmentInput {
            stage: stage.to_string(),
            category: tba::CATEGORY_VALUE_ADDING.to_string(),
            waste: None,
            start_ms,
            end_ms,
            actor_ref: actor_ref.map(ToString::to_string),
            location_ref: Some("place:33333333-3333-3333-3333-333333333333".to_string()),
        }
    }

    /// A closed segment produces both a `start` and a `complete` row; a
    /// still-running one produces only `start`.
    #[test]
    fn segments_emit_start_and_complete_rows() {
        let ctx = ctx();
        let segments = [
            segment("triage", 1_000, Some(5_000), None),
            segment("treatment", 6_000, None, None),
        ];
        let rows = event_log_rows(&ctx, &segments, &[], &[], &BTreeMap::new());
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].activity, "stage:triage");
        assert_eq!(rows[0].lifecycle, "start");
        assert_eq!(rows[1].activity, "stage:triage");
        assert_eq!(rows[1].lifecycle, "complete");
        assert_eq!(rows[2].activity, "stage:treatment");
        assert_eq!(rows[2].lifecycle, "start");
    }

    /// `resource` is the team role, resolved by `actor_ref`, and `None`
    /// when the actor is not a recorded team member — never the raw URN.
    #[test]
    fn resource_is_the_team_role_never_the_urn() {
        let ctx = ctx();
        let actor = "worker:44444444-4444-4444-4444-444444444444";
        let mut team = BTreeMap::new();
        team.insert(actor.to_string(), "nurse".to_string());
        let segments = [segment("triage", 1_000, Some(2_000), Some(actor))];
        let rows = event_log_rows(&ctx, &segments, &[], &[], &team);
        assert_eq!(rows[0].resource.as_deref(), Some("nurse"));
        assert_eq!(rows[1].resource.as_deref(), Some("nurse"));

        let unknown_actor = "worker:55555555-5555-5555-5555-555555555555";
        let segments = [segment("triage", 1_000, Some(2_000), Some(unknown_actor))];
        let rows = event_log_rows(&ctx, &segments, &[], &[], &team);
        assert_eq!(
            rows[0].resource, None,
            "unknown actor never falls back to the raw URN"
        );
    }

    /// Only completed steps produce a row.
    #[test]
    fn undone_steps_emit_no_row() {
        let ctx = ctx();
        let steps = [
            StepInput {
                label: "consent".to_string(),
                done_at_ms: Some(1_000),
            },
            StepInput {
                label: "follow-up call".to_string(),
                done_at_ms: None,
            },
        ];
        let rows = event_log_rows(&ctx, &[], &steps, &[], &BTreeMap::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].activity, "step:consent");
        assert_eq!(rows[0].lifecycle, "complete");
    }

    /// Instance events carry no resource — `instance_events.actor` is an
    /// audit identity, not a team role, and is deliberately never surfaced.
    #[test]
    fn events_carry_no_resource() {
        let ctx = ctx();
        let events = [EventInput {
            kind: "reviewed".to_string(),
            occurred_at_ms: 9_000,
        }];
        let rows = event_log_rows(&ctx, &[], &[], &events, &BTreeMap::new());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].activity, "event:reviewed");
        assert!(rows[0].resource.is_none());
    }

    /// Rows are sorted chronologically.
    #[test]
    fn rows_are_sorted_by_timestamp() {
        let ctx = ctx();
        let events = [
            EventInput {
                kind: "second".to_string(),
                occurred_at_ms: 5_000,
            },
            EventInput {
                kind: "first".to_string(),
                occurred_at_ms: 1_000,
            },
        ];
        let rows = event_log_rows(&ctx, &[], &[], &events, &BTreeMap::new());
        assert_eq!(rows[0].activity, "event:first");
        assert_eq!(rows[1].activity, "event:second");
    }

    /// No `event_log` row, across every field of every row shape, ever
    /// contains the substring `"person:"` or the literal `subject_ref` —
    /// T-14a's acceptance criterion, pinned directly.
    #[test]
    fn event_log_never_carries_a_person_urn_or_subject_ref() {
        let ctx = ctx();
        let actor = "worker:44444444-4444-4444-4444-444444444444";
        let mut team = BTreeMap::new();
        team.insert(actor.to_string(), "nurse".to_string());
        let segments = [segment("triage", 1_000, Some(2_000), Some(actor))];
        let steps = [StepInput {
            label: "consent".to_string(),
            done_at_ms: Some(1_500),
        }];
        let events = [EventInput {
            kind: "reviewed".to_string(),
            occurred_at_ms: 1_800,
        }];
        let rows = event_log_rows(&ctx, &segments, &steps, &events, &team);
        let csv = event_log_csv(&rows);
        let jsonl = event_log_jsonl(&rows);
        for haystack in [csv.as_str(), jsonl.as_str()] {
            assert!(
                !haystack.contains("person:"),
                "leaked a person URN: {haystack}"
            );
            assert!(
                !haystack.contains("subject_ref"),
                "leaked subject_ref: {haystack}"
            );
        }
    }

    fn sample_analysis() -> tba::InstanceAnalysis {
        let clock = tba::Clock {
            start_ms: 0,
            stop_ms: 10 * tba::DAY_MS,
            start_source: "enrolled_on",
            stop_source: "as_of",
            running: true,
        };
        let segments = [tba::Segment {
            label: "MRI".to_string(),
            stage: "diagnostics".to_string(),
            category: tba::CATEGORY_VALUE_ADDING.to_string(),
            waste: None,
            start_ms: 0,
            end_ms: Some(2 * tba::DAY_MS),
            actor_ref: None,
            location_ref: None,
        }];
        tba::analyze(clock, &segments, 10 * tba::DAY_MS)
    }

    /// `journey_feature_row` builds directly from an `InstanceAnalysis`,
    /// carries `censored` from `clock.running`, defers the still-unwired
    /// T-14c/i columns as `None` rather than omitting or fabricating
    /// them, and (T-14d) wires `anchors_delays` from the same analysis.
    #[test]
    fn journey_feature_row_derives_from_the_analysis() {
        let ctx = ctx();
        let analysis = sample_analysis();
        let row = journey_feature_row(&ctx, &analysis);
        assert_eq!(row.case_id, ctx.case_id);
        assert_eq!(row.lead_time_ms, analysis.lead_time_ms);
        assert_eq!(row.segments, analysis.segments);
        assert!(row.censored, "the clock was still running");
        assert_eq!(row.by_stage_ms.get("diagnostics"), Some(&(2 * tba::DAY_MS)));
        assert_eq!(
            row.by_stage_ms.get("triage"),
            None,
            "no segment in this stage"
        );
        assert_eq!(row.variant, None, "T-14c is landed but not yet wired here");
        assert_eq!(row.conformance, None, "T-14i is not yet built");
        let anchors_delays: serde_json::Value =
            serde_json::from_str(row.anchors_delays.as_deref().expect("T-14d is wired"))
                .expect("valid JSON");
        assert_eq!(
            anchors_delays["anchors"]
                .as_array()
                .expect("anchors array")
                .len(),
            tba::STAGES.len(),
            "one anchor per STAGES entry, reached or not"
        );
        assert_eq!(
            anchors_delays["delays"]
                .as_array()
                .expect("delays array")
                .len(),
            tba::STAGES.len() - 1,
            "one delay per adjacent STAGES pair"
        );
    }

    /// Every `tba::STAGES` entry gets its own CSV column, in order, even
    /// when the instance's `by_stage_ms` map is missing most of them —
    /// the export is never ragged.
    #[test]
    fn journey_features_csv_has_one_column_per_stage() {
        let ctx = ctx();
        let row = journey_feature_row(&ctx, &sample_analysis());
        let csv = journey_features_csv(&[row]);
        let header = csv.lines().next().unwrap();
        for stage in tba::STAGES {
            assert!(
                header.contains(&format!("stage_{stage}_ms")),
                "missing stage column for {stage}: {header}"
            );
        }
        // header + exactly one data row
        assert_eq!(csv.lines().count(), 2);
    }

    /// The exact `event_log` column set, order-pinned — a bupaR
    /// `eventlog(case_id, activity_id, lifecycle_id, timestamp,
    /// resource_id)` mapping breaks the moment a column is renamed,
    /// reordered, added, or removed without updating the caller's
    /// mapping, so a change here must be deliberate, not incidental.
    #[test]
    fn event_log_csv_header_is_pinned() {
        let csv = event_log_csv(&[]);
        assert_eq!(
            csv.lines().next().unwrap(),
            "case_id,activity,lifecycle,timestamp,category,waste,resource,\
             location_ref,pathway_pid,care_setting,urgency,status,outcome"
        );
    }

    /// The exact `journey_features` column set, order-pinned, for the
    /// same reason as `event_log_csv_header_is_pinned`.
    #[test]
    fn journey_features_csv_header_is_pinned() {
        let csv = journey_features_csv(&[]);
        assert_eq!(
            csv.lines().next().unwrap(),
            "case_id,pathway_pid,care_setting,urgency,status,outcome,lead_time_ms,\
             lead_time_days,value_time_ms,process_time_ms,waste_time_ms,touch_time_ms,\
             wait_time_ms,unrecorded_ms,value_adding_ratio,activity_ratio,coverage_ratio,\
             confidence,segments,handoffs_total,gap_count,censored,stage_referral_ms,\
             stage_triage_ms,stage_diagnostics_ms,stage_treatment_ms,stage_follow_up_ms,\
             stage_discharge_ms,stage_other_ms,variant,anchors_delays,conformance"
        );
    }

    /// The `journey_features` export never carries a person URN or
    /// `subject_ref` either — the case attributes are the same closed
    /// set as the event log's.
    #[test]
    fn journey_features_never_carries_a_person_urn_or_subject_ref() {
        let ctx = ctx();
        let row = journey_feature_row(&ctx, &sample_analysis());
        let csv = journey_features_csv(std::slice::from_ref(&row));
        let jsonl = journey_features_jsonl(&[row]);
        for haystack in [csv.as_str(), jsonl.as_str()] {
            assert!(!haystack.contains("person:"));
            assert!(!haystack.contains("subject_ref"));
        }
    }

    /// A field containing a comma or quote is quoted, with internal
    /// quotes doubled — RFC 4180 minimal escaping.
    #[test]
    fn csv_escaping_quotes_only_when_needed() {
        assert_eq!(csv_escape("plain"), "plain");
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
        assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
    }

    /// JSONL round-trips one object per line, and an empty slice renders
    /// as an empty string (no stray header/footer for a zero-row export).
    #[test]
    fn jsonl_is_one_object_per_line() {
        assert_eq!(event_log_jsonl(&[]), "");
        let ctx = ctx();
        let events = [EventInput {
            kind: "enrolled".to_string(),
            occurred_at_ms: 0,
        }];
        let rows = event_log_rows(&ctx, &[], &[], &events, &BTreeMap::new());
        let jsonl = event_log_jsonl(&rows);
        assert_eq!(jsonl.lines().count(), 1);
        let parsed: serde_json::Value =
            serde_json::from_str(jsonl.lines().next().unwrap()).unwrap();
        assert_eq!(parsed["activity"], "event:enrolled");
    }

    // -------------------------------------------------------------
    // T-14b: directly-follows process map
    // -------------------------------------------------------------

    fn stage_segment(stage: &str, start_ms: i64, end_ms: Option<i64>) -> SegmentInput {
        SegmentInput {
            stage: stage.to_string(),
            category: tba::CATEGORY_VALUE_ADDING.to_string(),
            waste: None,
            start_ms,
            end_ms,
            actor_ref: None,
            location_ref: None,
        }
    }

    /// Every occurrence contributes exactly one transition to the sum
    /// of edge occurrence counts — the acceptance criterion, checked
    /// directly rather than by proxy.
    #[test]
    fn edge_occurrence_counts_sum_to_the_transition_count() {
        let seq_a = process_map_sequence_from_segments(
            0,
            30 * tba::DAY_MS,
            &[
                stage_segment("triage", tba::DAY_MS, Some(2 * tba::DAY_MS)),
                stage_segment("treatment", 3 * tba::DAY_MS, Some(5 * tba::DAY_MS)),
            ],
        );
        let seq_b = process_map_sequence_from_segments(
            0,
            10 * tba::DAY_MS,
            &[stage_segment("triage", tba::DAY_MS, Some(2 * tba::DAY_MS))],
        );
        let sequences = vec![seq_a.clone(), seq_b.clone()];
        let map = build_process_map(&sequences);
        let total_transitions: usize = sequences.iter().map(|s| s.len() - 1).sum();
        let summed: usize = map.edges.iter().map(|e| e.occurrence_count).sum();
        assert_eq!(summed, total_transitions);
        assert_eq!(total_transitions, seq_a.len() - 1 + seq_b.len() - 1);
    }

    /// A cohort where every instance follows the identical sequence
    /// yields a simple chain: every node but `start`/`end` has exactly
    /// one distinct predecessor and one distinct successor activity.
    #[test]
    fn a_cohort_of_one_variant_yields_a_chain() {
        let make = || {
            process_map_sequence_from_segments(
                0,
                30 * tba::DAY_MS,
                &[
                    stage_segment("triage", tba::DAY_MS, Some(2 * tba::DAY_MS)),
                    stage_segment("treatment", 3 * tba::DAY_MS, Some(5 * tba::DAY_MS)),
                    stage_segment("discharge", 6 * tba::DAY_MS, Some(7 * tba::DAY_MS)),
                ],
            )
        };
        let sequences = vec![make(), make(), make()];
        let map = build_process_map(&sequences);
        assert_eq!(
            map.nodes.len(),
            5,
            "start, triage, treatment, discharge, end"
        );
        assert_eq!(map.edges.len(), 4, "one edge per consecutive pair");
        for node in &map.nodes {
            assert_eq!(
                node.instance_count, 3,
                "{}: every instance visits every node in a single-variant cohort",
                node.activity
            );
        }
    }

    /// Median and p90 gaps match a hand-computed value over a known
    /// set of gaps.
    #[test]
    fn median_and_p90_gaps_match_hand_computed_values() {
        // Three instances, each with one triage->treatment edge, gaps
        // of 1, 2, and 10 days: nearest-rank median (p50) of [1,2,10]
        // is 2 (rank = ceil(0.5*3) = 2 -> index 1), p90 is 10.
        let make = |gap_days: i64| {
            process_map_sequence_from_segments(
                0,
                30 * tba::DAY_MS,
                &[
                    stage_segment("triage", tba::DAY_MS, Some(2 * tba::DAY_MS)),
                    stage_segment(
                        "treatment",
                        (2 + gap_days) * tba::DAY_MS,
                        Some((3 + gap_days) * tba::DAY_MS),
                    ),
                ],
            )
        };
        let sequences = vec![make(1), make(2), make(10)];
        let map = build_process_map(&sequences);
        let edge = map
            .edges
            .iter()
            .find(|e| e.from == "triage" && e.to == "treatment")
            .expect("triage->treatment edge");
        assert_eq!(edge.occurrence_count, 3);
        assert!(
            (edge.median_gap_days - 2.0).abs() < 1e-9,
            "{}",
            edge.median_gap_days
        );
        assert!(
            (edge.p90_gap_days - 10.0).abs() < 1e-9,
            "{}",
            edge.p90_gap_days
        );
    }

    /// A repeated stage is a self-loop edge, kept rather than collapsed
    /// — the occurrence count on the node exceeds the instance count.
    #[test]
    fn self_loops_are_kept_not_collapsed() {
        let sequence = process_map_sequence_from_segments(
            0,
            30 * tba::DAY_MS,
            &[
                stage_segment("treatment", tba::DAY_MS, Some(2 * tba::DAY_MS)),
                stage_segment("treatment", 3 * tba::DAY_MS, Some(4 * tba::DAY_MS)),
            ],
        );
        let map = build_process_map(&[sequence]);
        let node = map
            .nodes
            .iter()
            .find(|n| n.activity == "treatment")
            .expect("treatment node");
        assert_eq!(node.instance_count, 1);
        assert_eq!(node.occurrence_count, 2, "visited twice by one instance");
        let self_loop = map
            .edges
            .iter()
            .find(|e| e.from == "treatment" && e.to == "treatment");
        assert!(self_loop.is_some(), "the self-loop edge must survive");
    }

    /// An instance with no recorded segments still yields a `start` ->
    /// `end` edge — absence is a finding, not an error.
    #[test]
    fn an_empty_sequence_still_bookends_to_start_and_end() {
        let sequence = process_map_sequence_from_segments(0, 5 * tba::DAY_MS, &[]);
        assert_eq!(sequence.len(), 2);
        let map = build_process_map(&[sequence]);
        assert_eq!(map.nodes.len(), 2);
        assert_eq!(map.edges.len(), 1);
        assert_eq!(map.edges[0].from, START_NODE);
        assert_eq!(map.edges[0].to, END_NODE);
    }

    /// A node's `median_duration_days` is `Some` only when at least one
    /// occurrence has closed; a step-level sequence never carries one.
    #[test]
    fn only_closed_segments_contribute_a_duration() {
        let sequence = process_map_sequence_from_segments(
            0,
            10 * tba::DAY_MS,
            &[stage_segment(
                "treatment",
                tba::DAY_MS,
                Some(3 * tba::DAY_MS),
            )],
        );
        let map = build_process_map(&[sequence]);
        let treatment = map
            .nodes
            .iter()
            .find(|n| n.activity == "treatment")
            .expect("treatment node");
        assert_eq!(treatment.median_duration_days, Some(2.0));

        let steps = process_map_sequence_from_steps(
            0,
            10 * tba::DAY_MS,
            &[StepInput {
                label: "consent".to_string(),
                done_at_ms: Some(tba::DAY_MS),
            }],
        );
        let step_map = build_process_map(&[steps]);
        let consent = step_map
            .nodes
            .iter()
            .find(|n| n.activity == "consent")
            .expect("consent node");
        assert_eq!(
            consent.median_duration_days, None,
            "a step is a point in time"
        );
    }

    /// An undone step contributes no node/edge at all — only completed
    /// steps enter the sequence.
    #[test]
    fn undone_steps_are_excluded_from_the_sequence() {
        let steps = [
            StepInput {
                label: "consent".to_string(),
                done_at_ms: Some(tba::DAY_MS),
            },
            StepInput {
                label: "follow-up call".to_string(),
                done_at_ms: None,
            },
        ];
        let sequence = process_map_sequence_from_steps(0, 10 * tba::DAY_MS, &steps);
        assert_eq!(
            sequence.len(),
            3,
            "start, consent, end — not follow-up call"
        );
        assert!(!sequence.iter().any(|s| s.activity == "follow-up call"));
    }
}
