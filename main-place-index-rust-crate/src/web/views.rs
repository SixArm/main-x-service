//! Tera template engine wrapper.
//!
//! Loads templates from `assets/views/**/*.tera`. The [`WebState`] is the
//! Axum shared state that holds the engine and per-app context defaults.

use std::sync::Arc;
use tera::{Context, Tera};

/// Errors that the web tier can surface.
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("template load error: {0}")]
    Load(#[from] tera::Error),
}

pub type WebResult<T> = std::result::Result<T, WebError>;

/// Per-subproject branding & entity metadata exposed to every template.
#[derive(Clone, Debug)]
pub struct AppContext {
    pub app_display: &'static str,
    pub entity_singular: &'static str,
    pub entity_plural: &'static str,
}

impl AppContext {
    pub const fn default() -> Self {
        Self {
            app_display: "Main Place Index",
            entity_singular: "place",
            entity_plural: "places",
        }
    }

    pub fn apply(&self, ctx: &mut Context) {
        ctx.insert("app_display", self.app_display);
        ctx.insert("entity_singular", self.entity_singular);
        ctx.insert("entity_plural", self.entity_plural);
    }
}

/// Axum shared state for the web UI: the Tera engine + branding.
#[derive(Clone)]
pub struct WebState {
    pub tera: Arc<Tera>,
    pub app: AppContext,
}

impl WebState {
    pub fn new() -> WebResult<Self> {
        Ok(Self {
            tera: Arc::new(engine()?),
            app: AppContext::default(),
        })
    }

    pub fn context(&self) -> Context {
        let mut ctx = Context::new();
        self.app.apply(&mut ctx);
        ctx
    }

    pub fn render(&self, template: &str, ctx: &Context) -> tera::Result<String> {
        self.tera.render(template, ctx)
    }
}

/// Build a fresh Tera engine pointed at `assets/views`.
pub fn engine() -> WebResult<Tera> {
    let pattern = "assets/views/**/*.tera";
    Tera::new(pattern).map_err(WebError::Load)
}
