//! Bulk export codecs over the pathway instance layer (spec
//! `13-tasks.md` T-14a): `event_log` and `journey_features`, as CSV or
//! JSONL — the pure row-shaping lives in [`crate::analytics`], this
//! module is only the loading + HTTP surface.
//!
//! **Scope note (documented, not silent).** T-14a says it "extends
//! T-10" — the native async bulk-import/export job contract
//! (`agents/share/bulk-import-export.md`) — which is not yet built for
//! this crate (only the durable `bulk_jobs` table + `ArtifactStore`
//! scaffolding exist, shared with the FHIR `$export` worker). This is a
//! **synchronous** v1: it renders the response on the request path
//! rather than queuing a job, bounded by the same per-cohort instance
//! cap ([`load_cohort`]) every other cohort endpoint in this crate
//! already uses. Promoting it to a T-10 job is tracked as follow-up,
//! not silently dropped.
//!
//! **Authorization.** A bulk pull of every enrolled instance on a
//! pathway is a materially larger disclosure than reading one instance,
//! so — mirroring `agents/share/cross-service-linking.md` §10.2's
//! precedent for the `continues_as` bulk reconciliation pull — this is
//! gated as [`authentication_verifier::Action::Destructive`], not
//! `Read`, and every call is audited as a disclosure
//! (`disclosure::action::EXPORT`).
//!
//! **No `masking_profile` knob.** The family bulk-export contract
//! (`agents/share/bulk-import-export.md` §8) offers a masked-by-default,
//! full-gated profile. This codec has nothing a "full" mode would add:
//! `subject_ref` and every actor's raw URN are excluded
//! *unconditionally* in [`crate::analytics`] — never produced, not
//! merely withheld by default — so there is no unmasked view to gate.

use std::collections::{BTreeMap, HashMap};

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use uuid::Uuid;

use crate::analytics::{self, CaseContext, EventInput, SegmentInput, StepInput};
use crate::auth::MaybeAuthUser;
use crate::compliance::disclosure::{self, AccessContext};
use crate::controllers::tba::{analyze_cohort, load_cohort};
use crate::models::_entities::{instance_events, instance_segments, instance_steps, instance_team};
use crate::models::care_pathways::Model as PathwayModel;

/// `422` with a reason.
fn refuse(reason: &str) -> Error {
    Error::CustomError(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable_entity", reason),
    )
}

/// The record-level ABAC decision was denied; map its reason to `401`/`403`.
fn record_rejection((status, reason): (axum::http::StatusCode, String)) -> Error {
    Error::CustomError(status, ErrorDetail::new("forbidden", &reason))
}

/// The read-audit write failed and the deployment has asked to fail
/// closed; refuse the export rather than disclose data unaccounted for.
fn audit_unavailable(_: disclosure::AuditWriteRefused) -> Error {
    Error::CustomError(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        ErrorDetail::new(
            "audit_unavailable",
            "the read-access audit write failed and CARE_PATHWAY_AUDIT_FAIL_CLOSED \
             is on; refusing rather than disclosing data unaccounted for",
        ),
    )
}

/// Epoch milliseconds of a stored timestamp.
fn ms(at: chrono::DateTime<chrono::FixedOffset>) -> i64 {
    at.timestamp_millis()
}

/// Midnight UTC of a stored date, in epoch milliseconds.
fn date_ms(date: chrono::NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0)
        .map_or(0, |dt| dt.and_utc().timestamp_millis())
}

/// `?format=csv|jsonl` (default `jsonl`) + `?status=open|closed|all`
/// (default: every non-deleted instance, same as [`load_cohort`]).
#[derive(Debug, serde::Deserialize)]
struct ExportQuery {
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

/// Resolve the requested format, or `422` on anything else — a typo in
/// `?format=` should not silently fall back to a format the caller did
/// not ask for.
fn parse_format(query: &ExportQuery) -> Result<&'static str> {
    match query
        .format
        .as_deref()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        None | Some("jsonl") => Ok("jsonl"),
        Some("csv") => Ok("csv"),
        Some(other) => Err(refuse(&format!(
            "unsupported format '{other}': use \"csv\" or \"jsonl\""
        ))),
    }
}

/// The pathway template's `care_setting`, lowercased — the same
/// derivation [`crate::auth::care_pathway_resource_attrs`] uses, kept
/// local so this module never has to import the matcher's `CareSetting`
/// enum just to `Debug`-format it.
fn care_setting_string(pathway: &care_pathway_matcher::CarePathway) -> Option<String> {
    pathway
        .care_setting
        .as_ref()
        .map(|setting| format!("{setting:?}").to_lowercase())
}

/// Build the response body + content-type for one codec's rendering.
fn render_response(body: String, content_type: &'static str) -> Result<Response> {
    Ok(Response::builder()
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(body))?)
}

/// Authorize + audit a bulk export against one pathway template, or
/// return the `Result` error to propagate. Shared by both export
/// endpoints so the two cannot drift in what they gate on.
async fn authorize_and_audit_export(
    ctx: &AppContext,
    template: &PathwayModel,
    pathway: &care_pathway_matcher::CarePathway,
    caller: &MaybeAuthUser,
    access: &AccessContext,
) -> Result<()> {
    crate::auth::authorize_record(
        caller,
        authentication_verifier::Action::Destructive,
        &crate::auth::care_pathway_resource_attrs(pathway),
    )
    .map_err(record_rejection)?;
    disclosure::record_access(
        &ctx.db,
        template.pid,
        disclosure::action::EXPORT,
        caller.actor(),
        access,
    )
    .await
    .map_err(audit_unavailable)?;
    Ok(())
}

/// Load every segment/step/event/team-membership row for a cohort of
/// instances in four bounded queries (no N+1), grouped by instance pid —
/// the same shape [`analyze_cohort`] uses for segments alone.
async fn load_event_log_inputs(
    ctx: &AppContext,
    pids: &[Uuid],
) -> Result<(
    HashMap<Uuid, Vec<SegmentInput>>,
    HashMap<Uuid, Vec<StepInput>>,
    HashMap<Uuid, Vec<EventInput>>,
    HashMap<Uuid, BTreeMap<String, String>>,
)> {
    if pids.is_empty() {
        return Ok((
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        ));
    }

    let segment_rows = instance_segments::Entity::find()
        .filter(instance_segments::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_segments::Column::StartedAt)
        .all(&ctx.db)
        .await?;
    let mut segments: HashMap<Uuid, Vec<SegmentInput>> = HashMap::new();
    for row in &segment_rows {
        segments
            .entry(row.instance_pid)
            .or_default()
            .push(SegmentInput {
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
        .filter(instance_steps::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_steps::Column::Position)
        .all(&ctx.db)
        .await?;
    let mut steps: HashMap<Uuid, Vec<StepInput>> = HashMap::new();
    for row in &step_rows {
        steps.entry(row.instance_pid).or_default().push(StepInput {
            label: row.label.clone(),
            done_at_ms: row.done_on.map(date_ms),
        });
    }

    let event_rows = instance_events::Entity::find()
        .filter(instance_events::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_events::Column::OccurredAt)
        .all(&ctx.db)
        .await?;
    let mut events: HashMap<Uuid, Vec<EventInput>> = HashMap::new();
    for row in &event_rows {
        events
            .entry(row.instance_pid)
            .or_default()
            .push(EventInput {
                kind: row.kind.clone(),
                occurred_at_ms: ms(row.occurred_at),
            });
    }

    let team_rows = instance_team::Entity::find()
        .filter(instance_team::Column::InstancePid.is_in(pids.to_vec()))
        .all(&ctx.db)
        .await?;
    let mut team: HashMap<Uuid, BTreeMap<String, String>> = HashMap::new();
    for row in &team_rows {
        team.entry(row.instance_pid)
            .or_default()
            .insert(row.member_ref.clone(), row.role.clone());
    }

    Ok((segments, steps, events, team))
}

/// `GET /api/care-pathways/{pathway}/export/event-log` (spec T-14a).
#[debug_handler]
async fn export_event_log(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<ExportQuery>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let format = parse_format(&query)?;
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(super::model_not_found)?;
    let pathway_dto = template.to_pathway()?;
    authorize_and_audit_export(&ctx, &template, &pathway_dto, &caller, &access).await?;

    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let care_setting = care_setting_string(&pathway_dto);
    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    let (mut segments, mut steps, mut events, mut team) =
        load_event_log_inputs(&ctx, &pids).await?;

    let mut rows = Vec::new();
    for instance in &instances {
        let case_ctx = CaseContext {
            case_id: instance.pid.to_string(),
            pathway_pid: template.pid.to_string(),
            care_setting: care_setting.clone(),
            urgency: instance.urgency.clone(),
            status: instance.status.clone(),
            outcome: instance.outcome.clone(),
        };
        rows.extend(analytics::event_log_rows(
            &case_ctx,
            &segments.remove(&instance.pid).unwrap_or_default(),
            &steps.remove(&instance.pid).unwrap_or_default(),
            &events.remove(&instance.pid).unwrap_or_default(),
            &team.remove(&instance.pid).unwrap_or_default(),
        ));
    }

    match format {
        "csv" => render_response(analytics::event_log_csv(&rows), analytics::CSV_CONTENT_TYPE),
        _ => render_response(
            analytics::event_log_jsonl(&rows),
            analytics::NDJSON_CONTENT_TYPE,
        ),
    }
}

/// `GET /api/care-pathways/{pathway}/export/journey-features` (spec T-14a).
#[debug_handler]
async fn export_journey_features(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<ExportQuery>,
    caller: MaybeAuthUser,
    access: AccessContext,
) -> Result<Response> {
    let format = parse_format(&query)?;
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(super::model_not_found)?;
    let pathway_dto = template.to_pathway()?;
    authorize_and_audit_export(&ctx, &template, &pathway_dto, &caller, &access).await?;

    let now = chrono::Utc::now().timestamp_millis();
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let analyses = analyze_cohort(&ctx, &instances, now).await?;
    let care_setting = care_setting_string(&pathway_dto);

    let rows: Vec<analytics::JourneyFeatureRow> = instances
        .iter()
        .zip(analyses.iter())
        .map(|(instance, analysis)| {
            let case_ctx = CaseContext {
                case_id: instance.pid.to_string(),
                pathway_pid: template.pid.to_string(),
                care_setting: care_setting.clone(),
                urgency: instance.urgency.clone(),
                status: instance.status.clone(),
                outcome: instance.outcome.clone(),
            };
            analytics::journey_feature_row(&case_ctx, analysis)
        })
        .collect();

    match format {
        "csv" => render_response(
            analytics::journey_features_csv(&rows),
            analytics::CSV_CONTENT_TYPE,
        ),
        _ => render_response(
            analytics::journey_features_jsonl(&rows),
            analytics::NDJSON_CONTENT_TYPE,
        ),
    }
}

/// Pathway-scoped export routes (prefix `/api/care-pathways`), added
/// before the registry's `/{pid}` capture — same ordering precaution as
/// [`super::tba::pathway_routes`].
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/{pathway}/export/event-log", get(export_event_log))
        .add(
            "/{pathway}/export/journey-features",
            get(export_journey_features),
        )
}
