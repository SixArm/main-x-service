# Observability

Every Main X Index crate ships structured `tracing`, configurable log
levels, and a Prometheus `/metrics.prom` endpoint. **OpenTelemetry OTLP
export is family-wide** as of 2026-09-02 (repo `tasks.md` PRO-H9 +
PRO-H12, both complete): eleven services actually export spans and
metrics over OTLP —
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
PRO-H12 slices 1–3),
[`organization-service`](../../organization/organization-service-with-loco/)
(`src/observability.rs`, ported from course's under `tasks.md` PRO-H12
slice 4) — the **first of the four loco-idiomatic registries**
(`src/controllers/`, not person-style `src/api/rest/`) to carry it —
[`care-pathway-service`](../../care-pathway/care-pathway-service-with-loco/)
(`src/observability.rs`, ported from organization's under `tasks.md`
PRO-H12 slice 5),
[`case-service`](../../case/case-service-with-loco/)
(`src/observability.rs`, ported from care-pathway's under `tasks.md`
PRO-H12 slice 6), and now
[`project-portfolio-management-service`](../../project-portfolio-management/project-portfolio-management-service-with-loco/)
(`src/observability.rs`, ported from case's under `tasks.md` PRO-H12
slice 7 — the last). Each of the four loco-idiomatic registries
independently **confirmed** rather than assumed organization's
single-router-surface shape held for it too (a fresh
`Router::new()`/`create_router` grep every time, never carried over
from an earlier finding) — all four land on the identical shape, four
for four. **Every entity registry in the family now exports real
OTLP/gRPC traces and metrics.** Copying any of the four loco-idiomatic
ports (case's or portfolio's, not a person-style crate's) is the
reference for a genuinely loco-idiomatic crate's shape — exactly
**one** router-construction surface, so the tower middleware is
layered once rather than on two — should a future entity registry
join the family.

**The `otlp-test-tonic` dev-dependency rename is not decided by the
`overview.md` gRPC-stub capability row.** It is needed whenever the
crate **declares** a `tonic` Cargo dependency at all, whether or not any
code actually uses it — course needed no rename because it declares no
`tonic` dependency; place and thing both needed the rename despite
showing `–` on the gRPC-stub row, because each already declares
`tonic` + `tonic-build` in anticipation of a not-yet-built gRPC server
(place's spec T-4, thing's T-3) — thing's own `AGENTS.md` even already
said as much in its "Running this crate" section before this landed.
None of the four loco-idiomatic registries (organization, care-pathway,
case, portfolio) declares a `tonic` dependency at all, so none needed
the rename. Check a crate's actual `Cargo.toml` before assuming either
way, not the capability matrix.

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

**IEC 62304 SOUP-register bookkeeping is per-crate, not per-shape.**
care-pathway and case each carry their own IEC 62304 SOUP register
(`compliance/soup.tsv`, verified live by
`every_direct_dependency_is_annotated`), so their ports needed new
rows for every new OTLP dependency (main and dev) before `cargo test
--lib` was green — 8 rows for care-pathway, 9 for case (case has no
`reqwest` main dependency the middleware test could reuse, so it
needed its own dev-only row). Organization and portfolio carry no
SOUP register at all, so their ports needed no such step. Check
whether a crate has `compliance/soup.tsv` before assuming either way.

All four loco-idiomatic crates (organization, care-pathway, case,
portfolio) now have a confirmed router-construction surface count and
layering point — one surface each, `trace_mw` layered once in
`after_routes`. See
[rust-tracing-opentelemetry-stack.md](rust-tracing-opentelemetry-stack.md)
for the full Loco-specific guide and
[overview.md](overview.md) for the honest per-crate picture; this file is
the short summary.
