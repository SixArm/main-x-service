//! Loco.rs application hooks for the web tier.
//!
//! This module sketches a Loco [`Hooks`] implementation so the project
//! can be booted via Loco's conventions (config in `config/<env>.yaml`,
//! `cargo loco start`, background workers, etc.).
//!
//! The wiring is intentionally minimal so it co-exists with the existing
//! Axum REST API. Where Loco's trait surface differs between versions,
//! prefer following the upstream [Loco starter](https://loco.rs) for the
//! version of `loco-rs` pinned in `Cargo.toml`.

pub const APP_NAME: &str = "main_person_index";

/// Convenience accessor for the web router used by the rest of the app.
///
/// Wire this from your main binary or from a Loco
/// `Hooks::after_routes` implementation:
///
/// ```ignore
/// async fn after_routes(router: axum::Router, _ctx: &AppContext) -> loco_rs::Result<axum::Router> {
///     Ok(router.merge(crate::web::app::web_router()?))
/// }
/// ```
pub fn web_router() -> super::views::WebResult<axum::Router> {
    super::router()
}
