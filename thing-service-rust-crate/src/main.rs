//! Thing Service binary — boot sequence for the REST API.
//!
//! `Config::from_env` → `db::create_connection` → `SearchEngine` →
//! matcher → `AppState` → `api::rest::serve`. Migrations are NOT auto-run.

// Always start with high quality coding conventions.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

// When we build for MUSL static, use faster memory allocator.
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use thing_service::{
    api::rest::{AppState, serve},
    config::Config,
    db::create_connection,
    matching::ThingMatcher,
    search::SearchEngine,
};
use tracing_subscriber::EnvFilter;

/// Process entry point. Delegates to [`run`] and translates any startup
/// error into a non-zero exit code after printing it to stderr.
#[tokio::main]
async fn main() -> std::process::ExitCode {
    if let Err(err) = run().await {
        eprintln!("thing-service failed to start: {err}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

/// Assemble and run the service: load config, init tracing, open the
/// database connection, build the search engine and matcher, wire them into
/// [`AppState`], and hand off to [`serve`].
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.observability.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        service = config.observability.service_name.as_str(),
        host = config.server.host.as_str(),
        port = config.server.port,
        "thing-service starting",
    );

    let db = create_connection(&config.database).await?;
    tracing::info!("database connected");

    let search_engine = SearchEngine::new(&config.search.index_path)?;
    tracing::info!(
        path = config.search.index_path.as_str(),
        "search index ready"
    );

    let matcher = ThingMatcher::new(config.matching.clone());
    let state = AppState::new(db, search_engine, matcher, config);

    serve(state).await?;
    Ok(())
}
