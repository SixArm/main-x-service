//! `GET /api/care-pathways/{pathway}/screening` — screening reach and
//! timing (spec §13 T-15c). See [`crate::screening`] for the counting and
//! the suppression rule; this module loads rows and renders.

use authentication_verifier::Action;
use axa::error::ErrorDetail;
use axa::extract::{Path, Query};
use axa::prelude::*;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::controllers::access;
use crate::controllers::tba::{CohortQuery, analyze_cohort, build_attrition, load_cohort};
use crate::instruments;
use crate::models::care_pathways::Model as PathwayModel;
use crate::models::instance_measures;
use crate::screening;
use crate::suppression;
use crate::tba;

/// Query parameters.
#[derive(Debug, serde::Deserialize)]
struct ScreeningQuery {
    /// The catalogued instrument to count screens on (required).
    instrument: Option<String>,
    /// Count only a first screen within this many days of the anchor.
    #[serde(default)]
    within_days: Option<i64>,
    /// `open` | `closed` | `all` (default), as for the cohort views.
    #[serde(default)]
    status: Option<String>,
}

fn refuse(reason: &str) -> Error {
    Error::CustomError(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable_entity", reason),
    )
}

/// `GET /api/care-pathways/{pathway}/screening?instrument=&within_days=`.
#[debug_handler]
async fn screening_reach(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pathway): Path<String>,
    Query(query): Query<ScreeningQuery>,
) -> Result<Response> {
    let Some(instrument) = query.instrument.as_deref().and_then(instruments::find) else {
        return Err(refuse(
            "instrument is required and must be a catalogued instrument id",
        ));
    };
    if query.within_days.is_some_and(|d| d < 0) {
        return Err(refuse("within_days must not be negative"));
    }
    let template = PathwayModel::find_by_pid(&ctx.db, &pathway)
        .await
        .map_err(|_| Error::NotFound)?;
    // A masked caller may not see mental-health scores, and every
    // catalogued instrument is one: an aggregate must not reveal more than
    // the single read does.
    let granted = access::pathway_access(&ctx, &caller, Action::Read, template.pid).await?;
    if granted.masked {
        return Err(Error::NotFound);
    }

    let as_of_ms = chrono::Utc::now().timestamp_millis();
    let instances = load_cohort(&ctx, template.pid, query.status.as_deref()).await?;
    let pids: Vec<Uuid> = instances.iter().map(|i| i.pid).collect();
    let measures = if pids.is_empty() {
        Vec::new()
    } else {
        instance_measures::Model::for_instances(&ctx.db, &pids).await?
    };
    let inputs: Vec<screening::Input> = instances
        .iter()
        .map(|instance| {
            let first = measures
                .iter()
                .filter(|m| m.instance_pid == instance.pid)
                .filter(|m| m.instrument.as_deref() == Some(instrument.id))
                .filter_map(|m| m.value_numeric.map(|v| (m.recorded_on, v)))
                .min_by_key(|(on, _)| *on);
            screening::Input {
                anchor: instance
                    .clock_start_at
                    .map_or(instance.enrolled_on, |at| at.date_naive()),
                first_screen: first,
            }
        })
        .collect();

    let counts = screening::count(&inputs, instrument, query.within_days);
    let withheld = screening::withheld_for_deployment(&counts);

    let analyses = analyze_cohort(&ctx, &instances, as_of_ms).await?;
    let cohort_query = CohortQuery::with_status(query.status.clone());
    let attrition =
        build_attrition(&ctx, &template, &cohort_query, &analyses, withheld, None).await?;

    let body_instrument = serde_json::json!({
        "id": instrument.id, "name": instrument.name,
        "screen_at": instrument.screen_at,
    });
    let figures = if withheld {
        serde_json::json!({
            "withheld": true,
            "reason": suppression::SUPPRESSED_REASON,
            "note": "reach and positive share are withheld together when any \
                     cell, or the cohort, is small: a single withheld cell \
                     could otherwise be recovered by subtraction",
        })
    } else {
        let median = tba::percentile(&counts.days_to_first_screen, 0.50);
        let p90 = tba::percentile(&counts.days_to_first_screen, 0.90);
        serde_json::json!({
            "withheld": false,
            "denominator": { "enrolled": counts.enrolled },
            "reach": {
                "screened": counts.screened,
                "unscreened": counts.unscreened,
                "share": screening::share(counts.screened, counts.enrolled),
            },
            "time_to_first_screen_days": { "median": median, "p90": p90 },
            "positive": counts.positive.map(|positive| serde_json::json!({
                "of_screened": counts.screened,
                "count": positive,
                "share": screening::share(positive, counts.screened),
            })),
        })
    };
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "pathway": { "pid": template.pid, "name": template.name },
        "instrument": body_instrument,
        "anchor": "clock_start",
        "window_days": query.within_days,
        "status": query.status.as_deref().unwrap_or("all"),
        "figures": figures,
        "attrition": attrition,
        "note": "An instance with no screen is in the denominator. Counts the \
                 first screen on this instrument only, from the instance's \
                 clock start; a first screen after the window counts as \
                 unscreened. A positive screen is a prompt to assess, not a \
                 diagnosis, and a low positive share over a low reach is not \
                 a low rate: read the two together. Nothing here is per person.",
    }))
}

/// Pathway-scoped routes (prefix `/api/care-pathways`), added before the
/// registry's `/{pid}` capture.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/{pathway}/screening", get(screening_reach))
}
