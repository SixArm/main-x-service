//! Configuration management for the Person Service.
//!
//! [`Config`](crate::config::Config) is the single, fully-populated settings tree consumed at
//! startup (`main`). Each subsection ([`ServerConfig`](crate::config::ServerConfig),
//! [`DatabaseConfig`](crate::config::DatabaseConfig), …) groups related knobs. [`Config::default`](crate::config::Config::default)
//! supplies sensible local-dev values, and [`Config::from_env`](crate::config::Config::from_env) layers
//! environment variables / a `.env` file on top of those defaults.
//! Typed env vars are parsed via the `parse_setting` helper, which
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

/// `PostgreSQL` connection and pool settings.
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
    /// `PostgreSQL` URL, an on-disk search index, and the standard
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
    /// Load configuration from the process environment, layered over
    /// [`Config::default`].
    ///
    /// Resolution order (highest wins): explicit environment variable →
    /// `.env` file (loaded best-effort via `dotenvy`; a missing file is
    /// not an error) → struct default. A variable set to a blank or
    /// whitespace-only value counts as **unset**, so an accidentally
    /// empty `SERVER_HOST` falls back to the default rather than
    /// binding to nothing.
    ///
    /// | Env var | Field | Type |
    /// |---|---|---|
    /// | `DATABASE_URL` | `database.url` | string |
    /// | `DATABASE_MAX_CONNECTIONS` | `database.max_connections` | `u32` |
    /// | `DATABASE_MIN_CONNECTIONS` | `database.min_connections` | `u32` |
    /// | `SERVER_HOST` | `server.host` | string |
    /// | `SERVER_PORT` | `server.port` | `u16` |
    /// | `GRPC_PORT` | `server.grpc_port` | `u16` |
    /// | `SEARCH_INDEX_PATH` | `search.index_path` | string |
    /// | `SEARCH_CACHE_SIZE_MB` | `search.cache_size_mb` | `usize` |
    /// | `MATCHING_THRESHOLD` | `matching.threshold_score` | `f64` |
    /// | `OTLP_SERVICE_NAME` | `observability.service_name` | string |
    /// | `OTLP_ENDPOINT` | `observability.otlp_endpoint` | string |
    /// | `RUST_LOG` | `observability.log_level` | string |
    /// | `STREAMING_BROKER_URL` | `streaming.broker_url` | string |
    /// | `STREAMING_TOPIC` | `streaming.topic` | string |
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`], naming the offending variable
    /// and its raw value, when a typed value fails to parse (e.g.
    /// `SERVER_PORT=not-a-number`, or a port above `65535`). A bad value
    /// is **refused rather than silently defaulted**: booting on a port
    /// or pool size the operator did not ask for is worse than failing
    /// at startup.
    pub fn from_env() -> crate::Result<Self> {
        // Best-effort load of a local `.env`; absence is not an error.
        dotenvy::dotenv().ok();
        Self::from_source(|name| std::env::var(name).ok())
    }

    /// The pure overlay behind [`from_env`](Self::from_env): applies the
    /// values `lookup` returns on top of [`Config::default`].
    ///
    /// Split out so the variable-to-field mapping is unit-testable
    /// without touching the process environment. That matters twice
    /// over: `std::env::set_var` is `unsafe` in the 2024 edition, which
    /// this crate forbids, and process environment is global mutable
    /// state that would make parallel tests flaky.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Config`] when a typed value fails to
    /// parse — see [`from_env`](Self::from_env).
    pub fn from_source(lookup: impl Fn(&str) -> Option<String>) -> crate::Result<Self> {
        let mut config = Self::default();
        // A blank value is treated as unset throughout.
        let get = |name: &str| lookup(name).filter(|v| !v.trim().is_empty());

        if let Some(v) = get("DATABASE_URL") {
            config.database.url = v;
        }
        if let Some(v) = parse_setting::<u32>("DATABASE_MAX_CONNECTIONS", get("DATABASE_MAX_CONNECTIONS"))? {
            config.database.max_connections = v;
        }
        if let Some(v) = parse_setting::<u32>("DATABASE_MIN_CONNECTIONS", get("DATABASE_MIN_CONNECTIONS"))? {
            config.database.min_connections = v;
        }

        if let Some(v) = get("SERVER_HOST") {
            config.server.host = v;
        }
        if let Some(v) = parse_setting::<u16>("SERVER_PORT", get("SERVER_PORT"))? {
            config.server.port = v;
        }
        if let Some(v) = parse_setting::<u16>("GRPC_PORT", get("GRPC_PORT"))? {
            config.server.grpc_port = v;
        }

        if let Some(v) = get("SEARCH_INDEX_PATH") {
            config.search.index_path = v;
        }
        if let Some(v) = parse_setting::<usize>("SEARCH_CACHE_SIZE_MB", get("SEARCH_CACHE_SIZE_MB"))? {
            config.search.cache_size_mb = v;
        }

        if let Some(v) = parse_setting::<f64>("MATCHING_THRESHOLD", get("MATCHING_THRESHOLD"))? {
            config.matching.threshold_score = v;
        }

        if let Some(v) = get("OTLP_SERVICE_NAME") {
            config.observability.service_name = v;
        }
        if let Some(v) = get("OTLP_ENDPOINT") {
            config.observability.otlp_endpoint = v;
        }
        if let Some(v) = get("RUST_LOG") {
            config.observability.log_level = v;
        }

        if let Some(v) = get("STREAMING_BROKER_URL") {
            config.streaming.broker_url = v;
        }
        if let Some(v) = get("STREAMING_TOPIC") {
            config.streaming.topic = v;
        }

        Ok(config)
    }
}

/// Parse an already-resolved environment value into `T`.
///
/// `Ok(None)` when the variable was unset (or blank);
/// [`crate::Error::Config`] when it was set but unparseable — the
/// message carries the variable name **and** the raw value, so a
/// startup failure says exactly which knob to fix.
fn parse_setting<T: std::str::FromStr>(name: &str, raw: Option<String>) -> crate::Result<Option<T>>
where
    T::Err: std::fmt::Display,
{
    match raw {
        None => Ok(None),
        Some(raw) => raw
            .trim()
            .parse::<T>()
            .map(Some)
            .map_err(|e| crate::Error::Config(format!("{name}={raw}: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    /// Build a lookup over a fixed `(name, value)` table — the pure
    /// stand-in for the process environment.
    fn source<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_string())
        }
    }

    /// With nothing set, every field keeps its default.
    #[test]
    fn empty_environment_yields_the_defaults() {
        let defaults = Config::default();
        let config = Config::from_source(|_| None).expect("no values to parse");

        assert_eq!(config.database.url, defaults.database.url);
        assert_eq!(config.database.max_connections, defaults.database.max_connections);
        assert_eq!(config.server.host, defaults.server.host);
        assert_eq!(config.server.port, defaults.server.port);
        assert_eq!(config.search.index_path, defaults.search.index_path);
        assert!((config.matching.threshold_score - defaults.matching.threshold_score).abs() < f64::EPSILON);
        assert_eq!(config.observability.log_level, defaults.observability.log_level);
        assert_eq!(config.streaming.topic, defaults.streaming.topic);
    }

    /// Every documented variable reaches its field.
    #[test]
    fn every_variable_overrides_its_field() {
        let config = Config::from_source(source(&[
            ("DATABASE_URL", "postgres://db/app"),
            ("DATABASE_MAX_CONNECTIONS", "42"),
            ("DATABASE_MIN_CONNECTIONS", "7"),
            ("SERVER_HOST", "127.0.0.1"),
            ("SERVER_PORT", "9090"),
            ("GRPC_PORT", "50100"),
            ("SEARCH_INDEX_PATH", "/var/lib/index"),
            ("SEARCH_CACHE_SIZE_MB", "256"),
            ("MATCHING_THRESHOLD", "0.42"),
            ("OTLP_SERVICE_NAME", "svc-under-test"),
            ("OTLP_ENDPOINT", "http://collector:4317"),
            ("RUST_LOG", "debug"),
            ("STREAMING_BROKER_URL", "broker:9003"),
            ("STREAMING_TOPIC", "topic-under-test"),
        ]))
        .expect("all values parse");

        assert_eq!(config.database.url, "postgres://db/app");
        assert_eq!(config.database.max_connections, 42);
        assert_eq!(config.database.min_connections, 7);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.grpc_port, 50100);
        assert_eq!(config.search.index_path, "/var/lib/index");
        assert_eq!(config.search.cache_size_mb, 256);
        assert!((config.matching.threshold_score - 0.42).abs() < f64::EPSILON);
        assert_eq!(config.observability.service_name, "svc-under-test");
        assert_eq!(config.observability.otlp_endpoint, "http://collector:4317");
        assert_eq!(config.observability.log_level, "debug");
        assert_eq!(config.streaming.broker_url, "broker:9003");
        assert_eq!(config.streaming.topic, "topic-under-test");
    }

    /// A blank or whitespace-only value counts as unset — an empty
    /// `SERVER_HOST` must not bind the server to nothing, and an empty
    /// numeric must not be a parse error.
    #[test]
    fn blank_values_are_treated_as_unset() {
        let defaults = Config::default();
        let config = Config::from_source(source(&[
            ("DATABASE_URL", ""),
            ("SERVER_HOST", "   "),
            ("SERVER_PORT", ""),
        ]))
        .expect("blank values are unset, not malformed");

        assert_eq!(config.database.url, defaults.database.url);
        assert_eq!(config.server.host, defaults.server.host);
        assert_eq!(config.server.port, defaults.server.port);
    }

    /// A malformed value is refused, and the error names the variable
    /// and the offending raw value.
    #[test]
    fn malformed_values_are_refused_by_name() {
        let err = Config::from_source(source(&[("SERVER_PORT", "not-a-number")]))
            .expect_err("a non-numeric port is refused");
        let message = err.to_string();
        assert!(message.contains("SERVER_PORT"), "{message}");
        assert!(message.contains("not-a-number"), "{message}");

        // Out of range for the field's type, not merely non-numeric.
        assert!(
            Config::from_source(source(&[("SERVER_PORT", "70000")])).is_err(),
            "a port above 65535 does not fit u16"
        );
        assert!(
            Config::from_source(source(&[("DATABASE_MAX_CONNECTIONS", "-1")])).is_err(),
            "a negative pool size does not fit u32"
        );
        assert!(
            Config::from_source(source(&[("MATCHING_THRESHOLD", "high")])).is_err(),
            "a non-numeric threshold is refused"
        );
    }

    /// Surrounding whitespace is tolerated on a typed value (a `.env`
    /// line like `SERVER_PORT = 9090 ` is a common shape).
    #[test]
    fn typed_values_tolerate_surrounding_whitespace() {
        let config = Config::from_source(source(&[("SERVER_PORT", " 9090 ")]))
            .expect("whitespace is trimmed before parsing");
        assert_eq!(config.server.port, 9090);
    }
}
