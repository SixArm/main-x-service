# Observability

Every Main X Index crate ships structured `tracing`, configurable log
levels, and a Prometheus `/metrics.prom` endpoint.

**OpenTelemetry OTLP export comes from axa** (`axa::opentelemetry`, the
`opentelemetry` feature) in the ten services that export: person, worker,
event, course, place, thing, organization, care-pathway, case, and
link-graph. Each sends spans and the `http.server.request.duration` metric
over OTLP/gRPC, and answers every request with a W3C `traceparent` header.
Until 2026-11-07 each carried its own `src/observability.rs`, ported in
turn from link-graph-service's (repo `tasks.md` PRO-H9 and PRO-H12); those
copies are gone, and a fix to the exporter is now one fix in axa.

Export is **off unless `OTLP_ENDPOINT` is set**, and plain `http` to a host
other than the service's own stops it at boot unless `OTLP_ALLOW_INSECURE`
is set, because spans carry what the logs do. Both differ from the old
modules, which exported to `http://localhost:4317` by default and allowed
any `http` endpoint, so a deployment that relied on either must now set
them.

How a service wires it, what it emits, and the configuration are in
[rust-tracing-opentelemetry-stack.md](rust-tracing-opentelemetry-stack.md);
the per-crate picture is in [overview.md](overview.md).
