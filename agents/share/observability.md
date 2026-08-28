# Observability

Every Main X Index crate ships structured `tracing`, configurable log
levels, and a Prometheus `/metrics.prom` endpoint. **OpenTelemetry OTLP
export is not yet family-wide**: as of 2026-08-28, two services actually
export spans and metrics over OTLP —
[`link-graph-service`](../../link/link-graph-service-with-loco/)
(`src/observability.rs`, the original reference) and
[`person-service`](../../person/person-service-with-loco/)
(`src/observability.rs`, ported from it under repo `tasks.md` PRO-H9).
worker / event still carry the stub whose exporter is commented out; the
other crates have none. Copy person's port (not link-graph-service's
directly) for worker and event — its `AGENTS.md` "OpenTelemetry OTLP
export" section documents the two adaptations person's shape needed
(the tower middleware wired onto two router-construction surfaces, and
a renamed `tonic` dev-dependency to avoid colliding with the crate's own
gRPC-stub dependency) that worker and event will need too. See
[rust-tracing-opentelemetry-stack.md](rust-tracing-opentelemetry-stack.md)
for the full Loco-specific guide and
[overview.md](overview.md) for the honest per-crate picture; this file is
the short summary.
