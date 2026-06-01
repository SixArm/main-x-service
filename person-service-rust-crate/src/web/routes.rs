//! Axum router for the server-rendered web UI.
//!
//! Mounts the home page, entity index, HTMX search partial, and the
//! static asset directory at `/static/*`.

use axum::{routing::get, Router};
use tower_http::services::ServeDir;

use super::controllers;
use super::views::{WebResult, WebState};

/// Build an [`axum::Router`] hosting the web UI.
pub fn router() -> WebResult<Router> {
    let state = WebState::new()?;
    let entity_plural = state.app.entity_plural;

    let pages = Router::new()
        .route("/", get(controllers::home))
        .route("/health", get(controllers::health))
        .route("/metrics", get(controllers::metrics))
        .route("/metrics.prom", get(controllers::prometheus))
        .route("/tour", get(controllers::tour))
        .route("/docs", get(controllers::docs))
        .route("/palette", get(controllers::palette))
        .route("/notifications", get(controllers::notifications))
        .route("/audit", get(controllers::audit_recent))
        .route("/settings", get(controllers::settings))
        .route("/tokens", get(controllers::tokens))
        .route("/dev/500", get(controllers::dev_error))
        .route(
            &format!("/{entity_plural}"),
            get(controllers::index),
        )
        .route(
            &format!("/{entity_plural}/search/partial"),
            get(controllers::search_partial),
        )
        .route(
            &format!("/{entity_plural}/search"),
            get(controllers::search_page),
        )
        .route(
            &format!("/{entity_plural}/review-queue"),
            get(controllers::review_queue),
        )
        .route(
            &format!("/{entity_plural}/review-queue/kanban"),
            get(controllers::review_queue_kanban),
        )
        .route(
            &format!("/{entity_plural}/deduplicate"),
            get(controllers::deduplicate),
        )
        .route(
            &format!("/{entity_plural}/trash"),
            get(controllers::trash),
        )
        .route(
            &format!("/{entity_plural}/starred"),
            get(controllers::starred),
        )
        .route(
            &format!("/{entity_plural}/compare"),
            get(controllers::compare),
        )
        .route(
            &format!("/{entity_plural}/import"),
            get(controllers::import),
        )
        .route(
            &format!("/{entity_plural}/calendar"),
            get(controllers::calendar),
        )
        .route(
            &format!("/{entity_plural}/map"),
            get(controllers::map),
        )
        .route(
            &format!("/{entity_plural}/:id"),
            get(controllers::show),
        )
        .route(
            &format!("/{entity_plural}/:id/edit"),
            get(controllers::edit),
        )
        .route(
            &format!("/{entity_plural}/:id/audit"),
            get(controllers::audit),
        )
        .route(
            &format!("/{entity_plural}/:id/export"),
            get(controllers::export),
        )
        .route(
            &format!("/{entity_plural}/:id/fhir"),
            get(controllers::fhir),
        )
        .route(
            &format!("/{entity_plural}/:id/consents"),
            get(controllers::consents),
        )
        .route(
            &format!("/{entity_plural}/:id/links"),
            get(controllers::links),
        )
        .route(
            &format!("/{entity_plural}/:id/qr"),
            get(controllers::qr),
        )
        .route(
            &format!("/{entity_plural}/:id/quality"),
            get(controllers::quality),
        )
        .route(
            &format!("/{entity_plural}/:id/notes"),
            get(controllers::notes),
        )
        .route(
            &format!("/{entity_plural}/:id/sign"),
            get(controllers::sign),
        )
        .fallback(controllers::not_found)
        .with_state(state);

    Ok(pages.nest_service("/static", ServeDir::new("assets/static")))
}
