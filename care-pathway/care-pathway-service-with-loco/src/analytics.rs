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
//! Three sibling tasks this module's acceptance criterion cites are not
//! yet built — T-14c (journey variants), T-14d (anchors/delays), T-14i
//! (conformance) — so [`JourneyFeatureRow`] carries their columns as
//! `None` with a documented reason rather than silently omitting them or
//! blocking this task on theirs (see each field's doc comment).

use std::collections::BTreeMap;

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
    /// T-14d (stage anchors and delay decomposition) is not yet built.
    /// Always `None`, for the same reason as `variant`.
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
        anchors_delays: None,
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
    /// carries `censored` from `clock.running`, and defers the
    /// not-yet-built T-14c/d/i columns as `None` rather than omitting or
    /// fabricating them.
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
        assert_eq!(row.variant, None);
        assert_eq!(row.anchors_delays, None);
        assert_eq!(row.conformance, None);
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
}
