//! Person Service binary — boot sequence for the REST API.
//!
//! Resolves `Config` from `Config::from_env()` (reads `.env` then env
//! vars), opens the database pool, opens / creates the Tantivy search
//! index, constructs the matcher, builds `AppState`, and hands off to
//! `api::rest::serve(state)`. Listens on
//! `${SERVER_HOST}:${SERVER_PORT}` (defaults `0.0.0.0:8080`).
//!
//! Migrations are NOT auto-run. The database schema must already
//! exist before the binary starts. The provided SQL migrations live
//! under `migrations/` and can be applied with `sea-orm-cli migrate up`
//! or by psql-piping the `up.sql` files in numbered order. See
//! [`README.md`](../../README.md) for the bring-up sequence.

use person_service::{
    api::rest::{serve, AppState},
    config::Config,
    db::create_connection,
    matching::ProbabilisticMatcher,
    search::SearchEngine,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    if let Err(err) = run().await {
        eprintln!("person-service failed to start: {err}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    // Initialise tracing. Honour RUST_LOG via EnvFilter (already
    // mirrored into config.observability.log_level by Config::from_env).
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&config.observability.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).compact().init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        service = config.observability.service_name.as_str(),
        host = config.server.host.as_str(),
        port = config.server.port,
        "person-service starting",
    );

    let db = create_connection(&config.database).await?;
    tracing::info!(url = mask_db_url(&config.database.url), "database connected");

    let search_engine = SearchEngine::new(&config.search.index_path)?;
    tracing::info!(path = config.search.index_path.as_str(), "search index ready");

    let matcher = ProbabilisticMatcher::new(config.matching.clone());
    let state = AppState::new(db, search_engine, matcher, config);

    serve(state).await?;
    Ok(())
}

// Strip credentials before logging the connection URL.
fn mask_db_url(url: &str) -> String {
    if let Some(at) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            return format!("{scheme}<credentials>@{}", &url[at + 1..]);
        }
    }
    url.to_string()
}
