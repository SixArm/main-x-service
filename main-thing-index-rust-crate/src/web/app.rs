//! Loco.rs application hooks for the web tier.

pub const APP_NAME: &str = "main_thing_index";

/// Convenience accessor for the web router.
pub fn web_router() -> super::views::WebResult<axum::Router> {
    super::router()
}
