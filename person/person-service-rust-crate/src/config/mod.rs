//! Configuration management for the Person Service.
//!
//! [`Config`](crate::config::Config) is the single, fully-populated settings tree consumed at
//! startup (`main`). Each subsection ([`ServerConfig`](crate::config::ServerConfig),
//! [`DatabaseConfig`](crate::config::DatabaseConfig), …) groups related knobs. [`Config::default`](crate::config::Config::default)
//! supplies sensible local-dev values, and [`Config::from_env`](crate::config::Config::from_env) layers
//! environment variables / a `.env` file on top of those defaults.
//! Numeric env vars are parsed via the `parse_env` helper, which
//! surfaces a [`crate::Error::Config`] on malformed input.

use serde::{Deserialize, Serialize};

/// Top-level configuration tree for the service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Search configuration
    pub search: SearchConfig,

    /// Matching configuration
    pub matching: MatchingConfig,

    /// Observability configuration
    pub observability: ObservabilityConfig,

    /// Streaming configuration
    pub streaming: StreamingConfig,
}

/// HTTP/gRPC server bind settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address for the REST server (e.g. `0.0.0.0`).
    pub host: String,
    /// TCP port for the REST/HTTP server.
    pub port: u16,
    /// TCP port reserved for the (stubbed) gRPC server.
    pub grpc_port: u16,
}

/// PostgreSQL connection and pool settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Connection URL (`postgres://user:pass@host/db`).
    pub url: String,
    /// Maximum size of the connection pool.
    pub max_connections: u32,
    /// Minimum number of idle connections kept warm.
    pub min_connections: u32,
}

/// Tantivy full-text search settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Filesystem directory for the search index.
    pub index_path: String,
    /// Tantivy reader/writer cache budget in megabytes.
    pub cache_size_mb: usize,
}

/// Matching-engine thresholds and scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// Minimum overall score to treat two records as a match.
    pub threshold_score: f64,
    /// Score assigned to a deterministic exact match.
    pub exact_match_score: f64,
    /// Score assigned to a fuzzy (non-exact) match.
    pub fuzzy_match_score: f64,
}

/// Observability (tracing / OpenTelemetry / logging) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// `service.name` reported to the OTLP collector.
    pub service_name: String,
    /// OTLP collector endpoint (gRPC or HTTP).
    pub otlp_endpoint: String,
    /// `tracing-subscriber` log-level / filter directive.
    pub log_level: String,
}

/// Event-streaming (Fluvio/broker) settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Broker connection URL.
    pub broker_url: String,
    /// Topic to publish person events onto.
    pub topic: String,
}

impl Default for Config {
    /// Local-development defaults: binds `0.0.0.0:8080`, a localhost
    /// PostgreSQL URL, an on-disk search index, and the standard
    /// matching thresholds. [`Config::from_env`] overrides these.
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                grpc_port: 50051,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/person_service".to_string(),
                max_connections: 10,
                min_connections: 2,
            },
            search: SearchConfig {
                index_path: "./data/search_index".to_string(),
                cache_size_mb: 512,
            },
            matching: MatchingConfig {
                threshold_score: 0.85,
                exact_match_score: 1.0,
                fuzzy_match_score: 0.8,
            },
            observability: ObservabilityConfig {
                service_name: "person-service".to_string(),
                otlp_endpoint: "http://localhost:4317".to_string(),
                log_level: "info".to_string(),
            },
            streaming: StreamingConfig {
                broker_url: "localhost:9003".to_string(),
                topic: "person-events".to_string(),
            },
        }
    }
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Resolution order (highest wins): explicit env var → `.env` file
    /// (via `dotenvy`) → struct default. Variables read:
    ///
    /// | Env var | Field |
    /// |---|---|
    /// | `DATABASE_URL` | `database.url` |
    /// | `DATABASE_MAX_CONNECTIONS` | `database.max_connections` |
    /// | `DATABASE_MIN_CONNECTIONS` | `database.min_connections` |
    /// | `SERVER_HOST` | `server.host` |
    /// | `SERVER_PORT` | `server.port` |
    /// | `GRPC_PORT` | `server.grpc_port` |
    /// | `SEARCH_INDEX_PATH` | `search.index_path` |
    /// | `MATCHING_THRESHOLD` | `matching.threshold_score` |
    /// | `OTLP_SERVICE_NAME` | `observability.service_name` |
    /// | `OTLP_ENDPOINT` | `observability.otlp_endpoint` |
    /// | `RUST_LOG` | `observability.log_level` |
    ///
    /// Returns `Error::Config(_)` with the offending variable name
    /// when a value fails to parse (e.g. `SERVER_PORT=not-a-number`).
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

/// Parse an environment variable into `T`, returning `Ok(None)` when the
/// variable is unset and `Err(Error::Config)` when it is set but cannot
/// be parsed (the message includes the variable name and raw value).
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
