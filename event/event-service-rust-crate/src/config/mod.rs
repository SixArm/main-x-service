//! Configuration for the event-service runtime.
//!
//! [`Config`](crate::config::Config) is the root struct, composed of one
//! struct per concern (server, database, search, matching,
//! observability, streaming). It is `Serialize`/`Deserialize` so it can
//! be loaded from a config file and implements [`Default`] with
//! development-friendly localhost values.
//! [`Config::from_env`](crate::config::Config::from_env) is the entry
//! point used at startup.
//!
//! # Examples
//!
//! ```
//! use event_service::config::Config;
//!
//! let cfg = Config::default();
//! assert_eq!(cfg.server.port, 8080);
//! assert_eq!(cfg.matching.threshold_score, 0.85);
//! ```

use serde::{Deserialize, Serialize};

/// Root configuration aggregating every per-concern config block.
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

/// HTTP + gRPC server bind settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address for the REST server (e.g. `0.0.0.0`).
    pub host: String,
    /// TCP port for the REST/HTTP server.
    pub port: u16,
    /// TCP port for the gRPC server.
    pub grpc_port: u16,
}

/// `PostgreSQL` connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Connection string (`postgres://…`).
    pub url: String,
    /// Maximum size of the connection pool.
    pub max_connections: u32,
    /// Minimum number of idle connections kept warm.
    pub min_connections: u32,
}

/// Tantivy full-text search settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Filesystem directory holding the Tantivy index.
    pub index_path: String,
    /// Per-writer heap budget in megabytes.
    pub cache_size_mb: usize,
}

/// Tunable thresholds/weights for the matching engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    /// Minimum overall score that counts as a probabilistic match.
    pub threshold_score: f64,
    /// Score assigned to an exact component match.
    pub exact_match_score: f64,
    /// Score assigned to a fuzzy component match.
    pub fuzzy_match_score: f64,
}

/// Tracing + OpenTelemetry export settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// `service.name` attribute reported to the OTLP collector.
    pub service_name: String,
    /// OTLP collector endpoint (gRPC or HTTP).
    pub otlp_endpoint: String,
    /// Default `tracing` log-level filter.
    pub log_level: String,
}

/// Event-streaming broker settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Broker address for the streaming backend.
    pub broker_url: String,
    /// Topic on which CRUD/merge events are published.
    pub topic: String,
}

impl Default for Config {
    /// Development-friendly defaults: localhost binds, a local Postgres
    /// URL, a relative search-index path, and the standard matching
    /// thresholds (`0.85` cutoff).
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                grpc_port: 50051,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/event_service".to_string(),
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
                service_name: "event-service".to_string(),
                otlp_endpoint: "http://localhost:4317".to_string(),
                log_level: "info".to_string(),
            },
            streaming: StreamingConfig {
                broker_url: "localhost:9003".to_string(),
                topic: "event-events".to_string(),
            },
        }
    }
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns an error if a configuration value is present but invalid.
    pub fn from_env() -> crate::Result<Self> {
        dotenvy::dotenv().ok();
        // TODO: Implement environment variable loading
        Ok(Self::default())
    }
}
