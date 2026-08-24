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

use crate::auth::MaybeAuthUser;
use crate::instances as rules;
use crate::models::_entities::{instance_segments, pathway_instances};
use crate::models::audit_logs::Model as Audit;
use crate::models::care_pathways::Model as PathwayModel;
use crate::tba;

/// Cohort reads are capped so an unbounded cohort cannot become an
/// unbounded query (security invariant 3; spec §11).
const MAX_COHORT_INSTANCES: u64 = 1000;

/// Per-instance segment cap, for the same reason.
const MAX_SEGMENTS: u64 = 5000;

/// The window default for flow analysis (spec §9, §17 — arbitrary, and
/// documented as such).
const DEFAULT_WINDOW_DAYS: i64 = 90;

/// The smallest cohort whose percentile detail is disclosed. Below it a
/// percentile isolates an individual patient by arithmetic (spec §12.2).
const MIN_COHORT_FOR_PERCENTILES: usize = 5;

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
}

/// Load a pathway's instances, filtered by the query's status lens.
pub(crate) async fn load_cohort(
    ctx: &AppContext,
    pathway_pid: Uuid,
    status: Option<&str>,
) -> Result<Vec<pathway_instances::Model>> {
    let mut query = pathway_instances::Entity::find()
        .filter(pathway_instances::Column::PathwayPid.eq(pathway_pid))
        .filter(pathway_instances::Column::DeletedAt.is_null());
    query = match status {
        Some("open") => {
            query.filter(pathway_instances::Column::Status.is_in(["active", "on_hold"]))
        }
        Some("closed") => {
            query.filter(pathway_instances::Column::Status.is_in(["completed", "discontinued"]))
        }
        _ => query,
    };
    Ok(query.limit(MAX_COHORT_INSTANCES).all(&ctx.db).await?)
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

/// Resolve the requested standard or explicit target into a compliance
/// score over the cohort's lead times.
fn score_compliance(lead_times: &[i64], query: &CohortQuery) -> Result<Option<tba::Compliance>> {
    if let Some(id) = query.standard.as_deref() {
        let standard = tba::standard(id).ok_or_else(|| {
            refuse(&format!(
                "unknown standard `{id}` (standards: {:?})",
                tba::STANDARDS.iter().map(|s| s.id).collect::<Vec<_>>()
            ))
        })?;
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
    let lead_times: Vec<i64> = analyses.iter().map(|a| a.lead_time_ms).collect();
    let compliance = score_compliance(&lead_times, &query)?;

    // Small-number suppression (spec §12.2): below the threshold the
    // percentile detail would isolate an individual patient, so the
    // counts and the ranking are returned without it.
    let suppressed = summary.instances > 0 && summary.instances < MIN_COHORT_FOR_PERCENTILES;
    let mut body = serde_json::to_value(&summary).unwrap_or_else(|_| serde_json::json!({}));
    if suppressed && let Some(map) = body.as_object_mut() {
        map.insert("lead_time".to_string(), serde_json::Value::Null);
    }

    format::json(serde_json::json!({
        "as_of": now,
        "pathway": { "pid": template.pid, "name": template.name },
        "note": "lead-time percentiles are nearest-rank, so every one is an \
                 observed journey; the mean is reported but is skew-sensitive. \
                 A divergence between aggregate and median value-adding ratio \
                 is itself the finding — `concentrated` means the waste sits in \
                 a minority of journeys.",
        "suppressed": suppressed,
        "suppression_note": suppressed.then(|| format!(
            "fewer than {MIN_COHORT_FOR_PERCENTILES} instances: percentile \
             detail withheld because it would identify an individual journey"
        )),
        "cohort": body,
        "compliance": compliance,
    }))
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
    format::json(serde_json::json!({
        "as_of": now,
        "pathway": { "pid": template.pid, "name": template.name },
        "note": "findings ordered by recoverable time; each names the rule that \
                 produced it and the threshold that fired. Deliberately not a \
                 composite score, and deliberately never per-clinician.",
        "instances": summary.instances,
        "findings": findings,
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
}
