//! Organization CRUD + matching endpoints.
//!
//! The API DTO is `organization_matcher::Organization` itself — the
//! service stores it verbatim (as JSON) and matches with the canonical
//! `organization-matcher` engine, so there is no separate model or
//! adapter to drift.

use loco_rs::prelude::*;
use organization_matcher::{MatchConfig, MatchingEngine, Organization};
use serde::{Deserialize, Serialize};

use crate::models::organizations::Model as OrgModel;

#[derive(Debug, Serialize)]
struct OrgRef {
    pid: String,
    name: String,
}

impl OrgRef {
    fn of(m: &OrgModel) -> Self {
        Self {
            pid: m.pid.to_string(),
            name: m.name.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MatchRequest {
    query: Organization,
    candidates: Vec<Organization>,
}

#[derive(Debug, Serialize)]
struct ScoredRef {
    pid: String,
    name: String,
    score: f64,
    confidence: String,
    is_match: bool,
}

/// Create an organization.
#[debug_handler]
async fn create(State(ctx): State<AppContext>, Json(org): Json<Organization>) -> Result<Response> {
    if org.name.trim().is_empty() {
        return bad_request("name is required");
    }
    let model = OrgModel::create(&ctx.db, &org).await?;
    format::json(OrgRef::of(&model))
}

/// Fetch an organization by public id.
#[debug_handler]
async fn get_one(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid).await?;
    let org = model.to_org()?;
    format::json(org)
}

/// Replace an organization's payload.
#[debug_handler]
async fn update(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    Json(org): Json<Organization>,
) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid).await?;
    let updated = model.into_active_model().update_data(&ctx.db, &org).await?;
    format::json(OrgRef::of(&updated))
}

/// Soft-delete an organization.
#[debug_handler]
async fn remove(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let model = OrgModel::find_by_pid(&ctx.db, &pid).await?;
    model.into_active_model().soft_delete(&ctx.db).await?;
    format::empty_json()
}

/// List active organizations (capped at 100).
#[debug_handler]
async fn list(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = OrgModel::list(&ctx.db, 100).await?;
    let refs: Vec<OrgRef> = rows.iter().map(OrgRef::of).collect();
    format::json(refs)
}

/// Score a query against an explicit candidate list (no persistence).
#[debug_handler]
async fn match_against(Json(req): Json<MatchRequest>) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let results = engine.rank(&req.query, &req.candidates);
    format::json(results)
}

/// Find stored organizations that match the query above the threshold.
#[debug_handler]
async fn check_duplicates(
    State(ctx): State<AppContext>,
    Json(query): Json<Organization>,
) -> Result<Response> {
    let engine = MatchingEngine::new(MatchConfig::default());
    let rows = OrgModel::list(&ctx.db, 1000).await?;
    let mut hits: Vec<ScoredRef> = Vec::new();
    for row in &rows {
        let candidate = row.to_org()?;
        let r = engine.match_organizations(&query, &candidate);
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
        .prefix("/api/organizations")
        .add("/", post(create))
        .add("/", get(list))
        .add("/match", post(match_against))
        .add("/check-duplicates", post(check_duplicates))
        .add("/{pid}", get(get_one))
        .add("/{pid}", put(update))
        .add("/{pid}", delete(remove))
}
