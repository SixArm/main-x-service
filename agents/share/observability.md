# Observability

Every Main X Index crate ships structured `tracing`, configurable log
levels, and a Prometheus `/metrics.prom` endpoint. **OpenTelemetry OTLP
export is not yet family-wide**: as of 2026-08-30, five services
actually export spans and metrics over OTLP —
[`link-graph-service`](../../link/link-graph-service-with-loco/)
(`src/observability.rs`, the original reference),
[`person-service`](../../person/person-service-with-loco/),
[`worker-service`](../../worker/worker-service-with-loco/), and
[`event-service`](../../event/event-service-with-loco/) (all three
`src/observability.rs`, ported from link-graph-service under repo
`tasks.md` PRO-H9), and
[`course-service`](../../course/course-service-with-loco/)
(`src/observability.rs`, ported from person's under `tasks.md` PRO-H12).
place, thing, organization, care-pathway, case, and portfolio have no
observability module yet — PRO-H12's remaining scope. Copy person's port
(not link-graph-service's directly) for the rest — its `AGENTS.md`
"OpenTelemetry OTLP export" section documents the adaptations a
person-style crate's shape needs (the tower middleware wired onto two
router-construction surfaces, and — only for a crate that also carries a
`tonic` gRPC stub, per [overview.md](overview.md)'s capability matrix —
a renamed `tonic` dev-dependency to avoid an extern-prelude collision;
course needed no rename, since it carries no gRPC stub of its own). The
four remaining loco-idiomatic crates (organization, care-pathway, case,
portfolio) have not yet had their router-construction surface count and
layering point worked out — expect it to differ from the person-style
crates' two-surface pattern, per PRO-H12's own note. See
[rust-tracing-opentelemetry-stack.md](rust-tracing-opentelemetry-stack.md)
for the full Loco-specific guide and
[overview.md](overview.md) for the honest per-crate picture; this file is
the short summary.
