//! Configuration management for the Thing Service.
//!
//! One struct per tier (server / database / search / matching /
//! observability / streaming), all loaded via [`Config::from_env`].

use serde::{Deserialize, Serialize};

/// Top-level service configuration, one field per concern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP bind settings.
    pub server: ServerConfig,
    /// PostgreSQL connection settings.
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
    /// Bind address (e.g. `0.0.0.0`).
    pub host: String,
    /// REST/HTTP listen port.
    pub port: u16,
    /// gRPC listen port.
    pub grpc_port: u16,
}

/// PostgreSQL connection-pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Connection URL (`postgres://…`).
    pub url: String,
    /// Maximum pooled connections.
    pub max_connections: u32,
    /// Minimum idle pooled connections.
    pub min_connections: u32,
}

/// Tantivy full-text index configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Filesystem path for the on-disk index.
    pub index_path: String,
    /// Writer/reader cache budget in megabytes.
    pub cache_size_mb: usize,
}

/// Matcher tuning shared with [`crate::matching::ThingMatcher`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// `is_match` cut-off score in `[0.0, 1.0]`.
    pub threshold_score: f64,
}

/// Observability (tracing + OTLP export) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// `service.name` resource attribute.
    pub service_name: String,
    /// OTLP collector endpoint.
    pub otlp_endpoint: String,
    /// `tracing-subscriber` env-filter directive.
    pub log_level: String,
}

/// Event-streaming publisher configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Broker URL (used by the deferred Fluvio publisher).
    pub broker_url: String,
    /// Topic to publish thing events to.
    pub topic: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                grpc_port: 50051,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/thing_service".to_string(),
                max_connections: 10,
                min_connections: 2,
            },
            search: SearchConfig {
                index_path: "./data/search_index".to_string(),
                cache_size_mb: 512,
            },
            matching: MatchingConfig { threshold_score: 0.7 },
            observability: ObservabilityConfig {
                service_name: "thing-service".to_string(),
                otlp_endpoint: "http://localhost:4317".to_string(),
                log_level: "info".to_string(),
            },
            streaming: StreamingConfig {
                broker_url: "localhost:9003".to_string(),
                topic: "thing-events".to_string(),
            },
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Resolution order: explicit env var → `.env` file → struct default.
    pub fn from_env() -> crate::Result<Self> {
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
fn parse_env<T: std::str::FromStr>(name: &str) -> crate::Result<Option<T>>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(raw) => raw
            .parse::<T>()
            .map(Some)
            .map_err(|e| crate::Error::Config(format!("{name}={raw}: {e}"))),
        Err(_) => Ok(None),
    }
}
