//! Per-user saved views (T-28l): route-scoped filter/sort/column
//! presets, keyed by the caller's token `sub` and nothing else
//! identity-shaped. Served through the BFF, so the browser never
//! holds the list itself — the front-end server calls this API with
//! the caller's own bearer token, exactly as any other authenticated
//! call. Pure validation lives in [`crate::saved_views`]; a view is
//! always scoped to the `route` it was saved from and to the caller
//! who saved it — never another user's, never another route's.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::models::_entities::saved_views;
use crate::saved_views as rules;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// `POST /api/saved-views` body.
#[derive(Debug, Deserialize)]
struct SavedViewPayload {
    route: String,
    name: String,
    #[serde(default)]
    filter: serde_json::Value,
    #[serde(default)]
    sort: serde_json::Value,
    #[serde(default)]
    columns: Vec<String>,
}

/// `POST /api/saved-views` — save a view under the caller's own
/// `sub`. Requires authentication ([`AuthUser`]): a saved view with no
/// owner is not a saved view.
#[debug_handler]
async fn create_saved_view(
    State(ctx): State<AppContext>,
    AuthUser(claims): AuthUser,
    Json(payload): Json<SavedViewPayload>,
) -> Result<Response> {
    let mut problems = rules::problems(&payload.route, &payload.name, &payload.columns);
    if rules::json_blob_too_large(&payload.filter) {
        problems.push("filter: too large".to_string());
    }
    if rules::json_blob_too_large(&payload.sort) {
        problems.push("sort: too large".to_string());
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }

    let row = saved_views::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        sub: ActiveValue::set(claims.sub.clone()),
        route: ActiveValue::set(payload.route.clone()),
        name: ActiveValue::set(payload.name.clone()),
        filter: ActiveValue::set(payload.filter.clone()),
        sort: ActiveValue::set(payload.sort.clone()),
        columns: ActiveValue::set(serde_json::json!(payload.columns)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    format::json(row)
}

/// `GET /api/saved-views?route=` query.
#[derive(Debug, Deserialize)]
struct ListParams {
    /// Narrow to one route; omitted ⇒ every one of the caller's own
    /// views, across routes (a "manage my views" listing).
    #[serde(default)]
    route: Option<String>,
}

/// `GET /api/saved-views?route=` — the caller's own views, optionally
/// narrowed to one route. Never returns another `sub`'s rows.
#[debug_handler]
async fn list_saved_views(
    State(ctx): State<AppContext>,
    AuthUser(claims): AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Response> {
    let mut query = saved_views::Entity::find()
        .filter(saved_views::Column::Sub.eq(claims.sub.clone()))
        .filter(saved_views::Column::DeletedAt.is_null());
    if let Some(route) = &params.route {
        query = query.filter(saved_views::Column::Route.eq(route.clone()));
    }
    let rows = query
        .order_by_asc(saved_views::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(rows)
}

/// Find one of the **caller's own** live saved views by pid. Another
/// user's view — or an unknown pid — is `404`, identically: a saved
/// view's existence is not disclosed across users.
async fn find_own(ctx: &AppContext, sub: &str, pid: &str) -> Result<saved_views::Model> {
    let pid = Uuid::parse_str(pid).map_err(|_| Error::NotFound)?;
    saved_views::Entity::find()
        .filter(saved_views::Column::Pid.eq(pid))
        .filter(saved_views::Column::Sub.eq(sub))
        .filter(saved_views::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// `DELETE /api/saved-views/{pid}` — soft-delete, scoped to the
/// caller's own `sub`.
#[debug_handler]
async fn delete_saved_view(
    State(ctx): State<AppContext>,
    AuthUser(claims): AuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row = find_own(&ctx, &claims.sub, &pid).await?;
    let mut active: saved_views::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await.map_err(db_err)?;
    format::empty_json()
}

/// The saved-views routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/saved-views", post(create_saved_view))
        .add("/saved-views", get(list_saved_views))
        .add("/saved-views/{pid}", delete(delete_saved_view))
}
