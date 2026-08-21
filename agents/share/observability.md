# Observability

Every Main X Index crate ships structured `tracing`, configurable log
levels, and a Prometheus `/metrics.prom` endpoint. **OpenTelemetry OTLP
export is not yet family-wide**: as of 2026-08-05 exactly one service —
[`link-graph-service`](../../link/link-graph-service-with-loco/) — actually
exports spans and metrics over OTLP (`src/observability.rs`, and it is the
reference to copy). person / worker / event carry a stub whose exporter is
commented out; the other crates have none. See
[rust-tracing-opentelemetry-stack.md](rust-tracing-opentelemetry-stack.md)
for the full Loco-specific guide and
[overview.md](overview.md) for the honest per-crate picture; this file is
the short summary.
