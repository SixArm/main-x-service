//! Configuration management for the Worker Service.
//!
//! Defines the [`Config`](crate::config::Config) tree and its sub-structs
//! (server, database, search, matching, observability, streaming).
//! [`Config::default`](crate::config::Config) provides development-friendly
//! defaults; [`Config::from_env`](crate::config::Config::from_env) is the
//! production entry
//! point. All structs are serde-(de)serializable so configuration can also be
//! loaded from YAML/JSON files.

use serde::{Deserialize, Serialize};

/// Top-level configuration aggregating every subsystem's settings.
///
/// Each field is an independent sub-config so a subsystem can be tuned in
/// isolation. The whole tree is `serde`-(de)serializable, so it can be
/// loaded from a YAML/JSON config file as well as from defaults
/// ([`Config::default`]) or the environment ([`Config::from_env`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP/gRPC server binding (host, REST port, gRPC port).
    pub server: ServerConfig,

    /// `PostgreSQL` connection URL and pool sizing.
    pub database: DatabaseConfig,

    /// Tantivy search-index path and cache budget.
    pub search: SearchConfig,

    /// Matching thresholds and reference component scores.
    pub matching: MatchingConfig,

    /// Tracing/OpenTelemetry service name, OTLP endpoint, and log level.
    pub observability: ObservabilityConfig,

    /// Event-streaming broker URL and topic.
    pub streaming: StreamingConfig,
}

/// Server binding configuration.
///
/// Maps to env vars `SERVER_HOST` / `SERVER_PORT` (the gRPC port has no
/// documented env override yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address for the HTTP server. Env `SERVER_HOST`; default
    /// `0.0.0.0` (all interfaces, suitable for containers).
    pub host: String,
    /// HTTP port for the REST/FHIR API. Env `SERVER_PORT`; default `8080`.
    pub port: u16,
    /// Port for the gRPC server (Tonic stub). Default `50051`.
    pub grpc_port: u16,
}

/// `PostgreSQL` connection-pool configuration.
///
/// Maps to env vars `DATABASE_URL` / `DATABASE_MAX_CONNECTIONS` /
/// `DATABASE_MIN_CONNECTIONS`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SeaORM/PostgreSQL connection URL. Env `DATABASE_URL`; default points
    /// at a local `worker_service` database.
    pub url: String,
    /// Maximum number of pooled connections. Env
    /// `DATABASE_MAX_CONNECTIONS`; default `10`.
    pub max_connections: u32,
    /// Minimum number of idle pooled connections to keep warm. Env
    /// `DATABASE_MIN_CONNECTIONS`; default `2`.
    pub min_connections: u32,
}

/// Search-index configuration.
///
/// Maps to env var `SEARCH_INDEX_PATH`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Filesystem path to the Tantivy index directory. Env
    /// `SEARCH_INDEX_PATH`; default `./data/search_index`.
    pub index_path: String,
    /// Reader/writer cache budget in megabytes. Default `512`.
    pub cache_size_mb: usize,
}

/// Matching thresholds and reference scores.
///
/// `threshold_score` maps to env var `MATCHING_THRESHOLD`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// Minimum overall score in `[0.0, 1.0]` for a pair to count as a
    /// match. Env `MATCHING_THRESHOLD`; default `0.85`.
    pub threshold_score: f64,
    /// Score assigned to an exact component match. Default `1.0`.
    pub exact_match_score: f64,
    /// Score assigned to a fuzzy component match. Default `0.8`.
    pub fuzzy_match_score: f64,
}

/// Observability/telemetry configuration.
///
/// `log_level` is the fallback when `RUST_LOG` is unset (see
/// [`crate::observability::init_telemetry`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// `service.name` resource attribute reported to the OTLP collector.
    /// Default `worker-service`.
    pub service_name: String,
    /// OTLP collector endpoint URL. Default `http://localhost:4317` (the
    /// standard OTLP/gRPC port).
    pub otlp_endpoint: String,
    /// Default log level used when `RUST_LOG` is unset (e.g. `info`,
    /// `debug`). Default `info`.
    pub log_level: String,
}

/// Event-streaming configuration.
///
/// Targets the planned Fluvio event bus; the current publisher is
/// in-memory, so these settings are advisory until the durable bus lands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Streaming broker connection URL. Default `localhost:9003`.
    pub broker_url: String,
    /// Topic to which worker events are published. Default `worker-events`.
    pub topic: String,
}

impl Default for Config {
    /// Returns development-friendly defaults: localhost binding on port 8080,
    /// a local `PostgreSQL` URL, a relative search-index path, a 0.85 match
    /// threshold, and localhost telemetry/streaming endpoints.
    fn default() -> Self {
        Self {
            server: ServerConfig {
                // SERVER_HOST default: bind all interfaces (container-friendly).
                host: "0.0.0.0".to_string(),
                // SERVER_PORT default: REST/FHIR HTTP port.
                port: 8080,
                // gRPC server port (Tonic stub).
                grpc_port: 50051,
            },
            database: DatabaseConfig {
                // DATABASE_URL default: local Postgres `worker_service` db.
                url: "postgres://localhost/worker_service".to_string(),
                // DATABASE_MAX_CONNECTIONS default.
                max_connections: 10,
                // DATABASE_MIN_CONNECTIONS default (idle warm pool).
                min_connections: 2,
            },
            search: SearchConfig {
                // SEARCH_INDEX_PATH default: relative Tantivy index dir.
                index_path: "./data/search_index".to_string(),
                // Reader/writer cache budget in MiB.
                cache_size_mb: 512,
            },
            matching: MatchingConfig {
                // MATCHING_THRESHOLD default: probable-match cutoff.
                threshold_score: 0.85,
                // Reference score for an exact component match.
                exact_match_score: 1.0,
                // Reference score for a fuzzy component match.
                fuzzy_match_score: 0.8,
            },
            observability: ObservabilityConfig {
                // service.name resource attribute.
                service_name: "worker-service".to_string(),
                // OTLP/gRPC collector endpoint (standard port 4317).
                otlp_endpoint: "http://localhost:4317".to_string(),
                // RUST_LOG fallback level.
                log_level: "info".to_string(),
            },
            streaming: StreamingConfig {
                // Planned Fluvio broker URL.
                broker_url: "localhost:9003".to_string(),
                // Topic for published worker events.
                topic: "worker-events".to_string(),
            },
        }
    }
}

impl Config {
    /// Loads configuration, reading a `.env` file if present.
    ///
    /// Currently this returns [`Config::default`] after loading any `.env`
    /// file into the process environment; per-field environment overrides are
    /// not yet wired up (see the inline `TODO`).
    ///
    /// # Errors
    ///
    /// Returns an [`Err`] of [`crate::Error`] if configuration loading fails.
    /// In the current default-only implementation this path never returns an
    /// error, but the signature reserves the contract for the planned
    /// env-override wiring.
    pub fn from_env() -> crate::Result<Self> {
        // Pull a local .env into the process environment if one exists; ignore
        // the "not found" case so production (no .env) still works.
        dotenvy::dotenv().ok();
        // TODO: Implement environment variable loading
        Ok(Self::default())
    }
}
