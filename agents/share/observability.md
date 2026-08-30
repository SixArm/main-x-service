# Observability

Every Main X Index crate ships structured `tracing`, configurable log
levels, and a Prometheus `/metrics.prom` endpoint. **OpenTelemetry OTLP
export is not yet family-wide**: as of 2026-08-30, eight services
actually export spans and metrics over OTLP —
[`link-graph-service`](../../link/link-graph-service-with-loco/)
(`src/observability.rs`, the original reference),
[`person-service`](../../person/person-service-with-loco/),
[`worker-service`](../../worker/worker-service-with-loco/), and
[`event-service`](../../event/event-service-with-loco/) (all three
`src/observability.rs`, ported from link-graph-service under repo
`tasks.md` PRO-H9),
[`course-service`](../../course/course-service-with-loco/),
[`place-service`](../../place/place-service-with-loco/), and
[`thing-service`](../../thing/thing-service-with-loco/)
(`src/observability.rs`, ported from person's under `tasks.md`
PRO-H12 slices 1–3), and now
[`organization-service`](../../organization/organization-service-with-loco/)
(`src/observability.rs`, ported from course's under `tasks.md` PRO-H12
slice 4) — the **first of the four loco-idiomatic registries**
(`src/controllers/`, not person-style `src/api/rest/`) to carry it.
care-pathway, case, and portfolio still have no observability module —
PRO-H12's remaining scope. Copy organization's port (not a
person-style crate's) for those three: its `AGENTS.md` "OpenTelemetry
OTLP export" section documents the adaptation a genuinely loco-idiomatic
crate's shape needs — exactly **one** router-construction surface, so
the tower middleware is layered once rather than on two.

**The `otlp-test-tonic` dev-dependency rename is not decided by the
`overview.md` gRPC-stub capability row.** It is needed whenever the
crate **declares** a `tonic` Cargo dependency at all, whether or not any
code actually uses it — course needed no rename because it declares no
`tonic` dependency; place and thing both needed the rename despite
showing `–` on the gRPC-stub row, because each already declares
`tonic` + `tonic-build` in anticipation of a not-yet-built gRPC server
(place's spec T-4, thing's T-3) — thing's own `AGENTS.md` even already
said as much in its "Running this crate" section before this landed.
Check each remaining crate's actual `Cargo.toml` before assuming
either way, not the capability matrix.

place and thing also illustrate a second trap: both already carried
`opentelemetry`/`opentelemetry-otlp`/`opentelemetry_sdk`/
`tracing-opentelemetry` in `Cargo.toml` at stale 0.27/0.28 pins with
**zero consumers** (dead scaffolding from an earlier, since-deleted
stub) — a crate can show OTLP-flavoured dependencies in its manifest
with no working exporter behind them; verify by grepping `src/` for
actual usages before trusting the manifest, and bump stale pins to the
family's settled 0.32/0.33 versions in the same change that adds the
real module rather than leaving two OTLP dependency generations in one
manifest.

The four remaining loco-idiomatic crates (organization, care-pathway,
case, portfolio) have not yet had their router-construction surface
count and layering point worked out — expect it to differ from the
person-style crates' two-surface pattern, per PRO-H12's own note. See
[rust-tracing-opentelemetry-stack.md](rust-tracing-opentelemetry-stack.md)
for the full Loco-specific guide and
[overview.md](overview.md) for the honest per-crate picture; this file is
the short summary.
