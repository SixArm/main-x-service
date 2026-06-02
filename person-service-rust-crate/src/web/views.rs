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
    pub const fn person() -> Self {
        Self {
            app_display: "Person Service",
            entity_singular: "person",
            entity_plural: "persons",
        }
    }

    /// Inject branding into a Tera [`Context`].
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
    pub themes: Vec<Theme>,
}

impl WebState {
    pub fn new() -> WebResult<Self> {
        Ok(Self {
            tera: Arc::new(engine()?),
            app: AppContext::person(),
            themes: scan_themes(),
        })
    }

    /// Build a Tera [`Context`] preloaded with branding defaults.
    pub fn context(&self) -> Context {
        let mut ctx = Context::new();
        self.app.apply(&mut ctx);
        ctx.insert("themes", &self.themes);
        ctx
    }

    /// Render a named template with the supplied context.
    pub fn render(&self, template: &str, ctx: &Context) -> tera::Result<String> {
        self.tera.render(template, ctx)
    }
}

/// Build a fresh Tera engine pointed at `assets/views`.
pub fn engine() -> WebResult<Tera> {
    let pattern = "assets/views/**/*.tera";
    Tera::new(pattern).map_err(WebError::Load)
}

/// A single visual theme discovered under
/// `assets/static/css/themes/`. `slug` is the filename stem
/// (e.g. `"dark"` from `dark.css`); `label` is the human-readable
/// label rendered in the picker UI.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Theme {
    pub slug: String,
    pub label: String,
}

/// Scan `assets/static/css/themes/*.css` at WebState-construction
/// time. Returns the discovered themes sorted alphabetically by slug,
/// or an empty `Vec` if the directory does not exist (so callers do
/// not have to special-case offline test environments).
fn scan_themes() -> Vec<Theme> {
    let dir = std::path::Path::new("assets/static/css/themes");
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };
    let mut slugs: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) != Some("css") {
                return None;
            }
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })
        .collect();
    slugs.sort();
    slugs.dedup();
    slugs
        .into_iter()
        .map(|slug| {
            let label = label_for_slug(&slug);
            Theme { slug, label }
        })
        .collect()
}

/// Map a theme slug (filename stem) to a human-readable label.
/// Slugs without an explicit mapping get title-case-by-first-letter
/// treatment.
fn label_for_slug(slug: &str) -> String {
    match slug {
        "cmyk" => "CMYK".to_string(),
        "lofi" => "Lo-fi".to_string(),
        "caramellatte" => "Caramel latte".to_string(),
        "united-kingdom-national-health-service-england" =>
            "United Kingdom NHS England".to_string(),
        "united-kingdom-national-health-service-scotland" =>
            "United Kingdom NHS Scotland".to_string(),
        "united-kingdom-national-health-service-wales-patients" =>
            "United Kingdom NHS Wales (Patients)".to_string(),
        "united-kingdom-national-health-service-wales-practitioners" =>
            "United Kingdom NHS Wales (Practitioners)".to_string(),
        s => {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        }
    }
}
