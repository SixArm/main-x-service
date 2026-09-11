//! **Time-based analysis** (TBA) — the HTTP surface over
//! [`crate::tba`]. Recording endpoints for journey segments and the
//! pathway clock, and read-only analysis endpoints for one instance,
//! for a pathway cohort, for the ranked constraints, and for
//! queueing-theory flow.
//!
//! See `spec/time-based-analysis.md` §10. Every analysis figure is
//! **derived on read** — there is no stored efficiency column, so a
//! corrected timestamp corrects the analysis, which is exactly the
//! correction the method exists to invite.

use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, PaginatorTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::analytics;
use crate::auth::MaybeAuthUser;
use crate::instances as rules;
use crate::models::_entities::{
    instance_events, instance_segments, instance_steps, pathway_instances,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::care_pathways::Model as PathwayModel;
use crate::split;
use crate::suppression;
use crate::tba;
use crate::variants;

/// Cohort reads are capped so an unbounded cohort cannot become an
/// unbounded query (security invariant 3; spec §11).
const MAX_COHORT_INSTANCES: u64 = 1000;

/// Per-instance segment cap, for the same reason.
const MAX_SEGMENTS: u64 = 5000;

/// The window default for flow analysis (spec §9, §17 — arbitrary, and
/// documented as such).
const DEFAULT_WINDOW_DAYS: i64 = 90;

/// `422` with a reason.
fn refuse(reason: &str) -> Error {
    Error::CustomError(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable_entity", reason),
    )
}

/// Parse a pid or `404`.
fn pid(raw: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).map_err(|_| Error::NotFound)
}

/// Find one live instance, or `404`.
async fn find_instance(ctx: &AppContext, raw: &str) -> Result<pathway_instances::Model> {
    pathway_instances::Entity::find()
        .filter(pathway_instances::Column::Pid.eq(pid(raw)?))
        .filter(pathway_instances::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)
}

/// Epoch milliseconds of a stored timestamp.
fn ms(at: chrono::DateTime<chrono::FixedOffset>) -> i64 {
    at.timestamp_millis()
}

/// A date at midnight UTC, in epoch milliseconds — the day-resolution
/// fallback for an instance predating the clock columns (spec §5.2).
fn date_ms(date: chrono::NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0)
        .map_or(0, |dt| dt.and_utc().timestamp_millis())
}

/// Resolve an instance's clock (spec §5.2), declaring which source each
/// end came from so a day-resolution figure is never mistaken for a
/// measured one.
fn resolve_clock(instance: &pathway_instances::Model, as_of_ms: i64) -> tba::Clock {
    let (start_ms, start_source) = instance.clock_start_at.map_or_else(
        || (date_ms(instance.enrolled_on), "enrolled_on"),
        |at| (ms(at), "clock_start_at"),
    );
    let terminal = rules::is_terminal(&instance.status);
    let (stop_ms, stop_source, running) = match (instance.clock_stop_at, instance.closed_on) {
        (Some(at), _) => (ms(at), "clock_stop_at", false),
        (None, Some(day)) if terminal => (date_ms(day), "closed_on", false),
        _ => (as_of_ms, "as_of", true),
    };
    tba::Clock {
        start_ms,
        stop_ms,
        start_source,
        stop_source,
        running,
    }
}

/// Load one instance's segments, in time order, capped.
async fn load_segments(
    ctx: &AppContext,
    instance_pid: Uuid,
) -> Result<Vec<instance_segments::Model>> {
    Ok(instance_segments::Entity::find()
        .filter(instance_segments::Column::InstancePid.eq(instance_pid))
        .order_by_asc(instance_segments::Column::StartedAt)
        .order_by_asc(instance_segments::Column::Position)
        .limit(MAX_SEGMENTS)
        .all(&ctx.db)
        .await?)
}

/// Stored row → the pure analysis input.
fn to_segment(row: &instance_segments::Model) -> tba::Segment {
    tba::Segment {
        label: row.label.clone(),
        stage: row.stage.clone(),
        category: row.category.clone(),
        waste: row.waste.clone(),
        start_ms: ms(row.started_at),
        end_ms: row.ended_at.map(ms),
        actor_ref: row.actor_ref.clone(),
        location_ref: row.location_ref.clone(),
    }
}

// ---------------------------------------------------------------------
// Recording (spec §10.1)
// ---------------------------------------------------------------------

/// `POST /api/instances/{pid}/segments` body.
#[derive(Debug, serde::Deserialize)]
struct SegmentPayload {
    label: String,
    stage: String,
    category: String,
    #[serde(default)]
    waste: Option<String>,
    started_at: chrono::DateTime<chrono::Utc>,
    /// Omitted opens a running segment.
    #[serde(default)]
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    actor_ref: Option<String>,
    #[serde(default)]
    location_ref: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/instances/{pid}/segments` — record a segment of the
/// journey. `422` on any spec §5.1 invariant.
#[debug_handler]
async fn record_segment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<SegmentPayload>,
) -> Result<Response> {
    if payload.label.trim().is_empty() {
        return Err(refuse("label is required"));
    }
    tba::validate_classification(&payload.stage, &payload.category, payload.waste.as_deref())
        .map_err(|reason| refuse(&reason))?;
    tba::validate_interval(
        payload.started_at.timestamp_millis(),
        payload.ended_at.map(|e| e.timestamp_millis()),
    )
    .map_err(|reason| refuse(&reason))?;

    let instance = find_instance(&ctx, &raw).await?;

    // Invariant 5: at most one open segment. The database enforces this
    // too (a partial unique index), so a concurrent double-POST cannot
    // slip past; this check is what turns that into a readable 422.
    if payload.ended_at.is_none() {
        let open = instance_segments::Entity::find()
            .filter(instance_segments::Column::InstancePid.eq(instance.pid))
            .filter(instance_segments::Column::EndedAt.is_null())
            .one(&ctx.db)
            .await?;
        if let Some(open) = open {
            return Err(refuse(&format!(
                "segment `{}` is still open; close it before opening another \
                 (two simultaneously-running segments have no defensible end time)",
                open.label
            )));
        }
    }

    let position = i32::try_from(
        instance_segments::Entity::find()
            .filter(instance_segments::Column::InstancePid.eq(instance.pid))
            .count(&ctx.db)
            .await?,
    )
    .unwrap_or(i32::MAX);

    let row = instance_segments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        instance_pid: ActiveValue::set(instance.pid),
        label: ActiveValue::set(payload.label.trim().to_string()),
        stage: ActiveValue::set(payload.stage.clone()),
        category: ActiveValue::set(payload.category.clone()),
        waste: ActiveValue::set(payload.waste.clone()),
        started_at: ActiveValue::set(payload.started_at.into()),
        ended_at: ActiveValue::set(payload.ended_at.map(Into::into)),
        actor_ref: ActiveValue::set(payload.actor_ref.clone()),
        location_ref: ActiveValue::set(payload.location_ref.clone()),
        note: ActiveValue::set(payload.note.clone()),
        position: ActiveValue::set(position),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;

    Audit::record(
        &ctx.db,
        instance.pid,
        "instance_segment_recorded",
        caller.actor(),
        Some(serde_json::json!({
            "stage": payload.stage, "category": payload.category,
            "waste": payload.waste, "open": payload.ended_at.is_none(),
        })),
    )
    .await
    .map_err(Error::Model)?;
    format::json(row)
}

/// `GET /api/instances/{pid}/segments` — this instance's segments in
/// time order.
#[debug_handler]
async fn list_segments(State(ctx): State<AppContext>, Path(raw): Path<String>) -> Result<Response> {
    let instance = find_instance(&ctx, &raw).await?;
    format::json(load_segments(&ctx, instance.pid).await?)
}

/// `POST /api/instances/{pid}/segments/{seg}/close` body.
#[derive(Debug, serde::Deserialize)]
struct CloseSegmentPayload {
    /// Defaults to now.
    #[serde(default)]
    ended_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `POST /api/instances/{pid}/segments/{seg}/close` — close a running
/// segment.
#[debug_handler]
async fn close_segment(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((raw, seg)): Path<(String, String)>,
    Json(payload): Json<CloseSegmentPayload>,
) -> Result<Response> {
    let instance = find_instance(&ctx, &raw).await?;
    let row = instance_segments::Entity::find()
        .filter(instance_segments::Column::Pid.eq(pid(&seg)?))
        .filter(instance_segments::Column::InstancePid.eq(instance.pid))
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    if row.ended_at.is_some() {
        return Err(refuse("segment is already closed"));
    }
    let ended_at = payload.ended_at.unwrap_or_else(chrono::Utc::now);
    tba::validate_interval(ms(row.started_at), Some(ended_at.timestamp_millis()))
        .map_err(|reason| refuse(&reason))?;
    let instance_pid = instance.pid;
    let mut active: instance_segments::ActiveModel = row.into();
    active.ended_at = ActiveValue::set(Some(ended_at.into()));
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        instance_pid,
        "instance_segment_closed",
        caller.actor(),
        Some(serde_json::json!({ "segment": updated.pid, "ended_at": ended_at })),
    )
    .await
    .map_err(Error::Model)?;
    format::json(updated)
}

/// `POST /api/instances/{pid}/clock` body.
#[derive(Debug, serde::Deserialize)]
struct ClockPayload {
    /// `start` or `stop`.
    event: String,
    /// Defaults to now.
    #[serde(default)]
    at: Option<chrono::DateTime<chrono::Utc>>,
}

/// `POST /api/instances/{pid}/clock` — set the clock start or stop
/// explicitly.
///
/// There is deliberately no `pause` (spec §12.3): the clock runs from
/// start to stop, and a patient-caused delay is recorded as an
/// `unnecessary_non_value_adding` segment so it is visible and
/// subtractable by the reader rather than silently shrinking the
/// denominator.
#[debug_handler]
async fn set_clock(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(raw): Path<String>,
    Json(payload): Json<ClockPayload>,
) -> Result<Response> {
    let at = payload.at.unwrap_or_else(chrono::Utc::now);
    let instance = find_instance(&ctx, &raw).await?;
    let instance_pid = instance.pid;
    let existing_start = instance
        .clock_start_at
        .map_or_else(|| date_ms(instance.enrolled_on), ms);
    let mut active: pathway_instances::ActiveModel = instance.into();
    match payload.event.as_str() {
        "start" => active.clock_start_at = ActiveValue::set(Some(at.into())),
        "stop" => {
            if at.timestamp_millis() <= existing_start {
                return Err(refuse("clock stop must be strictly after clock start"));
            }
            active.clock_stop_at = ActiveValue::set(Some(at.into()));
        }
        other => {
            return Err(refuse(&format!(
                "unknown clock event `{other}` (events: [\"start\", \"stop\"]; \
                 there is no pause — see spec §12.3)"
            )));
        }
    }
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        instance_pid,
        "instance_clock_set",
        caller.actor(),
        Some(serde_json::json!({ "event": payload.event, "at": at })),
    )
    .await
    .map_err(Error::Model)?;
    format::json(updated)
}

// ---------------------------------------------------------------------
// Analysis (spec §10.2)
// ---------------------------------------------------------------------

/// Analyse one instance: load its clock and segments, then compute.
pub(crate) async fn analyze_instance(
    ctx: &AppContext,
    instance: &pathway_instances::Model,
    as_of_ms: i64,
) -> Result<tba::InstanceAnalysis> {
    let rows = load_segments(ctx, instance.pid).await?;
    let segments: Vec<tba::Segment> = rows.iter().map(to_segment).collect();
    Ok(tba::analyze(
        resolve_clock(instance, as_of_ms),
        &segments,
        as_of_ms,
    ))
}

/// `GET /api/instances/{pid}/time-analysis` — the per-instance TBA
/// (spec §6).
#[debug_handler]
async fn instance_time_analysis(
    State(ctx): State<AppContext>,
    Path(raw): Path<String>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let instance = find_instance(&ctx, &raw).await?;
    let analysis = analyze_instance(&ctx, &instance, now.timestamp_millis()).await?;
    format::json(serde_json::json!({
        "as_of": now,
        "instance": { "pid": instance.pid, "status": instance.status },
        "note": "value_adding_ratio is value time over elapsed calendar time, \
                 not over recorded activity — unrecorded time counts as \
                 non-value-adding, and coverage_ratio says how much of the \
                 journey was mapped at all. by_category partitions the clock \
                 (the four sum to lead time); by_stage may overlap, so its \
                 shares need not. touch_time_ms is the raw sum and may exceed \
                 lead time when care was concurrent.",
        "analysis": analysis,
    }))
}

/// `GET /api/instances/{pid}/timeline` — the mapped journey as an
/// ordered wall of segments and gaps: the visual artefact of the method
/// (spec §10.2).
#[debug_handler]
async fn instance_timeline(
    State(ctx): State<AppContext>,
    Path(raw): Path<String>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let as_of_ms = now.timestamp_millis();
    let instance = find_instance(&ctx, &raw).await?;
    let clock = resolve_clock(&instance, as_of_ms);
    let rows = load_segments(&ctx, instance.pid).await?;
    let segments: Vec<tba::Segment> = rows.iter().map(to_segment).collect();
    let analysis = tba::analyze(clock, &segments, as_of_ms);

    // Interleave recorded segments and gaps into one time-ordered wall.
    let mut wall: Vec<(i64, serde_json::Value)> = rows
        .iter()
        .map(|row| {
            let end = row.ended_at.map_or(as_of_ms, ms);
            let duration = end.saturating_sub(ms(row.started_at)).max(0);
            (
                ms(row.started_at),
                serde_json::json!({
                    "kind": "segment",
                    "pid": row.pid,
                    "label": row.label,
                    "stage": row.stage,
                    "category": row.category,
                    "waste": row.waste,
                    "started_at": row.started_at,
                    "ended_at": row.ended_at,
                    "open": row.ended_at.is_none(),
                    "actor_ref": row.actor_ref,
                    "location_ref": row.location_ref,
                    "duration_ms": duration,
                    "duration_days": tba::as_days(duration),
                }),
            )
        })
        .collect();
    for gap in &analysis.gaps {
        wall.push((
            gap.start_ms,
            serde_json::json!({
                "kind": "gap",
                "label": format!(
                    "{} → {}",
                    gap.after.as_deref().unwrap_or("clock start"),
                    gap.before.as_deref().unwrap_or("clock stop")
                ),
                "stage": gap.stage,
                "duration_ms": gap.duration_ms,
                "duration_days": gap.days,
                "at_handoff": gap.at_handoff,
            }),
        ));
    }
    wall.sort_by_key(|(start, _)| *start);

    format::json(serde_json::json!({
        "as_of": now,
        "instance": { "pid": instance.pid, "status": instance.status },
        "clock": clock,
        "note": "segments and gaps interleaved in time order; a `gap` is clock \
                 time no segment covered, named by what it sits between",
        "totals": {
            "lead_time_ms": analysis.lead_time_ms,
            "lead_time_days": analysis.lead_time_days,
            "value_adding_ratio": analysis.value_adding_ratio,
            "coverage_ratio": analysis.coverage_ratio,
            "confidence": analysis.confidence,
        },
        "wall": wall.into_iter().map(|(_, v)| v).collect::<Vec<_>>(),
    }))
}

/// Cohort query parameters.
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct CohortQuery {
    /// A [`tba::STANDARDS`] id.
    #[serde(default)]
    standard: Option<String>,
    /// An explicit threshold in days, for a local promise.
    #[serde(default)]
    target_days: Option<f64>,
    /// `open` | `closed` | `all` (default `all`).
    #[serde(default)]
    status: Option<String>,
    /// `withhold` (default) | `remove` — how a suppressed cohort's
    /// detail renders (spec T-14k). Parsed via
    /// [`suppression::Mode::parse`].
    #[serde(default)]
    mode: Option<String>,
    /// A named stage (a [`tba::STAGES`] value) to anchor the compliance
    /// interval's start on (spec T-14d). Requires `to_anchor`; an
    /// unrecognised or one-sided pair falls back to the ordinary
    /// whole-clock lead time with a disclosed
    /// `compliance.anchor_note`, rather than approximating a scoring
    /// that wasn't actually requested.
    #[serde(default)]
    from_anchor: Option<String>,
    /// The stage to anchor the compliance interval's end on (spec
    /// T-14d). Requires `from_anchor`; see its doc for the fallback
    /// rule. Not required to be adjacent to `from_anchor` in
    /// [`tba::STAGES`] — `referral` to `treatment` is as valid as
    /// `referral` to `triage`.
    #[serde(default)]
    to_anchor: Option<String>,
    /// `event` (default) | `censor` — whether a `discontinued` closure
    /// counts as the event or as a right-censoring in the Kaplan–Meier
    /// survival blocks (spec T-14e). Echoed on the response
    /// (`survival.discontinued`) rather than silently applied.
    #[serde(default)]
    discontinued: Option<String>,
    /// A comma-separated list of `type:value` predicates every
    /// matched instance must satisfy (spec T-14f); see
    /// [`rules::Predicate`] for the closed-where-possible vocabulary.
    /// Absent (with `excludes`) reproduces today's unsplit cohort.
    #[serde(default)]
    contains: Option<String>,
    /// A comma-separated list of `type:value` predicates no matched
    /// instance may satisfy (spec T-14f).
    #[serde(default)]
    excludes: Option<String>,
    /// Return the complement side's figures too, alongside the
    /// matched side (spec T-14f). Ignored (no `split` block at all)
    /// when `contains`/`excludes` are both absent — there is nothing
    /// to compare.
    #[serde(default)]
    compare: Option<bool>,
}

/// The `pathway_instances` filter [`load_cohort`] and the attrition
/// record's own counts (spec T-14g) both build from, so a query used
/// to *load* the cohort and a query used to *count* it (unbounded by
/// [`MAX_COHORT_INSTANCES`]) can never silently diverge.
fn cohort_query(
    pathway_pid: Uuid,
    status: Option<&str>,
) -> sea_orm::Select<pathway_instances::Entity> {
    let query = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::PathwayPid.eq(pathway_pid))
        .filter(pathway_instances::Column::DeletedAt.is_null());
    match status {
        Some("open") => {
            query.filter(pathway_instances::Column::Status.is_in(["active", "on_hold"]))
        }
        Some("closed") => {
            query.filter(pathway_instances::Column::Status.is_in(["completed", "discontinued"]))
        }
        _ => query,
    }
}

/// Load a pathway's instances, filtered by the query's status lens.
pub(crate) async fn load_cohort(
    ctx: &AppContext,
    pathway_pid: Uuid,
    status: Option<&str>,
) -> Result<Vec<pathway_instances::Model>> {
    Ok(cohort_query(pathway_pid, status)
        .limit(MAX_COHORT_INSTANCES)
        .all(&ctx.db)
        .await?)
}

/// Build the full attrition array for one cohort response (spec
/// T-14g): the base six-step trail, plus the rule-based-split branch
/// (spec T-14f) when `plan` was resolved.
async fn build_attrition(
    ctx: &AppContext,
    template: &PathwayModel,
    query: &CohortQuery,
    analyses: &[tba::InstanceAnalysis],
    suppressed: bool,
    plan: Option<&SplitPlan>,
) -> Result<Vec<tba::AttritionStep>> {
    let (enrolled, after_status) =
        attrition_counts(ctx, template.pid, query.status.as_deref()).await?;
    let degenerate = analyses.iter().filter(|a| a.reason.is_some()).count();
    let status_label = query.status.as_deref().unwrap_or("all");
    let mut trail =
        tba::attrition_trail(enrolled, after_status, status_label, degenerate, suppressed);
    if let Some(plan) = plan {
        let rule_description = format!(
            "contains={} excludes={}",
            query.contains.as_deref().unwrap_or(""),
            query.excludes.as_deref().unwrap_or(""),
        );
        trail.extend(tba::attrition_rule_branch(
            &rule_description,
            plan.matched_instances.len(),
            plan.complement_instances.len(),
        ));
    }
    Ok(trail)
}

/// The two counts [`tba::attrition_trail`]'s first two steps need:
/// every non-deleted instance ever enrolled on this pathway, and how
/// many remain after the query's status lens — both **unbounded**
/// `COUNT` queries, not `instances.len()`, since the latter is capped
/// at [`MAX_COHORT_INSTANCES`] and would silently understate the true
/// population on a pathway large enough to hit that cap (spec T-14g).
async fn attrition_counts(
    ctx: &AppContext,
    pathway_pid: Uuid,
    status: Option<&str>,
) -> Result<(usize, usize)> {
    let enrolled = cohort_query(pathway_pid, None).count(&ctx.db).await?;
    let after_status = cohort_query(pathway_pid, status).count(&ctx.db).await?;
    Ok((
        usize::try_from(enrolled).unwrap_or(usize::MAX),
        usize::try_from(after_status).unwrap_or(usize::MAX),
    ))
}

/// Analyse a whole cohort in two bounded queries (no N+1): one for the
/// instances, one for all their segments.
pub(crate) async fn analyze_cohort(
    ctx: &AppContext,
    instances: &[pathway_instances::Model],
    as_of_ms: i64,
) -> Result<Vec<tba::InstanceAnalysis>> {
    if instances.is_empty() {
        return Ok(Vec::new());
    }
    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    let rows = instance_segments::Entity::find()
        .filter(instance_segments::Column::InstancePid.is_in(pids))
        .order_by_asc(instance_segments::Column::StartedAt)
        .all(&ctx.db)
        .await?;
    let mut per_instance: std::collections::HashMap<Uuid, Vec<tba::Segment>> =
        std::collections::HashMap::new();
    for row in &rows {
        per_instance
            .entry(row.instance_pid)
            .or_default()
            .push(to_segment(row));
    }
    Ok(instances
        .iter()
        .map(|instance| {
            let segments = per_instance
                .get(&instance.pid)
                .map_or::<&[tba::Segment], _>(&[], Vec::as_slice);
            tba::analyze(resolve_clock(instance, as_of_ms), segments, as_of_ms)
        })
        .collect())
}

/// Bulk-load each instance's [`split::Features`] for a rule-based
/// cohort split (spec T-14f), in the same order as `instances` — one
/// bounded query per source (segments, steps, events), no team-role
/// lookup (unlike [`crate::controllers::exports::load_event_log_inputs`],
/// which this deliberately does not reuse: rule matching needs no
/// actor/location data, only the same source rows).
async fn load_features(
    ctx: &AppContext,
    template: &PathwayModel,
    instances: &[pathway_instances::Model],
) -> Result<Vec<split::Features>> {
    if instances.is_empty() {
        return Ok(Vec::new());
    }
    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();

    let segment_rows = instance_segments::Entity::find()
        .filter(instance_segments::Column::InstancePid.is_in(pids.clone()))
        .all(&ctx.db)
        .await?;
    let mut segments: std::collections::HashMap<Uuid, Vec<analytics::SegmentInput>> =
        std::collections::HashMap::new();
    for row in &segment_rows {
        segments
            .entry(row.instance_pid)
            .or_default()
            .push(analytics::SegmentInput {
                stage: row.stage.clone(),
                category: row.category.clone(),
                waste: row.waste.clone(),
                start_ms: ms(row.started_at),
                end_ms: row.ended_at.map(ms),
                actor_ref: row.actor_ref.clone(),
                location_ref: row.location_ref.clone(),
            });
    }

    let step_rows = instance_steps::Entity::find()
        .filter(instance_steps::Column::InstancePid.is_in(pids.clone()))
        .all(&ctx.db)
        .await?;
    let mut steps: std::collections::HashMap<Uuid, Vec<analytics::StepInput>> =
        std::collections::HashMap::new();
    for row in &step_rows {
        steps
            .entry(row.instance_pid)
            .or_default()
            .push(analytics::StepInput {
                label: row.label.clone(),
                done_at_ms: row.done_on.map(date_ms),
            });
    }

    let event_rows = instance_events::Entity::find()
        .filter(instance_events::Column::InstancePid.is_in(pids))
        .all(&ctx.db)
        .await?;
    let mut events: std::collections::HashMap<Uuid, Vec<analytics::EventInput>> =
        std::collections::HashMap::new();
    for row in &event_rows {
        events
            .entry(row.instance_pid)
            .or_default()
            .push(analytics::EventInput {
                kind: row.kind.clone(),
                occurred_at_ms: ms(row.occurred_at),
            });
    }

    let pathway_dto = template.to_pathway()?;
    let care_setting = super::exports::care_setting_string(&pathway_dto);

    Ok(instances
        .iter()
        .map(|instance| {
            let case_ctx = analytics::CaseContext {
                case_id: instance.pid.to_string(),
                pathway_pid: template.pid.to_string(),
                care_setting: care_setting.clone(),
                urgency: instance.urgency.clone(),
                status: instance.status.clone(),
                outcome: instance.outcome.clone(),
            };
            split::features_of(
                &case_ctx,
                segments
                    .get(&instance.pid)
                    .map_or::<&[analytics::SegmentInput], _>(&[], Vec::as_slice),
                steps
                    .get(&instance.pid)
                    .map_or::<&[analytics::StepInput], _>(&[], Vec::as_slice),
                events
                    .get(&instance.pid)
                    .map_or::<&[analytics::EventInput], _>(&[], Vec::as_slice),
            )
        })
        .collect())
}

/// Parse the query's `contains=`/`excludes=` into a [`split::Rule`],
/// `422`-refusing a malformed or unrecognised predicate rather than
/// silently matching nothing (spec T-14f).
fn resolve_rule(query: &CohortQuery) -> Result<split::Rule> {
    split::Rule::parse(query.contains.as_deref(), query.excludes.as_deref())
        .map_err(|reason| refuse(&reason))
}

/// A resolved rule-based cohort split (spec T-14f): each side's own
/// `instances`/`analyses` slice, and whether each side's *detail*
/// should render — decided once from [`split::split_table`] so
/// `time-analysis` and `constraints` can never disagree on which side
/// is protected.
struct SplitPlan {
    compare: bool,
    matched_instances: Vec<pathway_instances::Model>,
    matched_analyses: Vec<tba::InstanceAnalysis>,
    matched_detail_suppressed: bool,
    complement_instances: Vec<pathway_instances::Model>,
    complement_analyses: Vec<tba::InstanceAnalysis>,
    complement_detail_suppressed: bool,
}

/// Build the split plan for one cohort request (spec T-14f), or
/// `None` when there is nothing to split: the query names neither
/// `contains=` nor `excludes=` (the identity rule — today's unsplit
/// response, unchanged), or the *unsplit* cohort is already below the
/// suppression floor, in which case a breakdown of an already-too-small
/// cohort would disclose more, not less, so it is skipped rather than
/// attempted.
async fn resolve_split(
    ctx: &AppContext,
    template: &PathwayModel,
    instances: &[pathway_instances::Model],
    analyses: &[tba::InstanceAnalysis],
    unsplit_suppressed: bool,
    query: &CohortQuery,
) -> Result<Option<SplitPlan>> {
    let rule = resolve_rule(query)?;
    if rule.is_identity() || unsplit_suppressed {
        return Ok(None);
    }
    let features = load_features(ctx, template, instances).await?;
    let (matched_idx, complement_idx) = split::partition(&rule, &features);
    let pick = |indices: &[usize]| -> (Vec<pathway_instances::Model>, Vec<tba::InstanceAnalysis>) {
        indices
            .iter()
            .map(|&i| (instances[i].clone(), analyses[i].clone()))
            .unzip()
    };
    let (matched_instances, matched_analyses) = pick(&matched_idx);
    let (complement_instances, complement_analyses) = pick(&complement_idx);

    let compare = query.compare.unwrap_or(false);
    let (matched_detail_suppressed, complement_detail_suppressed) = if compare {
        // The complement is about to be shown too, so a suppressed
        // matched side's sums (by_stage, by_waste, …) would be exactly
        // `unsplit - complement` unless the complement's detail is
        // withheld as well (spec T-14k) — decide() applies that
        // recruitment automatically over the two published counts.
        let table = split::split_table(matched_instances.len(), complement_instances.len());
        let suppressed = suppression::decide(&table, suppression::min_cell_count());
        (suppressed.contains(&0), suppressed.contains(&1))
    } else {
        // The complement is never shown at all, so there is nothing
        // to difference against — only the matched side's own count
        // governs its own detail.
        (suppression::is_suppressed(matched_instances.len()), false)
    };

    Ok(Some(SplitPlan {
        compare,
        matched_instances,
        matched_analyses,
        matched_detail_suppressed,
        complement_instances,
        complement_analyses,
        complement_detail_suppressed,
    }))
}

/// One split side's `time-analysis`-shaped payload: the same
/// `cohort`/`compliance`/`survival` blocks the unsplit response
/// already carries, or all three withheld together when this side's
/// own detail is suppressed.
fn split_side_payload(
    instances: &[pathway_instances::Model],
    analyses: &[tba::InstanceAnalysis],
    detail_suppressed: bool,
    mode: suppression::Mode,
    query: &CohortQuery,
) -> Result<serde_json::Value> {
    let n = instances.len();
    if !detail_suppressed {
        let summary = tba::cohort(analyses);
        let compliance = score_compliance(analyses, query)?;
        let survival = survival_analysis(instances, analyses, query)?;
        return Ok(serde_json::json!({
            "instances": n,
            "suppressed": false,
            "suppression_note": serde_json::Value::Null,
            "cohort": summary,
            "compliance": compliance,
            "survival": survival,
        }));
    }
    let mut value = serde_json::json!({
        "instances": n,
        "suppressed": true,
        "suppression_note": suppression::SUPPRESSED_REASON,
        "cohort": serde_json::Value::Null,
        "compliance": serde_json::Value::Null,
        "survival": serde_json::Value::Null,
    });
    if mode == suppression::Mode::Remove
        && let Some(map) = value.as_object_mut()
    {
        map.remove("cohort");
        map.remove("compliance");
        map.remove("survival");
    }
    Ok(value)
}

/// The `type:value` tokens in a raw `contains=`/`excludes=` value, for
/// echoing back verbatim (each token already validated during
/// [`resolve_rule`], so this is display only, not a second parse).
fn echo_tokens(raw: Option<&str>) -> Vec<&str> {
    raw.map(|raw| {
        raw.split(',')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

/// The whole `split` block for a `time-analysis` response (spec
/// T-14f): the rule echoed back, the matched side's payload, and the
/// complement's alongside it only when `?compare=true`.
fn split_payload(
    plan: &SplitPlan,
    mode: suppression::Mode,
    query: &CohortQuery,
) -> Result<serde_json::Value> {
    let matched = split_side_payload(
        &plan.matched_instances,
        &plan.matched_analyses,
        plan.matched_detail_suppressed,
        mode,
        query,
    )?;
    let complement = if plan.compare {
        Some(split_side_payload(
            &plan.complement_instances,
            &plan.complement_analyses,
            plan.complement_detail_suppressed,
            mode,
            query,
        )?)
    } else {
        None
    };
    Ok(serde_json::json!({
        "rule": {
            "contains": echo_tokens(query.contains.as_deref()),
            "excludes": echo_tokens(query.excludes.as_deref()),
        },
        "matched": matched,
        "complement": complement,
    }))
}

/// One split side's `constraints`-shaped payload: the same `findings`
/// block the unsplit response already carries, or withheld when this
/// side's own detail is suppressed. Infallible, unlike
/// [`split_side_payload`] — findings need no standard lookup.
fn split_side_findings(
    analyses: &[tba::InstanceAnalysis],
    detail_suppressed: bool,
    mode: suppression::Mode,
) -> serde_json::Value {
    let n = analyses.len();
    if !detail_suppressed {
        let summary = tba::cohort(analyses);
        let findings = tba::constraints(analyses, &summary);
        return serde_json::json!({
            "instances": n,
            "suppressed": false,
            "suppression_note": serde_json::Value::Null,
            "findings": findings,
        });
    }
    let mut value = serde_json::json!({
        "instances": n,
        "suppressed": true,
        "suppression_note": suppression::SUPPRESSED_REASON,
        "findings": serde_json::Value::Null,
    });
    if mode == suppression::Mode::Remove
        && let Some(map) = value.as_object_mut()
    {
        map.remove("findings");
    }
    value
}

/// The whole `split` block for a `constraints` response (spec T-14f).
fn split_payload_constraints(
    plan: &SplitPlan,
    mode: suppression::Mode,
    query: &CohortQuery,
) -> serde_json::Value {
    let matched = split_side_findings(&plan.matched_analyses, plan.matched_detail_suppressed, mode);
    let complement = plan.compare.then(|| {
        split_side_findings(
            &plan.complement_analyses,
            plan.complement_detail_suppressed,
            mode,
        )
    });
    serde_json::json!({
        "rule": {
            "contains": echo_tokens(query.contains.as_deref()),
            "excludes": echo_tokens(query.excludes.as_deref()),
        },
        "matched": matched,
        "complement": complement,
    })
}

/// Look up the query's named standard, `422`-refusing an unrecognised
/// id. `None` when the query names no standard at all (the
/// `target_days` path).
fn resolve_standard(query: &CohortQuery) -> Result<Option<&'static tba::Standard>> {
    query
        .standard
        .as_deref()
        .map(|id| {
            tba::standard(id).ok_or_else(|| {
                refuse(&format!(
                    "unknown standard `{id}` (standards: {:?})",
                    tba::STANDARDS.iter().map(|s| s.id).collect::<Vec<_>>()
                ))
            })
        })
        .transpose()
}

/// Score a plain (whole-clock) lead-time sample against an already
/// resolved standard, or `query.target_days`.
fn score_whole_clock(
    lead_times: &[i64],
    query: &CohortQuery,
    standard: Option<&'static tba::Standard>,
) -> Result<Option<tba::Compliance>> {
    if let Some(standard) = standard {
        return Ok(Some(tba::compliance(
            lead_times,
            standard.id,
            standard.threshold_ms,
            Some(standard.target_ratio),
            Some(standard.as_of),
        )));
    }
    if let Some(days) = query.target_days {
        if !days.is_finite() || days <= 0.0 {
            return Err(refuse("target_days must be a positive number"));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        // bounded by the finiteness + positivity check above
        let threshold_ms = (days * tba::DAY_MS as f64) as i64;
        return Ok(Some(tba::compliance(
            lead_times,
            "custom",
            threshold_ms,
            None,
            None,
        )));
    }
    Ok(None)
}

/// Score a per-instance anchored-interval sample (spec T-14d) against
/// an already resolved standard, or `query.target_days` — an unreached
/// anchor pair (`None`) counts against
/// [`tba::Compliance::unreached`] rather than a breach.
fn score_anchored(
    intervals_ms: &[Option<i64>],
    query: &CohortQuery,
    standard: Option<&'static tba::Standard>,
) -> Result<Option<tba::Compliance>> {
    if let Some(standard) = standard {
        return Ok(Some(tba::anchored_compliance(
            intervals_ms,
            standard.id,
            standard.threshold_ms,
            Some(standard.target_ratio),
            Some(standard.as_of),
        )));
    }
    if let Some(days) = query.target_days {
        if !days.is_finite() || days <= 0.0 {
            return Err(refuse("target_days must be a positive number"));
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        // bounded by the finiteness + positivity check above
        let threshold_ms = (days * tba::DAY_MS as f64) as i64;
        return Ok(Some(tba::anchored_compliance(
            intervals_ms,
            "custom",
            threshold_ms,
            None,
            None,
        )));
    }
    Ok(None)
}

/// Validate the query's `from_anchor`/`to_anchor` pair against
/// [`tba::STAGES`] (spec T-14d). `Ok(Some((from, to)))` when both are
/// given and recognised; `Ok(None)` when neither is given (nothing to
/// override — the caller falls through to whatever the requested
/// standard itself declares, or whole-clock); `Err(reason)` — never a
/// hard failure — when exactly one is given, or either name is not a
/// recognised stage, so the caller falls all the way back to the
/// whole-clock figure with a disclosed reason rather than
/// approximating a scoring it didn't actually ask for.
fn resolve_anchor_pair(
    query: &CohortQuery,
) -> std::result::Result<Option<(&str, &str)>, &'static str> {
    match (query.from_anchor.as_deref(), query.to_anchor.as_deref()) {
        (None, None) => Ok(None),
        (Some(from), Some(to)) => {
            if !tba::STAGES.contains(&from) {
                Err("unknown_from_anchor")
            } else if !tba::STAGES.contains(&to) {
                Err("unknown_to_anchor")
            } else {
                Ok(Some((from, to)))
            }
        }
        (Some(_), None) => Err("to_anchor_missing"),
        (None, Some(_)) => Err("from_anchor_missing"),
    }
}

/// Score the cohort's compliance (spec T-14d). Precedence, most to
/// least specific: (1) an explicit, valid `?from_anchor=&to_anchor=`
/// pair always wins, even over a standard's own declared anchor, so a
/// caller can be more specific than the catalogue; (2) naming neither
/// falls through to the requested standard's own `from_anchor`/
/// `to_anchor` — e.g. `cancer_fds_28_days` scores referral ->
/// diagnostics without the caller asking for it explicitly; (3)
/// neither the query nor the standard declaring one leaves the
/// whole-clock path — today's behaviour — untouched, which is every
/// other catalogue entry. An explicit pair that fails validation (only
/// one side given, or an unrecognised stage name) never silently
/// reverts to the standard's own anchor: it always falls all the way
/// to whole-clock, with `compliance.anchor_note` disclosing why.
fn score_compliance(
    analyses: &[tba::InstanceAnalysis],
    query: &CohortQuery,
) -> Result<Option<tba::Compliance>> {
    let standard = resolve_standard(query)?;
    let lead_times = || analyses.iter().map(|a| a.lead_time_ms).collect::<Vec<_>>();
    let anchored = |from: &str, to: &str| -> Vec<Option<i64>> {
        analyses
            .iter()
            .map(|a| tba::anchor_interval(&a.anchors, from, to))
            .collect()
    };

    match resolve_anchor_pair(query) {
        Ok(Some((from, to))) => score_anchored(&anchored(from, to), query, standard),
        Ok(None) => {
            if let Some(s) = standard
                && let (Some(from), Some(to)) = (s.from_anchor, s.to_anchor)
            {
                return score_anchored(&anchored(from, to), query, standard);
            }
            score_whole_clock(&lead_times(), query, standard)
        }
        Err(reason) => {
            let mut compliance = score_whole_clock(&lead_times(), query, standard)?;
            if let Some(c) = compliance.as_mut() {
                c.anchor_note = Some(reason.to_string());
            }
            Ok(compliance)
        }
    }
}

/// Both Kaplan–Meier blocks for one cohort (spec T-14e).
#[derive(Debug, serde::Serialize)]
struct Survival {
    /// Time-to-close: every instance contributes, closed or still
    /// open, so `?status=all`'s mixing problem (a running lead time
    /// read as if it were a finished one) is what this whole block
    /// exists to correct.
    time_to_close: tba::KaplanMeier,
    /// Time-to-anchor for the query's `from_anchor`/`to_anchor` pair
    /// (reusing T-14d's own mechanism) — `None` when the query names
    /// no such pair, or names one that does not validate; an invalid
    /// pair is already surfaced via `compliance.anchor_note`, so this
    /// block simply omits the block it cannot honestly compute rather
    /// than repeating that disclosure.
    time_to_anchor: Option<tba::KaplanMeier>,
    /// `event` | `censor` — which way `discontinued` closures were
    /// counted in `time_to_close` (and in `time_to_anchor`, for an
    /// instance that discontinued before reaching either anchor).
    discontinued: &'static str,
}

/// Parse the query's `discontinued` parameter (spec T-14e): `event`
/// (default) or `censor`, `422`-refusing anything else rather than
/// silently defaulting a typo'd value.
fn resolve_discontinued(query: &CohortQuery) -> Result<(bool, &'static str)> {
    match query.discontinued.as_deref() {
        None | Some("event") => Ok((true, "event")),
        Some("censor") => Ok((false, "censor")),
        Some(other) => Err(refuse(&format!(
            "unknown discontinued mode `{other}` (expected `event` or `censor`)"
        ))),
    }
}

/// Build both Kaplan–Meier blocks (spec T-14e). `instances` and
/// `analyses` are zipped positionally — both come from the same
/// `analyze_cohort` call over the same `instances` slice, so the
/// pairing is exact.
fn survival_analysis(
    instances: &[pathway_instances::Model],
    analyses: &[tba::InstanceAnalysis],
    query: &CohortQuery,
) -> Result<Survival> {
    let (discontinued_is_event, discontinued_label) = resolve_discontinued(query)?;

    let close_observations: Vec<tba::Observation> = instances
        .iter()
        .zip(analyses)
        .map(|(instance, analysis)| {
            tba::close_observation(analysis, &instance.status, discontinued_is_event)
        })
        .collect();

    let time_to_anchor = match resolve_anchor_pair(query) {
        Ok(Some((from, to))) => {
            let observations: Vec<tba::Observation> = analyses
                .iter()
                .filter_map(|a| tba::anchor_observation(a, from, to))
                .collect();
            Some(tba::kaplan_meier(&observations))
        }
        _ => None,
    };

    Ok(Survival {
        time_to_close: tba::kaplan_meier(&close_observations),
        time_to_anchor,
        discontinued: discontinued_label,
    })
}

/// `GET /api/care-pathways/{pathway}/time-analysis` — the cohort view
/// (spec §7).
#[debug_handler]
async fn cohort_time_analysis(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<CohortQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let as_of_ms = now.timestamp_millis();
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let analyses = analyze_cohort(&ctx, &instances, as_of_ms).await?;
    let summary = tba::cohort(&analyses);
    let compliance = score_compliance(&analyses, &query)?;
    let survival = survival_analysis(&instances, &analyses, &query)?;

    // Small-number suppression (spec §12.2, generalised T-14k): below
    // the deployment's cell-count floor the percentile detail would
    // isolate an individual patient, so the counts and the ranking are
    // returned without it. `?mode=remove` drops the key entirely
    // instead of nulling it; either way the *decision* is the same
    // (`suppression::is_suppressed`), so the two can never disagree on
    // whether this cohort is small.
    let suppressed = suppression::is_suppressed(summary.instances);
    let mode = suppression::Mode::parse(query.mode.as_deref());
    let mut body = serde_json::to_value(&summary).unwrap_or_else(|_| serde_json::json!({}));
    if suppressed && let Some(map) = body.as_object_mut() {
        match mode {
            suppression::Mode::Withhold => {
                map.insert("lead_time".to_string(), serde_json::Value::Null);
            }
            suppression::Mode::Remove => {
                map.remove("lead_time");
            }
        }
    }

    let mut response = serde_json::json!({
        "as_of": now,
        "pathway": { "pid": template.pid, "name": template.name },
        "note": "lead-time percentiles are nearest-rank, so every one is an \
                 observed journey; the mean is reported but is skew-sensitive. \
                 A divergence between aggregate and median value-adding ratio \
                 is itself the finding — `concentrated` means the waste sits in \
                 a minority of journeys.",
        "suppressed": suppressed,
        "suppression_note": suppressed.then_some(suppression::SUPPRESSED_REASON),
        "cohort": body,
        "compliance": compliance,
        "survival": survival,
    });
    // The survival curves carry the identical disclosure risk the
    // percentile detail above does — a handful of instances' own
    // closure/anchor times, not an aggregate — so they are withheld
    // under the same suppression decision, never a separate one.
    if suppressed && let Some(map) = response.as_object_mut() {
        match mode {
            suppression::Mode::Withhold => {
                map.insert("survival".to_string(), serde_json::Value::Null);
            }
            suppression::Mode::Remove => {
                map.remove("survival");
            }
        }
    }

    // Rule-based cohort split (spec T-14f): absent unless the query
    // names `contains=`/`excludes=` and the unsplit cohort itself
    // clears the floor (see resolve_split's own doc for why).
    let plan = resolve_split(&ctx, &template, &instances, &analyses, suppressed, &query).await?;
    if let Some(plan) = &plan
        && let Some(map) = response.as_object_mut()
    {
        map.insert("split".to_string(), split_payload(plan, mode, &query)?);
    }

    // Cohort attrition record (spec T-14g): the denominator explained
    // inside the response, not merely stated.
    let attrition = build_attrition(
        &ctx,
        &template,
        &query,
        &analyses,
        suppressed,
        plan.as_ref(),
    )
    .await?;
    if let Some(map) = response.as_object_mut() {
        map.insert(
            "attrition".to_string(),
            serde_json::to_value(&attrition).unwrap_or(serde_json::Value::Null),
        );
    }

    format::json(response)
}

/// `GET /api/care-pathways/{pathway}/constraints` — the ranked
/// constraints (spec §8).
#[debug_handler]
async fn cohort_constraints(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<CohortQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let as_of_ms = now.timestamp_millis();
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let analyses = analyze_cohort(&ctx, &instances, as_of_ms).await?;
    let summary = tba::cohort(&analyses);
    let findings = tba::constraints(&analyses, &summary);

    // Small-number suppression (spec T-14k): a constraint finding
    // names a rule and a threshold computed over the whole cohort, so
    // at a low `n` it can describe one patient's journey precisely —
    // the same disclosure risk `cohort_time_analysis`'s percentiles
    // carry, closed here for the first time (this endpoint previously
    // returned `findings` unsuppressed at any cohort size).
    let suppressed = suppression::is_suppressed(summary.instances);
    let mode = suppression::Mode::parse(query.mode.as_deref());
    let mut body = serde_json::json!({
        "as_of": now,
        "pathway": { "pid": template.pid, "name": template.name },
        "note": "findings ordered by recoverable time; each names the rule that \
                 produced it and the threshold that fired. Deliberately not a \
                 composite score, and deliberately never per-clinician.",
        "instances": summary.instances,
        "suppressed": suppressed,
        "suppression_note": suppressed.then_some(suppression::SUPPRESSED_REASON),
        "findings": findings,
    });
    if suppressed && let Some(map) = body.as_object_mut() {
        match mode {
            suppression::Mode::Withhold => {
                map.insert("findings".to_string(), serde_json::Value::Null);
            }
            suppression::Mode::Remove => {
                map.remove("findings");
            }
        }
    }

    // Rule-based cohort split (spec T-14f) — same precondition and
    // suppression decision as `cohort_time_analysis`'s own split.
    let plan = resolve_split(&ctx, &template, &instances, &analyses, suppressed, &query).await?;
    if let Some(plan) = &plan
        && let Some(map) = body.as_object_mut()
    {
        map.insert(
            "split".to_string(),
            split_payload_constraints(plan, mode, &query),
        );
    }

    // Cohort attrition record (spec T-14g) — same trail as
    // `cohort_time_analysis`'s own.
    let attrition = build_attrition(
        &ctx,
        &template,
        &query,
        &analyses,
        suppressed,
        plan.as_ref(),
    )
    .await?;
    if let Some(map) = body.as_object_mut() {
        map.insert(
            "attrition".to_string(),
            serde_json::to_value(&attrition).unwrap_or(serde_json::Value::Null),
        );
    }

    format::json(body)
}

/// `?level=` + the shared cohort status/mode query (spec T-14b).
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct ProcessMapQuery {
    /// `stage` (default) | `step`.
    #[serde(default)]
    level: Option<String>,
    /// `open` | `closed` | `all` (default `all`).
    #[serde(default)]
    status: Option<String>,
    /// `withhold` (default) | `remove` — how a below-floor node/edge
    /// renders (spec T-14k).
    #[serde(default)]
    mode: Option<String>,
}

/// Bulk-load one cohort's segments as [`analytics::SegmentInput`]s, one
/// query, grouped by instance pid — the stage-level input to
/// [`analytics::process_map_sequence_from_segments`].
async fn load_segment_inputs(
    ctx: &AppContext,
    pids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<analytics::SegmentInput>>> {
    let mut map = std::collections::HashMap::new();
    if pids.is_empty() {
        return Ok(map);
    }
    let rows = instance_segments::Entity::find()
        .filter(instance_segments::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_segments::Column::StartedAt)
        .all(&ctx.db)
        .await?;
    for row in &rows {
        map.entry(row.instance_pid)
            .or_insert_with(Vec::new)
            .push(analytics::SegmentInput {
                stage: row.stage.clone(),
                category: row.category.clone(),
                waste: row.waste.clone(),
                start_ms: ms(row.started_at),
                end_ms: row.ended_at.map(ms),
                actor_ref: row.actor_ref.clone(),
                location_ref: row.location_ref.clone(),
            });
    }
    Ok(map)
}

/// Bulk-load one cohort's steps as [`analytics::StepInput`]s, one
/// query, grouped by instance pid — the step-level input to
/// [`analytics::process_map_sequence_from_steps`].
async fn load_step_inputs(
    ctx: &AppContext,
    pids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<analytics::StepInput>>> {
    let mut map = std::collections::HashMap::new();
    if pids.is_empty() {
        return Ok(map);
    }
    let rows = instance_steps::Entity::find()
        .filter(instance_steps::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_steps::Column::Position)
        .all(&ctx.db)
        .await?;
    for row in &rows {
        map.entry(row.instance_pid)
            .or_insert_with(Vec::new)
            .push(analytics::StepInput {
                label: row.label.clone(),
                done_at_ms: row.done_on.map(date_ms),
            });
    }
    Ok(map)
}

/// Build every instance's bookended activity sequence at the requested
/// level, in one bulk load (no N+1).
async fn cohort_sequences(
    ctx: &AppContext,
    instances: &[pathway_instances::Model],
    level: &str,
    as_of_ms: i64,
) -> Result<Vec<Vec<analytics::ActivityStep>>> {
    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    if level == "step" {
        let mut by_instance = load_step_inputs(ctx, &pids).await?;
        Ok(instances
            .iter()
            .map(|instance| {
                let clock = resolve_clock(instance, as_of_ms);
                let steps = by_instance.remove(&instance.pid).unwrap_or_default();
                analytics::process_map_sequence_from_steps(clock.start_ms, clock.stop_ms, &steps)
            })
            .collect())
    } else {
        let mut by_instance = load_segment_inputs(ctx, &pids).await?;
        Ok(instances
            .iter()
            .map(|instance| {
                let clock = resolve_clock(instance, as_of_ms);
                let segments = by_instance.remove(&instance.pid).unwrap_or_default();
                analytics::process_map_sequence_from_segments(
                    clock.start_ms,
                    clock.stop_ms,
                    &segments,
                )
            })
            .collect())
    }
}

/// Render one node/edge, applying the small-number floor (spec T-14k):
/// below it, `Mode::Withhold` nulls the counts/duration and carries a
/// reason; `Mode::Remove` omits the entry entirely. Never a silent
/// zero either way.
fn render_process_map_entry<T: serde::Serialize>(
    value: &T,
    instance_count: usize,
    null_keys: &[&str],
    mode: suppression::Mode,
) -> Option<serde_json::Value> {
    let suppressed = suppression::is_suppressed(instance_count);
    if suppressed && mode == suppression::Mode::Remove {
        return None;
    }
    let mut rendered = serde_json::to_value(value).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(map) = rendered.as_object_mut() {
        if suppressed {
            for key in null_keys {
                map.insert((*key).to_string(), serde_json::Value::Null);
            }
            map.insert(
                "suppression_note".to_string(),
                serde_json::json!(suppression::SUPPRESSED_REASON),
            );
        }
        map.insert("suppressed".to_string(), serde_json::json!(suppressed));
    }
    Some(rendered)
}

/// `GET /api/care-pathways/{pathway}/process-map` — the directly-follows
/// process map (spec T-14b). Never a discovered model: nodes and edges
/// are exactly the observed activities and transitions.
#[debug_handler]
async fn process_map(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<ProcessMapQuery>,
) -> Result<Response> {
    let level = match query.level.as_deref() {
        None | Some("stage") => "stage",
        Some("step") => "step",
        Some(other) => {
            return Err(refuse(&format!(
                "level must be \"stage\" or \"step\", got \"{other}\""
            )));
        }
    };
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let as_of_ms = chrono::Utc::now().timestamp_millis();
    let sequences = cohort_sequences(&ctx, &instances, level, as_of_ms).await?;
    let map = analytics::build_process_map(&sequences);

    let mode = suppression::Mode::parse(query.mode.as_deref());
    let nodes: Vec<serde_json::Value> = map
        .nodes
        .iter()
        .filter_map(|node| {
            render_process_map_entry(
                node,
                node.instance_count,
                &["instance_count", "occurrence_count", "median_duration_days"],
                mode,
            )
        })
        .collect();
    let edges: Vec<serde_json::Value> = map
        .edges
        .iter()
        .filter_map(|edge| {
            render_process_map_entry(
                edge,
                edge.instance_count,
                &[
                    "instance_count",
                    "occurrence_count",
                    "median_gap_days",
                    "p90_gap_days",
                ],
                mode,
            )
        })
        .collect();

    format::json(serde_json::json!({
        "pathway": { "pid": template.pid, "name": template.name },
        "level": level,
        "instances": instances.len(),
        "note": "directly-follows only — never a discovered model. Self-loops are \
                 kept: a return to a stage/step is a finding. `start`/`end` \
                 pseudo-nodes make entry/exit variety visible. At `level=step`, \
                 `done_on` is a date, so a same-day pair is a 0-day edge. Nodes \
                 and edges below the minimum cell count are withheld (T-14k), \
                 never shown as zero.",
        "nodes": nodes,
        "edges": edges,
    }))
}

/// `?min_segment_days=&collapse_gap_days=&combination_window_days=&min_post_combination_days=&filter=&max_path_length=`
/// plus the shared cohort `status` (spec T-14c).
#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct VariantsQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    min_segment_days: Option<f64>,
    #[serde(default)]
    collapse_gap_days: Option<f64>,
    #[serde(default)]
    combination_window_days: Option<f64>,
    #[serde(default)]
    min_post_combination_days: Option<f64>,
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    max_path_length: Option<usize>,
}

fn variant_params_from_query(query: &VariantsQuery) -> Result<variants::VariantParams> {
    let filter = variants::FilterMode::parse(query.filter.as_deref()).map_err(|e| refuse(&e))?;
    Ok(variants::VariantParams {
        min_segment_days: query
            .min_segment_days
            .unwrap_or(variants::DEFAULT_MIN_SEGMENT_DAYS),
        collapse_gap_days: query
            .collapse_gap_days
            .unwrap_or(variants::DEFAULT_COLLAPSE_GAP_DAYS),
        combination_window_days: query
            .combination_window_days
            .unwrap_or(variants::DEFAULT_COMBINATION_WINDOW_DAYS),
        min_post_combination_days: query
            .min_post_combination_days
            .unwrap_or(variants::DEFAULT_MIN_POST_COMBINATION_DAYS),
        filter,
        max_path_length: query
            .max_path_length
            .unwrap_or(variants::DEFAULT_MAX_PATH_LENGTH),
    })
}

/// `GET /api/care-pathways/{pathway}/variants` — journey variants
/// (pathway strings), the frequency/coverage Pareto, and per-position
/// duration lines (spec T-14c).
#[debug_handler]
async fn variants_endpoint(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<VariantsQuery>,
) -> Result<Response> {
    let params = variant_params_from_query(&query)?;
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let as_of_ms = chrono::Utc::now().timestamp_millis();
    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    let mut segments_by_instance = load_segment_inputs(&ctx, &pids).await?;

    let instance_variants: Vec<variants::InstanceVariant> = instances
        .iter()
        .map(|instance| {
            let segments = segments_by_instance
                .remove(&instance.pid)
                .unwrap_or_default();
            variants::build_variant(&segments, as_of_ms, &params)
        })
        .collect();
    let report = variants::summarize_variants(&instance_variants, suppression::min_cell_count());

    format::json(serde_json::json!({
        "pathway": { "pid": template.pid, "name": template.name },
        "instances": report.instances,
        "suppressed_instances": report.suppressed_instances,
        "params": {
            "min_segment_days": params.min_segment_days,
            "collapse_gap_days": params.collapse_gap_days,
            "combination_window_days": params.combination_window_days,
            "min_post_combination_days": params.min_post_combination_days,
            "filter": params.filter.as_str(),
            "max_path_length": params.max_path_length,
        },
        "note": "never a discovered model. A combination step is a canonical \
                 alphabetical a+b (or a+b+c) join; a short overlap is a handoff, \
                 attributed to the incoming stage instead. Variants below the \
                 minimum cell count are folded into suppressed_instances, never \
                 listed individually — visible variants' shares are renormalised \
                 so they alone sum to 1.0.",
        "variants": report.variants,
        "lines": report.lines,
    }))
}

/// `GET /api/instances/time-standards` — the standards catalogue
/// (spec §7.3).
#[debug_handler]
async fn time_standards() -> Result<Response> {
    format::json(serde_json::json!({
        "note": "reference data with a citation date, not an assertion that any \
                 given pathway is subject to these. Pass ?standard=<id> to a \
                 cohort time-analysis, or ?target_days= for a local promise.",
        "standards": tba::STANDARDS,
        "vocabularies": {
            "stages": tba::STAGES,
            "categories": tba::CATEGORIES,
            "wastes": tba::WASTES,
        },
    }))
}

/// Flow query parameters.
#[derive(Debug, serde::Deserialize)]
struct FlowQuery {
    #[serde(default)]
    window_days: Option<i64>,
    /// Restrict to one pathway template.
    #[serde(default)]
    pathway: Option<String>,
}

/// `GET /api/instances/flow` — queueing-theory flow analysis (spec §9).
#[debug_handler]
async fn flow(State(ctx): State<AppContext>, Query(query): Query<FlowQuery>) -> Result<Response> {
    let now = chrono::Utc::now();
    let window_days = query.window_days.unwrap_or(DEFAULT_WINDOW_DAYS);
    if window_days <= 0 || window_days > 3650 {
        return Err(refuse("window_days must be between 1 and 3650"));
    }
    let since = now.date_naive() - chrono::Duration::days(window_days);

    let mut find =
        pathway_instances::Entity::find().filter(pathway_instances::Column::DeletedAt.is_null());
    if let Some(raw) = query.pathway.as_deref() {
        let template = PathwayModel::find_by_pid(&ctx.db, raw)
            .await
            .map_err(|_| Error::NotFound)?;
        find = find.filter(pathway_instances::Column::PathwayPid.eq(template.pid));
    }
    let instances = find.limit(MAX_COHORT_INSTANCES).all(&ctx.db).await?;

    let arrivals = instances.iter().filter(|i| i.enrolled_on >= since).count();
    let closed_in_window: Vec<&pathway_instances::Model> = instances
        .iter()
        .filter(|i| i.closed_on.is_some_and(|day| day >= since))
        .collect();
    let work_in_progress = instances
        .iter()
        .filter(|i| !rules::is_terminal(&i.status))
        .count();

    // Observed median lead time of the journeys that actually finished
    // in the window — the figure Little's Law is checked against.
    let as_of_ms = now.timestamp_millis();
    let mut observed: Vec<i64> = closed_in_window
        .iter()
        .map(|i| resolve_clock(i, as_of_ms).lead_time_ms())
        .collect();
    observed.sort_unstable();
    let observed_p50 = tba::percentile(&observed, 0.50);

    let analysis = tba::flow(
        window_days,
        arrivals,
        closed_in_window.len(),
        work_in_progress,
        observed_p50,
    );
    format::json(serde_json::json!({
        "as_of": now,
        "window_since": since,
        "note": "Little's Law κ = λτ used as a consistency check on observed \
                 figures, not as a forecast. It assumes arrivals and departures \
                 balance over the window, so a short window on a volatile \
                 pathway gives an implied lead time that should not be quoted. \
                 Utilisation is reported here rather than in a separate capacity \
                 view because expected wait grows without bound as it approaches \
                 1 — a pathway at 95% is not 5% from trouble, it is already in it.",
        "instances_considered": instances.len(),
        "flow": analysis,
    }))
}

// ---------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------

/// Instance-scoped TBA routes (prefix `/api/instances`). The literal
/// `flow` / `time-standards` paths are declared before the `{pid}`
/// captures so they are not swallowed by them.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/instances")
        .add("/flow", get(flow))
        .add("/time-standards", get(time_standards))
        .add("/{pid}/segments", post(record_segment))
        .add("/{pid}/segments", get(list_segments))
        .add("/{pid}/segments/{seg}/close", post(close_segment))
        .add("/{pid}/clock", post(set_clock))
        .add("/{pid}/time-analysis", get(instance_time_analysis))
        .add("/{pid}/timeline", get(instance_timeline))
}

/// Pathway-scoped TBA routes (prefix `/api/care-pathways`), added
/// before the registry's `/{pid}` capture so the literal sub-paths win.
pub fn pathway_routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/{pathway}/time-analysis", get(cohort_time_analysis))
        .add("/{pathway}/constraints", get(cohort_constraints))
        .add("/{pathway}/process-map", get(process_map))
        .add("/{pathway}/variants", get(variants_endpoint))
}
