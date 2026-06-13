//! Care-pathway CRUD + matching endpoints.
//!
//! The API DTO is `care_pathway_matcher::CarePathway` itself — the
//! service stores it verbatim (as JSON) and matches with the canonical
//! `care-pathway-matcher` engine, so there is no separate model or
//! adapter to drift.

use axum::http::StatusCode;
use care_pathway_matcher::{CarePathway, MatchConfig, MatchingEngine};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::care_pathways::Model as PathwayModel;

/// Validate an incoming `CarePathway` payload.
///
/// Family convention (OQ-1 resolution): validation failures return
/// `422 Unprocessable Entity`, matching the person/place services.
/// loco has no `unprocessable_entity` helper, so this uses
/// `Error::CustomError(StatusCode::UNPROCESSABLE_ENTITY, …)`.
///
/// # Errors
///
/// Returns a `422` error when `name` is blank.
pub fn validate(pathway: &CarePathway) -> Result<()> {
    if pathway.name.trim().is_empty() {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new("validation", "name is required"),
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct PathwayRef {
    pid: String,
    name: String,
}

impl PathwayRef {
    fn of(m: &PathwayModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            name: m.name.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MatchRequest {
    query: CarePathway,
    candidates: Vec<CarePathway>,
}

#[derive(Debug, Serialize)]
struct ScoredRef {
    pid: String,
    name: String,
    score: f64,
    confidence: String,
    is_match: bool,
}

/// Create a care pathway.
#[debug_handler]
async fn create(
    State(ctx): State<AppContext>,
    Json(pathway): Json<CarePathway>,
) -> Result<Response> {
    validate(&pathway)?;
    let model = PathwayModel::create(&ctx.db, &pathway).await?;
    format::json(PathwayRef::of(&model))
}

/// Fetch a care pathway by public id.
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = PathwayModel::find_by_pid(&ctx.db, &pid).await?;
    format::json(model.to_pathway()?)
}

/// Replace a care pathway's payload.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    Json(pathway): Json<CarePathway>,
) -> Result<Response> {
    validate(&pathway)?;
    let model = PathwayModel::find_by_pid(&ctx.db, &pid).await?;
    let updated = model
        .into_active_model()
        .update_data(&ctx.db, &pathway)
        .await?;
    format::json(PathwayRef::of(&updated))
}

/// Soft-delete a care pathway.
#[debug_handler]
async fn remove(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = PathwayModel::find_by_pid(&ctx.db, &pid).await?;
    model.into_active_model().soft_delete(&ctx.db).await?;
    format::empty_json()
}

/// List active care pathways (capped at 100).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = PathwayModel::list(&ctx.db, 100).await?;
    let refs: Vec<PathwayRef> = rows.iter().map(PathwayRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored care pathways that match the query above the threshold.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<CarePathway>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = PathwayModel::list(&ctx.db, 1000).await?;
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_pathway()?;
        let r = engine.match_care_pathways(&query, &candidate);
        if r.is_match {
            hits.push(ScoredRef {
                pid: row.pid.to_string(),
                name: row.name.clone(),
                score: r.score,
                confidence: format!("{:?}", r.confidence),
                is_match: r.is_match,
            });
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    format::json(hits)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/", post(create))
        .add("/", get(list))
        .add("/match", post(match_against))
        .add("/check-duplicates", post(check_duplicates))
        .add("/{pid}", get(get_one))
        .add("/{pid}", put(update))
        .add("/{pid}", delete(remove))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the OQ-1 / T-2 decision: blank-name validation failure is
    /// `422 Unprocessable Entity` (family convention), not `400`.
    /// Runs without a database, so the pin holds on default `cargo test`.
    #[test]
    fn blank_name_validation_returns_422() {
        for name in ["", "   ", "\t\n"] {
            let err = validate(&CarePathway::new(name)).expect_err("blank name must fail");
            match err {
                Error::CustomError(status, _) => {
                    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
                }
                other => panic!("expected CustomError(422), got {other:?}"),
            }
        }
    }

    /// The 422 must survive loco's error-to-response conversion.
    #[test]
    fn blank_name_validation_response_status_is_422() {
        let err = validate(&CarePathway::new("")).expect_err("blank name must fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[test]
    fn non_blank_name_passes_validation() {
        assert!(validate(&CarePathway::new("Acute Stroke Care Pathway")).is_ok());
    }
}
