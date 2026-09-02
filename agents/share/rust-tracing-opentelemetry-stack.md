### Rust tracing opentelemetry stack

- Structured logging with `tracing` crate
- Telemetry with `opentelemetry` crate
- Configurable log levels (RUST_LOG)
- Request/response logging
- Error logging with context

## Status (2026-09-02 — family-wide, PRO-H12 complete)

This document describes the shape every entity registry now
implements — no longer a target, a completed rollout as of 2026-09-02.
[`link-graph-service`](../../link/link-graph-service-with-loco/) implemented
it first (`src/observability.rs` — the family's first working exporter,
proved against a real in-process OTLP/gRPC collector in its
`tests/otlp_export.rs` / `tests/otlp_middleware.rs`).
[`person-service`](../../person/person-service-with-loco/) is a close port
of it (repo `tasks.md` PRO-H9, 2026-08-28), with the same in-process-collector
test tier ported alongside; **worker and event** followed in the same
task (also 2026-08-28), each copying person's already-adapted port rather
than re-deriving link-graph-service's. **course, place, and thing** are
the next ports (`tasks.md` PRO-H12 slices 1–3, 2026-08-30), all copying
person's port. All six of those crates' `AGENTS.md` document the
adaptations each needed beyond link-graph-service's shape: the tower
middleware wired onto **two** router-construction surfaces instead of
one, for every crate that (like person/worker/event/course/place/thing)
carries a hand-rolled `create_router` alongside the loco-native path;
and — for a crate that **declares** a `tonic` Cargo dependency at all —
a renamed dev-dependency, `otlp-test-tonic = { package = "tonic", … }`,
so the in-process collector's tonic 0.14 does not collide with it in a
test binary's extern prelude. **This is not the same test as
[overview.md](overview.md)'s gRPC-stub capability row**, which tracks
only whether a `src/api/grpc` module exists: person/worker/event need
the rename because they have both a stub and the dependency; place and
thing need the rename too despite showing `–` on that row, because each
already declares `tonic` + `tonic-build` in `Cargo.toml` in
anticipation of a not-yet-built gRPC server (place's spec T-4, thing's
T-3) — Cargo does not care that no code calls `tonic::` yet, only that
the name and version collide. course needed no rename because it is
one of the crates here that declares no `tonic` dependency at all.
**Check each remaining crate's actual `Cargo.toml`, not the capability
matrix**, before assuming either way.

**organization** is the newest port (`tasks.md` PRO-H12 slice 4 of 7,
2026-08-30, copying course's port) and the **first of the four
loco-idiomatic registries** (organization, care-pathway, case,
portfolio — `src/controllers/`, not person-style `src/api/rest/`) to
carry one. It settles, for itself, the open question the other six
ports left for this group: organization has exactly **one**
router-construction surface (`App::routes`/`App::after_routes`; even
its own request-level test suite boots the real `App` via loco's
testing harness rather than a second hand-rolled router), so
`trace_mw` is layered once, not twice — and it declares no `tonic`
dependency at all, so it needed no rename either, same as course.
**care-pathway** is the second loco-idiomatic port (`tasks.md` PRO-H12
slice 5 of 7, 2026-08-30, copying organization's port) and confirms
rather than assumes organization's single-router-surface shape: a grep
of `src/` and `tests/` for a second `Router::new()`/`create_router`
turned up one hit, a unit test for the auth middleware itself, not an
app-level router, so `trace_mw` is layered once in `after_routes`, same
as organization. It also declares no `tonic` dependency of its own, so
it needed no rename either. Landing this raised `cargo test --lib` from
308 to 316 (8 new `src/observability.rs` unit tests). This crate is
also the family's IEC 62304 SOUP-register reference
(`compliance/soup.tsv`), so the port additionally needed eight new SOUP
rows (five main dependencies, three test-only) — a bookkeeping step the
person-style and organization ports had no equivalent of.
**case** is the third loco-idiomatic port (`tasks.md` PRO-H12 slice 6
of 7, 2026-09-02, copying care-pathway's port) and confirms rather
than assumes the same single-router-surface shape a third time: a
fresh grep of `src/` and `tests/` for a second
`Router::new()`/`create_router` turned up one hit, again a unit test
for the auth middleware itself, not an app-level router, so `trace_mw`
is layered once in `after_routes`. It also declares no `tonic`
dependency of its own, so it needed no rename either. Landing this
raised `cargo test --lib` from 253 to 261 (8 new
`src/observability.rs` unit tests). Case also carries its own IEC
62304 SOUP register, needing 9 new rows this time (one more than
care-pathway's 8) — case has no existing `reqwest` main dependency the
middleware test could reuse, so `reqwest` itself needed a dev-only SOUP
row care-pathway's port did not.
**portfolio** is the fourth and last loco-idiomatic port (`tasks.md`
PRO-H12 slice 7 of 7, 2026-09-02, copying case's port) and confirms
rather than assumes the same single-router-surface shape a fourth
time: a fresh grep of `src/` and `tests/` for a second
`Router::new()`/`create_router` turned up one hit, again a unit test
for the auth middleware itself, not an app-level router, so `trace_mw`
is layered once in `after_routes` — four for four loco-idiomatic
registries now share the identical single-surface shape. It also
declares no `tonic` dependency of its own, so it needed no rename
either. Unlike care-pathway and case, portfolio carries **no** IEC
62304 SOUP register, so this port needed no `compliance/soup.tsv`
bookkeeping step at all — the simplest of the four loco-idiomatic
ports for exactly that reason. Landing this raised `cargo test --lib`
from 353 to 361 (8 new `src/observability.rs` unit tests). **This
closes PRO-H12**: every entity registry in the family — all ten,
plus link-graph-service — now exports real OTLP/gRPC traces and
metrics. Three things settled that this doc does not say:

- **Versions.** `opentelemetry` / `_sdk` / `-otlp` /
  `-semantic-conventions` **0.32**, `tracing-opentelemetry` **0.33**, with
  the `grpc-tonic` feature (the crate defaults to HTTP + blocking reqwest,
  which does not match the `:4317` endpoint default below). person's
  0.27/0.28 pins are stale — 0.27's `install_batch(runtime::Tokio)`
  pipeline API no longer exists upstream.
- **Where it hooks in.** loco 1.0's `Hooks::init_logger` returning
  `Ok(true)`, composing loco's public `logger::init_env_filter` +
  `logger::init_layer` so the deployment's configured filter policy and
  log format are reused rather than re-derived. `Hooks::on_shutdown`
  flushes.
- **`RUST_LOG` gains a second job.** With the bridge layer installed the
  filter governs what is **exported**, not only what is logged. Loco's
  module whitelist is what keeps the trace stream to the service's own
  spans — and, until widened, also swallows the SDK's own export-failure
  errors, so a broken exporter looks exactly like a working one.

## Stack

| Concern                 | Crate                                                      | Notes                                      |
| ----------------------- | ---------------------------------------------------------- | ------------------------------------------ |
| Structured logging      | `tracing`, `tracing-subscriber`                            | JSON in production, compact in development |
| Metrics + traces export | `opentelemetry`, `opentelemetry-otlp`, `opentelemetry_sdk` | OTLP gRPC or HTTP                          |
| Bridge                  | `tracing-opentelemetry`                                    | Forwards `tracing` spans to OTel           |
| Semantic conventions    | `opentelemetry-semantic-conventions`                       | HTTP, DB, RPC attributes                   |

## What gets emitted

- **Spans** for every HTTP request, every DB query, every match scoring run, every search query
- **Events** at `info` / `warn` / `error` for create/update/delete operations
- **Audit log entries** to the `audit_log` table (separate from the trace stream)
- **Metrics** for request latency histograms, match scoring duration, search hit counts

## Configuration

Environment variables (`.env` or process env):

| Variable            | Default                 | Purpose                                   |
| ------------------- | ----------------------- | ----------------------------------------- |
| `RUST_LOG`          | `info`                  | `tracing-subscriber::EnvFilter` directive |
| `OTLP_SERVICE_NAME` | crate name              | Service.name attribute                    |
| `OTLP_ENDPOINT`     | `http://localhost:4317` | OTLP collector endpoint                   |

In `config/*.yaml`, the `logger:` block controls format (`compact` / `json`), level, and pretty backtraces.

## Where to look first

- Health: `GET /api/health`
- Audit history: `GET /api/audit/recent`, `GET /api/<plural>/{id}/audit`
- Per-request tracing: every response carries a `traceparent` header when an OTLP endpoint is configured
