//! Configuration management for the MPI system

use serde::{Deserialize, Serialize};

/// Main configuration structure
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub grpc_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub index_path: String,
    pub cache_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchingConfig {
    pub threshold_score: f64,
    pub exact_match_score: f64,
    pub fuzzy_match_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub service_name: String,
    pub otlp_endpoint: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub broker_url: String,
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
