//! Course Service binary — boot sequence for the REST API.
//!
//! `Config::from_env` → `db::create_connection` → `SearchEngine` →
//! matcher → `AppState` → `api::rest::serve`. Mirrors the person-service
//! boot sequence so operators get one shape to learn.
//!
//! Migrations are NOT auto-run; see `README.md` for the bring-up
//! sequence.

use course_service::{
    api::rest::{serve, AppState},
    config::Config,
    db::create_connection,
    matching::CourseMatcher,
    search::SearchEngine,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    if let Err(err) = run().await {
        eprintln!("course-service failed to start: {err}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.observability.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        service = config.observability.service_name.as_str(),
        host = config.server.host.as_str(),
        port = config.server.port,
        "course-service starting",
    );

    let db = create_connection(&config.database).await?;
    tracing::info!(url = mask_db_url(&config.database.url), "database connected");

    let search_engine = SearchEngine::new(&config.search.index_path)?;
    tracing::info!(path = config.search.index_path.as_str(), "search index ready");

    let matcher = CourseMatcher::new(config.matching.clone());
    let state = AppState::new(db, search_engine, matcher, config);

    serve(state).await?;
    Ok(())
}

fn mask_db_url(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            return format!("{scheme}<credentials>@{}", &url[at + 1..]);
        }
    }
    url.to_string()
}
