//! Observability bootstrap: structured logging **and real OpenTelemetry
//! OTLP export** (repo `tasks.md` PRO-H9).
//!
//! Ported from [`link-graph-service`]'s `src/observability.rs` — the
//! family's first working OTLP pipeline (`agents/share/rust-tracing-opentelemetry-stack.md`).
//! This crate previously carried a stub (`src/observability/mod.rs`) that
//! built an `OTel` [`Resource`] and then installed a plain JSON `tracing`
//! subscriber, with the exporter and the bridge layer commented out
//! behind `// TODO: Initialize OTLP exporter` — dead scaffolding that was
//! never wired into [`crate::app::App`]'s `Hooks` impl at all. This module
//! replaces it outright rather than filling in the stub in place, because
//! the stub's shape (a bare `tracing_subscriber::fmt` layer with no
//! bridge) cannot be extended into the real pipeline without discarding
//! most of it.
//!
//! [`link-graph-service`]: https://github.com/sixarm/main-x-service/tree/main/link/link-graph-service-with-loco
//!
//! ## What it installs
//!
//! [`init`] builds one [`Resource`] (`service.name` + `service.version`,
//! plus whatever the SDK's own detectors add) and hangs three things off it:
//!
//! 1. a `tracing-subscriber` **fmt layer** in the format loco's own config
//!    asks for, filtered by loco's own [`EnvFilter`] policy (so `RUST_LOG`
//!    and the `logger:` config block keep working exactly as before);
//! 2. a [`tracing_opentelemetry`] **bridge layer** over an OTLP/gRPC
//!    [`SdkTracerProvider`], so every `tracing` span becomes an exported
//!    `OTel` span;
//! 3. an OTLP/gRPC [`SdkMeterProvider`] set as the global meter provider,
//!    read periodically, feeding [`http_metrics`].
//!
//! Local logs and remote export are **both** active — not either/or.
//!
//! ## Configuration
//!
//! | Variable | Default | Effect |
//! |---|---|---|
//! | `RUST_LOG` | loco's whitelist at the configured level | `EnvFilter` directive |
//! | `OTLP_SERVICE_NAME` | `person-service` | `service.name` resource attribute |
//! | `OTLP_ENDPOINT` | `http://localhost:4317` | OTLP/gRPC collector endpoint |
//!
//! These three variables are **deliberately not `PERSON_`-prefixed**,
//! matching link-graph-service's own choice and
//! `agents/share/rust-tracing-opentelemetry-stack.md`'s "Configuration"
//! table verbatim — unlike the per-service `<ENTITY>_REQUIRE_AUTH`
//! activation-flag convention, OTLP export has no such flag (see below)
//! and the shared doc fixes one family-wide variable name rather than a
//! `<ENTITY>_`-prefixed one per crate. This crate's own
//! [`crate::config::ObservabilityConfig`] happens to read the same three
//! variable names into a `Config.observability` substructure that nothing
//! in this pipeline consults (it predates this module and was already
//! unused before this change — see that struct's doc comment); the two
//! are independent readers of the same three variables, not a layering.
//!
//! The defaults are the shared doc's, which means export is **on by
//! default** — there is no `<ENTITY>_…`-style activation flag, deliberately.
//! Setting `OTLP_ENDPOINT` to the **empty string** disables export and
//! leaves only local logging; that is the one escape hatch, and it is a
//! value of the documented variable rather than a new one.
//!
//! `RUST_LOG` is **not** only a log-verbosity knob once the bridge layer is
//! installed: the filter sits above both sinks, so it also decides what is
//! exported. Loco's default is a module whitelist, which keeps the trace
//! stream to this service's own spans; a blanket `RUST_LOG=trace` would ship
//! every internal `h2` / `hyper` / `sqlx` span to the collector as well.
//! That is a volume problem, not a correctness one, but it is worth knowing
//! before turning the dial during an incident.
//!
//! ## Why booting without a collector is safe
//!
//! Verified against the SDK version pinned here, not assumed:
//!
//! - the tonic channel is built with `connect_lazy()`, so **nothing dials
//!   the collector during boot** — [`build_span_exporter`] cannot block or
//!   fail on an unreachable endpoint;
//! - the batch span processor and the periodic metric reader each own a
//!   dedicated OS thread, so a stalled export never occupies a Tokio worker;
//! - a full span queue **drops**, it does not block the emitting request.
//!
//! A service with no collector therefore starts, serves, and shuts down
//! identically — it just logs an export error every batch interval **while
//! it is actually producing spans**, which is the honest signal for
//! "someone configured an endpoint that is not there".

use std::env;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use opentelemetry::metrics::Histogram;
use opentelemetry::trace::{TraceContextExt, TracerProvider as _};
use opentelemetry::{KeyValue, global};
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// The default OTLP/gRPC collector endpoint
/// (`agents/share/rust-tracing-opentelemetry-stack.md`).
pub const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4317";

/// The `service.name` used when `OTLP_SERVICE_NAME` is unset.
pub const DEFAULT_SERVICE_NAME: &str = "person-service";

/// The boxed formatting layer type loco's
/// [`init_layer`](loco_rs::logger::init_layer) returns, accepted by
/// [`init`] so the log format a deployment configured is reused verbatim
/// rather than re-derived here.
pub type FmtLayer =
    Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync + 'static>;

/// Response header carrying the W3C trace context of the request that
/// produced it, so an operator can jump straight from a response to its
/// trace ("every response carries a `traceparent` header when an OTLP
/// endpoint is configured" — the shared doc's *Where to look first*).
const TRACEPARENT: HeaderName = HeaderName::from_static("traceparent");

/// Resolved telemetry configuration (see the module docs' table).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryConfig {
    /// `service.name` resource attribute (`OTLP_SERVICE_NAME`).
    pub service_name: String,
    /// OTLP/gRPC collector endpoint (`OTLP_ENDPOINT`). `None` when the
    /// variable is set to the empty string, which disables export.
    pub endpoint: Option<String>,
}

impl TelemetryConfig {
    /// Read the configuration from the process environment.
    ///
    /// An **unset** `OTLP_ENDPOINT` means the documented default
    /// (`http://localhost:4317`), i.e. export on. An `OTLP_ENDPOINT` set to
    /// the empty string means export off — the deliberate escape hatch for
    /// a deployment (or a developer) with no collector, expressed as a
    /// value of the documented variable rather than as a second flag.
    #[must_use]
    pub fn from_env() -> Self {
        Self::resolve(
            env::var("OTLP_SERVICE_NAME").ok().as_deref(),
            env::var("OTLP_ENDPOINT").ok().as_deref(),
        )
    }

    /// The pure half of [`from_env`](Self::from_env): the same rules over
    /// explicit values.
    ///
    /// Split out so the precedence can be unit-tested without mutating the
    /// process environment — this crate is `#![forbid(unsafe_code)]`, and
    /// since Rust 2024 `env::set_var` is `unsafe`, so a test that reached
    /// for the environment could not compile here at all. That is a better
    /// constraint than it sounds: the rules are now testable in parallel,
    /// with no shared global to serialise on.
    #[must_use]
    pub fn resolve(service_name: Option<&str>, endpoint: Option<&str>) -> Self {
        let service_name = service_name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_SERVICE_NAME)
            .to_string();
        let endpoint = match endpoint.map(str::trim) {
            // Set-but-empty is the documented "export off" escape hatch.
            Some("") => None,
            Some(value) => Some(value.to_string()),
            None => Some(DEFAULT_OTLP_ENDPOINT.to_string()),
        };
        Self {
            service_name,
            endpoint,
        }
    }

    /// Whether OTLP export is enabled for this configuration.
    #[must_use]
    pub fn export_enabled(&self) -> bool {
        self.endpoint.is_some()
    }

    /// The `OTel` [`Resource`] describing this process.
    ///
    /// Deliberately minimal: `service.name` and `service.version` on top
    /// of the SDK's default detectors (which supply the `telemetry.sdk.*`
    /// attributes and merge `OTEL_RESOURCE_ATTRIBUTES`).
    /// `service.instance.id` is **not** set —
    /// a per-boot random id would make every restart look like a new
    /// instance forever, and a collector or a deployment that genuinely
    /// needs one can supply it through `OTEL_RESOURCE_ATTRIBUTES`, which
    /// the SDK's default detectors already merge in.
    #[must_use]
    pub fn resource(&self) -> Resource {
        Resource::builder()
            .with_service_name(self.service_name.clone())
            .with_attributes([KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_VERSION,
                env!("CARGO_PKG_VERSION"),
            )])
            .build()
    }
}

/// Build the OTLP/gRPC **span** exporter for `endpoint`.
///
/// Non-blocking: the tonic channel underneath is created with
/// `connect_lazy()`, so this returns successfully whether or not a
/// collector is listening. That property is what makes export-on-by-default
/// safe, and it is pinned by a unit test.
///
/// # Errors
///
/// Returns the exporter builder's error if `endpoint` is not a usable URI.
pub fn build_span_exporter(
    endpoint: &str,
) -> Result<SpanExporter, opentelemetry_otlp::ExporterBuildError> {
    SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(10))
        .build()
}

/// Build the OTLP/gRPC **metric** exporter for `endpoint`. Same laziness
/// guarantee as [`build_span_exporter`].
///
/// # Errors
///
/// Returns the exporter builder's error if `endpoint` is not a usable URI.
pub fn build_metric_exporter(
    endpoint: &str,
) -> Result<MetricExporter, opentelemetry_otlp::ExporterBuildError> {
    MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(10))
        .build()
}

/// Build a batch-exporting [`SdkTracerProvider`] for `config`.
///
/// Exposed (rather than kept private to [`init`]) so a test can drive the
/// whole pipeline — bridge layer, protobuf encoding, gRPC — against an
/// in-process collector **without** touching the global subscriber, which
/// can only be installed once per process.
///
/// # Errors
///
/// Propagates an exporter build failure.
pub fn build_tracer_provider(
    config: &TelemetryConfig,
) -> Result<Option<SdkTracerProvider>, opentelemetry_otlp::ExporterBuildError> {
    let Some(endpoint) = config.endpoint.as_deref() else {
        return Ok(None);
    };
    let exporter = build_span_exporter(endpoint)?;
    Ok(Some(
        SdkTracerProvider::builder()
            .with_resource(config.resource())
            .with_batch_exporter(exporter)
            .build(),
    ))
}

/// Build a periodically-exporting [`SdkMeterProvider`] for `config`.
///
/// # Errors
///
/// Propagates an exporter build failure.
pub fn build_meter_provider(
    config: &TelemetryConfig,
) -> Result<Option<SdkMeterProvider>, opentelemetry_otlp::ExporterBuildError> {
    let Some(endpoint) = config.endpoint.as_deref() else {
        return Ok(None);
    };
    let exporter = build_metric_exporter(endpoint)?;
    Ok(Some(
        SdkMeterProvider::builder()
            .with_resource(config.resource())
            .with_periodic_exporter(exporter)
            .build(),
    ))
}

/// The installed providers, held so shutdown can flush them.
///
/// Dropping this without calling [`Telemetry::shutdown`] loses whatever is
/// still queued, which is why [`crate::app::App`] holds it in a `OnceLock`
/// and shuts it down from loco's `on_shutdown` hook.
#[derive(Debug, Default)]
pub struct Telemetry {
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
}

impl Telemetry {
    /// Whether OTLP export is actually running.
    #[must_use]
    pub fn exporting(&self) -> bool {
        self.tracer_provider.is_some()
    }

    /// Flush anything queued, without tearing the providers down.
    pub fn force_flush(&self) {
        if let Some(provider) = &self.tracer_provider {
            let _ = provider.force_flush();
        }
        if let Some(provider) = &self.meter_provider {
            let _ = provider.force_flush();
        }
    }

    /// Flush and tear down both providers. Call once, during graceful
    /// shutdown — a batch processor holds up to one scheduled delay's worth
    /// of spans, so exiting without this silently loses the last few
    /// seconds of a trace, which is exactly the window an operator looking
    /// at a crash cares about.
    pub fn shutdown(&self) {
        if let Some(provider) = &self.tracer_provider
            && let Err(error) = provider.shutdown()
        {
            tracing::warn!(%error, "tracer provider shutdown failed");
        }
        if let Some(provider) = &self.meter_provider
            && let Err(error) = provider.shutdown()
        {
            tracing::warn!(%error, "meter provider shutdown failed");
        }
    }
}

/// The `OpenTelemetry` crates whose own diagnostics report a failing export.
const EXPORTER_DIAGNOSTIC_TARGETS: [&str; 3] =
    ["opentelemetry", "opentelemetry_sdk", "opentelemetry-otlp"];

/// Widen `filter` so the exporter's own `warn`/`error` diagnostics are not
/// swallowed.
///
/// Found the hard way in link-graph-service, by booting that service with
/// no collector and watching **nothing** appear in the log: loco's default
/// filter is a fixed module whitelist, and `opentelemetry*` is not on it,
/// so every failed export was invisible. A silent exporter is worse than
/// no exporter — it looks like it is working.
///
/// Only applied when the filter came from that built-in whitelist. If an
/// operator set `RUST_LOG` or `logger.override_filter`, their directives are
/// left exactly as written: overriding an explicit choice to quiet these
/// targets would be the same class of surprise in the other direction.
#[must_use]
pub fn with_exporter_diagnostics(
    filter: tracing_subscriber::EnvFilter,
    operator_supplied: bool,
) -> tracing_subscriber::EnvFilter {
    if operator_supplied {
        return filter;
    }
    EXPORTER_DIAGNOSTIC_TARGETS
        .iter()
        .fold(filter, |filter, target| {
            match format!("{target}=warn").parse() {
                Ok(directive) => filter.add_directive(directive),
                Err(_) => filter,
            }
        })
}

/// Install process-wide logging plus OTLP export.
///
/// `env_filter` and `fmt_layer` come from loco (`logger::init_env_filter`
/// and `logger::init_layer`), so overriding loco's logger does not quietly
/// change the filter policy or the log format a deployment already
/// configured — the *only* difference from loco's own `logger::init` is the
/// extra OpenTelemetry layer.
///
/// Must be called exactly once: the global `tracing` subscriber can only be
/// set a single time per process.
///
/// # Errors
///
/// Returns an error if an exporter cannot be built. A collector being
/// unreachable is **not** such an error — see the module docs.
pub fn init(
    config: &TelemetryConfig,
    env_filter: tracing_subscriber::EnvFilter,
    fmt_layer: Option<FmtLayer>,
) -> Result<Telemetry, opentelemetry_otlp::ExporterBuildError> {
    let tracer_provider = build_tracer_provider(config)?;
    let meter_provider = build_meter_provider(config)?;

    let otel_layer = tracer_provider.as_ref().map(|provider| {
        tracing_opentelemetry::layer().with_tracer(provider.tracer(config.service_name.clone()))
    });

    // Layer order matches loco's own `logger::init`: sinks first, the
    // `EnvFilter` last, so it filters everything beneath it.
    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(otel_layer)
        .with(env_filter)
        .init();

    if let Some(provider) = &tracer_provider {
        global::set_tracer_provider(provider.clone());
    }
    if let Some(provider) = &meter_provider {
        global::set_meter_provider(provider.clone());
    }

    Ok(Telemetry {
        tracer_provider,
        meter_provider,
    })
}

/// The HTTP server instruments, registered against the global meter
/// provider on first use.
struct HttpMetrics {
    duration: Histogram<f64>,
}

fn http_metrics() -> &'static HttpMetrics {
    static METRICS: OnceLock<HttpMetrics> = OnceLock::new();
    METRICS.get_or_init(|| {
        let meter = global::meter("person-service");
        HttpMetrics {
            duration: meter
                .f64_histogram("http.server.request.duration")
                .with_description("Duration of inbound HTTP requests.")
                .with_unit("s")
                .build(),
        }
    })
}

/// Per-request tracing + metrics middleware.
///
/// Opens one `tracing` span per request (which the bridge layer turns into
/// an exported `OTel` span), records the request duration on the
/// `http.server.request.duration` histogram, and stamps the W3C
/// `traceparent` of the request's span onto the response.
///
/// The route is **not** used as a metric attribute: this service's paths
/// embed record UUIDs (`/api/persons/{id}`, `/api/persons/{id}/links/{link_id}`,
/// FHIR `/fhir/Patient/{id}`, …), so a per-path label would be unbounded
/// cardinality — the same reasoning link-graph-service applied to its own
/// `EntityRef`-shaped paths, here for plain UUID path segments instead.
///
/// Wired onto **both** of this crate's router-construction surfaces —
/// [`crate::api::rest::create_router`] (the hand-rolled `Router` used by
/// the DB-gated integration tests) and [`crate::app::App::after_routes`]
/// (the loco-booted production router) — so tracing behaves identically
/// regardless of which one a caller builds. link-graph-service has only
/// one router-construction path (pure loco), so this two-surface wiring
/// has no analogue there; see the crate's `AGENTS.md` / spec for the
/// deviation note.
pub async fn trace_mw(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    // A fixed span name, not the semconv-preferred `{method} {route}`: a
    // `from_fn` layer runs outside routing, so no `MatchedPath` is available
    // yet, and the raw path is not usable as a name here — this service's
    // paths embed record UUIDs.
    let span = tracing::info_span!(
        "http.server.request",
        otel.kind = "server",
        http.request.method = %method,
        // Declared empty so it can be recorded once the response is known;
        // an undeclared field would be silently dropped.
        http.response.status_code = tracing::field::Empty,
    );
    let _entered = span.enter();

    let started = Instant::now();
    let mut response = next.run(req).await;
    let elapsed = started.elapsed().as_secs_f64();

    let status = response.status();
    span.record("http.response.status_code", u64::from(status.as_u16()));

    http_metrics().duration.record(
        elapsed,
        &[
            KeyValue::new("http.request.method", method.to_string()),
            KeyValue::new("http.response.status_code", i64::from(status.as_u16())),
        ],
    );

    if let Some(value) = traceparent(&span) {
        response.headers_mut().insert(TRACEPARENT, value);
    }
    response
}

/// The W3C `traceparent` value for `span`, or `None` when the span carries
/// no valid `OTel` context (which is the case whenever export is disabled,
/// since no bridge layer is installed).
fn traceparent(span: &tracing::Span) -> Option<HeaderValue> {
    let context = span.context();
    let span_ref = context.span();
    let sc = span_ref.span_context();
    if !sc.is_valid() {
        return None;
    }
    let value = format!(
        "00-{}-{}-{:02x}",
        sc.trace_id(),
        sc.span_id(),
        sc.trace_flags().to_u8()
    );
    HeaderValue::from_str(&value).ok()
}

#[cfg(test)]
mod tests {
    use opentelemetry::trace::Tracer as _;

    use super::*;

    #[test]
    fn defaults_enable_export_at_the_documented_endpoint() {
        let config = TelemetryConfig::resolve(None, None);
        assert_eq!(config.service_name, DEFAULT_SERVICE_NAME);
        assert_eq!(config.endpoint.as_deref(), Some(DEFAULT_OTLP_ENDPOINT));
        assert!(config.export_enabled());
    }

    #[test]
    fn empty_endpoint_disables_export() {
        let config = TelemetryConfig::resolve(None, Some(""));
        assert!(!config.export_enabled());
        assert!(build_tracer_provider(&config).expect("build").is_none());
        assert!(build_meter_provider(&config).expect("build").is_none());
    }

    /// Loco's whitelist has no `opentelemetry*` entry, so a failed export
    /// is invisible until the filter is widened. Pinned with the concrete
    /// symptom: an SDK error event must pass the filter.
    #[test]
    fn exporter_diagnostics_are_added_to_a_default_filter() {
        // Loco's whitelist, standing in for what `init_env_filter` builds.
        let base = || tracing_subscriber::EnvFilter::new("person_service=info");

        assert!(
            !sdk_error_is_recorded(base()),
            "precondition: loco's whitelist really does swallow SDK errors"
        );
        assert!(
            sdk_error_is_recorded(with_exporter_diagnostics(base(), false)),
            "a failing export would still be silent"
        );
    }

    /// Emit an `opentelemetry_sdk` error under `filter` and report whether
    /// anything reached a layer — i.e. whether an operator would ever see a
    /// failing export.
    fn sdk_error_is_recorded(filter: tracing_subscriber::EnvFilter) -> bool {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use tracing_subscriber::Layer;

        #[derive(Clone)]
        struct Spy(Arc<AtomicBool>);
        impl<S: tracing::Subscriber> Layer<S> for Spy {
            fn on_event(
                &self,
                event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                if event.metadata().target().starts_with("opentelemetry") {
                    self.0.store(true, Ordering::SeqCst);
                }
            }
        }

        let seen = Arc::new(AtomicBool::new(false));
        let subscriber = tracing_subscriber::registry()
            .with(Spy(seen.clone()))
            .with(filter);
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!(target: "opentelemetry_sdk", "BatchSpanProcessor.ExportError");
        });
        seen.load(Ordering::SeqCst)
    }

    /// An operator who wrote their own filter keeps it verbatim; quietly
    /// re-enabling targets they silenced would be the mirror-image surprise.
    #[test]
    fn an_operator_supplied_filter_is_left_alone() {
        let base = tracing_subscriber::EnvFilter::new("person_service=info");
        let untouched = with_exporter_diagnostics(base, true);
        assert_eq!(untouched.to_string(), "person_service=info");
    }

    #[test]
    fn blank_service_name_falls_back_to_the_default() {
        let config = TelemetryConfig::resolve(Some("   "), None);
        assert_eq!(config.service_name, DEFAULT_SERVICE_NAME);
    }

    #[test]
    fn explicit_service_name_and_endpoint_are_honoured() {
        let config = TelemetryConfig::resolve(
            Some("person-service-canary"),
            Some("http://collector.internal:4317"),
        );
        assert_eq!(config.service_name, "person-service-canary");
        assert_eq!(
            config.endpoint.as_deref(),
            Some("http://collector.internal:4317")
        );
    }

    /// The load-bearing property for export-on-by-default: building the
    /// exporter must not dial the collector. Port 1 on localhost has
    /// nothing listening; if the builder connected eagerly this would fail
    /// or hang. (`tests/otlp_export.rs` proves the same end to end.)
    #[tokio::test]
    async fn building_an_exporter_does_not_require_a_live_collector() {
        let config = TelemetryConfig {
            service_name: "person-service".to_string(),
            endpoint: Some("http://127.0.0.1:1".to_string()),
        };
        let provider = build_tracer_provider(&config)
            .expect("exporter build must not fail on an unreachable endpoint")
            .expect("export is enabled");
        // Emitting through it must not block or panic either; the span is
        // queued and the export attempt fails in the background.
        provider
            .tracer("test")
            .in_span("unreachable-collector", |_| {});
        provider.shutdown().ok();
    }

    #[test]
    fn resource_carries_service_name_and_version() {
        let config = TelemetryConfig {
            service_name: "person-service".to_string(),
            endpoint: None,
        };
        let resource = config.resource();
        let name = resource
            .get(&opentelemetry::Key::from_static_str("service.name"))
            .expect("service.name");
        assert_eq!(name.as_str(), "person-service");
        let version = resource
            .get(&opentelemetry::Key::from_static_str("service.version"))
            .expect("service.version");
        assert_eq!(version.as_str(), env!("CARGO_PKG_VERSION"));
    }
}
