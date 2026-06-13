//! Configuration management for the Worker Service (MPI).
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// HTTP/gRPC server binding.
    pub server: ServerConfig,

    /// PostgreSQL connection settings.
    pub database: DatabaseConfig,

    /// Tantivy search-index settings.
    pub search: SearchConfig,

    /// Matching thresholds and weights.
    pub matching: MatchingConfig,

    /// Tracing/OpenTelemetry settings.
    pub observability: ObservabilityConfig,

    /// Event-streaming broker settings.
    pub streaming: StreamingConfig,
}

/// Server binding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address for the HTTP server (e.g. "0.0.0.0").
    pub host: String,
    /// HTTP port for the REST/FHIR API.
    pub port: u16,
    /// Port for the gRPC server.
    pub grpc_port: u16,
}

/// PostgreSQL connection-pool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// SeaORM/PostgreSQL connection URL.
    pub url: String,
    /// Maximum number of pooled connections.
    pub max_connections: u32,
    /// Minimum number of idle pooled connections to keep warm.
    pub min_connections: u32,
}

/// Search-index configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Filesystem path to the Tantivy index directory.
    pub index_path: String,
    /// Reader/writer cache budget in megabytes.
    pub cache_size_mb: usize,
}

/// Matching thresholds and reference scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// Minimum overall score for a pair to count as a match.
    pub threshold_score: f64,
    /// Score assigned to an exact component match.
    pub exact_match_score: f64,
    /// Score assigned to a fuzzy component match.
    pub fuzzy_match_score: f64,
}

/// Observability/telemetry configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// `service.name` attribute reported to the collector.
    pub service_name: String,
    /// OTLP collector endpoint URL.
    pub otlp_endpoint: String,
    /// Default log level (e.g. "info", "debug").
    pub log_level: String,
}

/// Event-streaming configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Streaming broker connection URL.
    pub broker_url: String,
    /// Topic to which worker events are published.
    pub topic: String,
}

impl Default for Config {
    /// Returns development-friendly defaults: localhost binding on port 8080,
    /// a local PostgreSQL URL, a relative search-index path, a 0.85 match
    /// threshold, and localhost telemetry/streaming endpoints.
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                grpc_port: 50051,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/worker_service".to_string(),
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
                service_name: "worker-service".to_string(),
                otlp_endpoint: "http://localhost:4317".to_string(),
                log_level: "info".to_string(),
            },
            streaming: StreamingConfig {
                broker_url: "localhost:9003".to_string(),
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
    pub fn from_env() -> crate::Result<Self> {
        // Pull a local .env into the process environment if one exists; ignore
        // the "not found" case so production (no .env) still works.
        dotenvy::dotenv().ok();
        // TODO: Implement environment variable loading
        Ok(Self::default())
    }
}
