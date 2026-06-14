//! Configuration management for the Place Service.
//!
//! One struct per tier (server / database / search / matching /
//! observability / streaming), all loaded from environment variables via
//! [`Config::from_env`].

use serde::{Deserialize, Serialize};

/// Top-level service configuration, one field per concern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP bind settings.
    pub server: ServerConfig,
    /// `PostgreSQL` connection settings.
    pub database: DatabaseConfig,
    /// Tantivy search-index settings.
    pub search: SearchConfig,
    /// Matcher threshold settings.
    pub matching: MatchingConfig,
    /// Tracing / OpenTelemetry settings.
    pub observability: ObservabilityConfig,
    /// Event-streaming settings.
    pub streaming: StreamingConfig,
}

/// Network bind configuration for the HTTP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address (e.g. `0.0.0.0`). Env `SERVER_HOST`; defaults to
    /// `0.0.0.0` so the container listens on every interface.
    pub host: String,
    /// REST/HTTP listen port. Env `SERVER_PORT`; defaults to `8080`.
    pub port: u16,
    /// gRPC listen port. Env `GRPC_PORT`; defaults to `50051` (the
    /// conventional gRPC port).
    pub grpc_port: u16,
}

/// `PostgreSQL` connection-pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Connection URL (`postgres://…`). Env `DATABASE_URL`; defaults to
    /// `postgres://localhost/place_service` for local development.
    pub url: String,
    /// Maximum pooled connections. Env `DATABASE_MAX_CONNECTIONS`;
    /// defaults to `10` (a modest ceiling that suits a single service
    /// instance against a development database).
    pub max_connections: u32,
    /// Minimum idle pooled connections. Env `DATABASE_MIN_CONNECTIONS`;
    /// defaults to `2` so a couple of warm connections are kept ready to
    /// avoid cold-start latency on the first requests.
    pub min_connections: u32,
}

/// Tantivy full-text index configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Filesystem path for the on-disk index. Env `SEARCH_INDEX_PATH`;
    /// defaults to `./data/search_index` (relative to the working dir).
    pub index_path: String,
    /// Writer/reader cache budget in megabytes. Defaults to `512`, the
    /// minimum Tantivy writer heap that keeps indexing throughput
    /// reasonable. No env override is wired up in [`Config::from_env`].
    pub cache_size_mb: usize,
}

/// Matcher tuning shared with [`crate::matching::PlaceMatcher`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// `is_match` cut-off score in `[0.0, 1.0]`. Env `MATCHING_THRESHOLD`;
    /// defaults to `0.7`, the "Possible/Probable" boundary above which a
    /// candidate pair is treated as a duplicate.
    pub threshold_score: f64,
}

/// Observability (tracing + OTLP export) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// `service.name` resource attribute attached to every exported span.
    /// Env `OTLP_SERVICE_NAME`; defaults to `place-service`.
    pub service_name: String,
    /// OTLP collector endpoint. Env `OTLP_ENDPOINT`; defaults to
    /// `http://localhost:4317` (the standard OTLP/gRPC port).
    pub otlp_endpoint: String,
    /// `tracing-subscriber` env-filter directive. Env `RUST_LOG`;
    /// defaults to `info`.
    pub log_level: String,
}

/// Event-streaming publisher configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Broker URL (used by the deferred Fluvio publisher). Defaults to
    /// `localhost:9003`; no env override is wired up in
    /// [`Config::from_env`] because the durable publisher is not yet
    /// active (the service uses an in-memory event stream).
    pub broker_url: String,
    /// Topic to publish place events to. Defaults to `place-events`; no
    /// env override is wired up in [`Config::from_env`].
    pub topic: String,
}

impl Default for Config {
    /// Build the baseline configuration used when no env vars are set.
    ///
    /// [`Config::from_env`] starts from this and overlays env overrides,
    /// so these literals are the documented defaults for every field.
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                grpc_port: 50051,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/place_service".to_string(),
                max_connections: 10,
                min_connections: 2,
            },
            search: SearchConfig {
                index_path: "./data/search_index".to_string(),
                cache_size_mb: 512,
            },
            matching: MatchingConfig {
                threshold_score: 0.7,
            },
            observability: ObservabilityConfig {
                service_name: "place-service".to_string(),
                otlp_endpoint: "http://localhost:4317".to_string(),
                log_level: "info".to_string(),
            },
            streaming: StreamingConfig {
                broker_url: "localhost:9003".to_string(),
                topic: "place-events".to_string(),
            },
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Resolution order: explicit env var → `.env` file → struct default.
    /// String fields use `std::env::var` directly (any value is accepted);
    /// numeric fields go through [`parse_env`], which surfaces a parse
    /// failure as [`Error::Config`](crate::Error::Config). Not every field
    /// has an env override — `search.cache_size_mb` and the entire
    /// `streaming` tier are default-only.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Config`](crate::Error::Config) when a numeric env
    /// var (`DATABASE_MAX_CONNECTIONS`, `DATABASE_MIN_CONNECTIONS`,
    /// `SERVER_PORT`, `GRPC_PORT`, `MATCHING_THRESHOLD`) is set but cannot
    /// be parsed into its target type.
    pub fn from_env() -> crate::Result<Self> {
        // Best-effort load of a local `.env`; absence is not an error.
        dotenvy::dotenv().ok();
        let mut config = Self::default();

        if let Ok(v) = std::env::var("DATABASE_URL") {
            config.database.url = v;
        }
        if let Some(v) = parse_env::<u32>("DATABASE_MAX_CONNECTIONS")? {
            config.database.max_connections = v;
        }
        if let Some(v) = parse_env::<u32>("DATABASE_MIN_CONNECTIONS")? {
            config.database.min_connections = v;
        }
        if let Ok(v) = std::env::var("SERVER_HOST") {
            config.server.host = v;
        }
        if let Some(v) = parse_env::<u16>("SERVER_PORT")? {
            config.server.port = v;
        }
        if let Some(v) = parse_env::<u16>("GRPC_PORT")? {
            config.server.grpc_port = v;
        }
        if let Ok(v) = std::env::var("SEARCH_INDEX_PATH") {
            config.search.index_path = v;
        }
        if let Some(v) = parse_env::<f64>("MATCHING_THRESHOLD")? {
            config.matching.threshold_score = v;
        }
        if let Ok(v) = std::env::var("OTLP_SERVICE_NAME") {
            config.observability.service_name = v;
        }
        if let Ok(v) = std::env::var("OTLP_ENDPOINT") {
            config.observability.otlp_endpoint = v;
        }
        if let Ok(v) = std::env::var("RUST_LOG") {
            config.observability.log_level = v;
        }
        Ok(config)
    }
}

/// Parse environment variable `name` into `T`, returning `Ok(None)` when
/// unset and `Err(Error::Config)` when set but unparseable.
///
/// The `Option` return lets callers distinguish "unset, keep the default"
/// (`None`) from "set to a value" (`Some(v)`) without conflating an absent
/// var with a parse failure.
///
/// # Errors
///
/// Returns [`Error::Config`](crate::Error::Config) when the variable is
/// present but `T::from_str` rejects its value; the message embeds the
/// `name`, the raw string, and the parser's own error for diagnosis.
fn parse_env<T: std::str::FromStr>(name: &str) -> crate::Result<Option<T>>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        // Present: attempt the parse, mapping any failure to a config error.
        Ok(raw) => raw
            .parse::<T>()
            .map(Some)
            .map_err(|e| crate::Error::Config(format!("{name}={raw}: {e}"))),
        // Absent (or non-UTF-8): treat as "unset" so the default stands.
        Err(_) => Ok(None),
    }
}
