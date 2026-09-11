//! Journey data-quality and missingness report (spec `13-tasks.md`
//! T-14h) — the HTTP surface over [`crate::data_quality`]'s pure
//! detectors: `GET /api/care-pathways/{pathway}/data-quality`.
//!
//! Gated exactly like the sibling cohort views
//! (`cohort_time_analysis`/`cohort_constraints` in
//! [`crate::controllers::tba`]): no record-level ABAC or explicit
//! audit call here — this is an aggregate count report, not a bulk
//! pull of instance rows, so it stays under the ordinary blanket
//! `<ENTITY>_REQUIRE_AUTH` guard rather than
//! [`crate::controllers::exports`]'s elevated `Destructive` gate.

use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use uuid::Uuid;

use crate::controllers::tba::{
    analyze_cohort, date_ms, load_cohort, resolve_anchor_pair_raw, to_segment,
};
use crate::data_quality::{self, InstanceDqInputs, StepOrder};
use crate::models::_entities::{instance_segments, instance_steps};
use crate::models::care_pathways::Model as PathwayModel;
use crate::tba;

/// `?status=` (shared with the other cohort endpoints) plus
/// `?from_anchor=&to_anchor=` (spec T-14d's own pair, reused verbatim
/// for the `anchors_unreached` code).
#[derive(Debug, Default, serde::Deserialize)]
struct DataQualityQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    from_anchor: Option<String>,
    #[serde(default)]
    to_anchor: Option<String>,
}

/// Bulk-load every instance's segments (as [`tba::Segment`]) and steps
/// (as [`StepOrder`], carrying `position` — the one fact
/// [`crate::analytics::StepInput`] does not, since the event-log codec
/// it serves never needed it) in two bounded queries, grouped by
/// instance pid.
async fn load_dq_inputs(
    ctx: &AppContext,
    pids: &[Uuid],
) -> Result<(
    std::collections::HashMap<Uuid, Vec<tba::Segment>>,
    std::collections::HashMap<Uuid, Vec<StepOrder>>,
)> {
    if pids.is_empty() {
        return Ok((
            std::collections::HashMap::new(),
            std::collections::HashMap::new(),
        ));
    }
    let segment_rows = instance_segments::Entity::find()
        .filter(instance_segments::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_segments::Column::StartedAt)
        .all(&ctx.db)
        .await?;
    let mut segments: std::collections::HashMap<Uuid, Vec<tba::Segment>> =
        std::collections::HashMap::new();
    for row in &segment_rows {
        segments
            .entry(row.instance_pid)
            .or_default()
            .push(to_segment(row));
    }

    let step_rows = instance_steps::Entity::find()
        .filter(instance_steps::Column::InstancePid.is_in(pids.to_vec()))
        .order_by_asc(instance_steps::Column::Position)
        .all(&ctx.db)
        .await?;
    let mut steps: std::collections::HashMap<Uuid, Vec<StepOrder>> =
        std::collections::HashMap::new();
    for row in &step_rows {
        steps.entry(row.instance_pid).or_default().push(StepOrder {
            position: row.position,
            done_at_ms: row.done_on.map(date_ms),
        });
    }

    Ok((segments, steps))
}

/// `GET /api/care-pathways/{pathway}/data-quality` (spec T-14h).
#[debug_handler]
async fn data_quality(
    State(ctx): State<AppContext>,
    Path(pathway): Path<String>,
    Query(query): Query<DataQualityQuery>,
) -> Result<Response> {
    let now = chrono::Utc::now();
    let as_of_ms = now.timestamp_millis();
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let analyses = analyze_cohort(&ctx, &instances, as_of_ms).await?;

    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    let (segments, steps) = load_dq_inputs(&ctx, &pids).await?;

    // An invalid or one-sided pair falls back to "not evaluated" with
    // a disclosed reason (`report.anchor_note`), exactly like T-14d's
    // own compliance path — never a hard `422` for a query naming no
    // anchor at all, and never a silent guess at what the caller
    // meant by a malformed one. `build_report` takes this `Result`
    // directly: it is the one value that already distinguishes "no
    // pair requested" from "a pair was requested but did not parse".
    let anchor_pair =
        resolve_anchor_pair_raw(query.from_anchor.as_deref(), query.to_anchor.as_deref());

    let empty_segments: Vec<tba::Segment> = Vec::new();
    let empty_steps: Vec<StepOrder> = Vec::new();
    let inputs: Vec<InstanceDqInputs<'_>> = instances
        .iter()
        .zip(&analyses)
        .map(|(instance, analysis)| InstanceDqInputs {
            status: &instance.status,
            segments: segments.get(&instance.pid).unwrap_or(&empty_segments),
            steps: steps.get(&instance.pid).unwrap_or(&empty_steps),
            clock: &analysis.clock,
            enrolled_on_ms: date_ms(instance.enrolled_on),
            as_of_ms,
            coverage_ratio: analysis.coverage_ratio.value,
            anchors: &analysis.anchors,
        })
        .collect();

    let report = data_quality::build_report(&inputs, anchor_pair);

    format::json(serde_json::json!({
        "as_of": now,
        "pathway": { "pid": template.pid, "name": template.name },
        "note": "the report is the finding; it never imputes. window/coverage_floor-style \
                 gaps in what this crate can detect are named, not silently treated as clean.",
        "report": report,
    }))
}

/// This resource's routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/{pathway}/data-quality", get(data_quality))
}
