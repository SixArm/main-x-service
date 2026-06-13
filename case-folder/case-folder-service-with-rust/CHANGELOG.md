# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Changed

- **LT-6 / T-15 — Modulus-11 hardening.** `src/nhs.rs` now has 8 lib tests
  (was 6). Added explicit coverage for the `check == 10 → invalid` branch
  (`999 000 0140`), the documented invalid number `614 309 0431`,
  grouped/bare-form parity, and the empty-input case. No behaviour change;
  the validator was already correct — this closes a coverage gap on a
  documented core invariant (`spec/nhs-number.md`).

### Added

- **Inaugural scaffold (v0.1.0).** Loco / Axum / SeaORM **JSON API** for
  Case Tracking — a pure-aggregator back-end that tracks the physical
  location of NHS paper case-note folders. Owns no domain tables; every
  entity lives in one of five external Main-X-Services.
  - **Upstream service clients** (`src/main_{patient,place,worker,thing,event}_service/`):
    each a `Client` trait + projection types + an `http` implementation +
    an in-process `stub`. Swappable at boot.
  - **Stub mode** (`USE_UPSTREAM_STUBS=1`, `src/initializers/bootstrap_stubs.rs`):
    swaps every client for an in-process stub, auto-seeded with demo
    data, so the API boots fully functional with no external services —
    used by the Svelte sibling's Playwright suite and local UI iteration.
  - **Controllers**: `folders` (list/create/show/history), `moves`
    (list/create/show), `volumes` (list/create/show/rename + folder
    assign/remove + move-whole-volume), `patients` (list/show), `places`
    (list/create/show + cabinet presence history), `workers` (list/show),
    `stats`, `alerts` (cross-building/geofence), `healthz`.
  - **Magic-link auth** (`src/auth/`, `src/controllers/auth.rs`):
    stateless signed tokens, allowlist identity, session-cookie guard,
    dev mailer that logs the link. `GET /api/auth/me` always requires a
    session; domain routes honour the configurable `require_session`.
  - **NHS Number** Modulus 11 validator + formatter (`src/nhs.rs`).
  - **Snapshotting**: patient/cabinet/worker labels are snapshotted into
    every move event at record time so the audit trail survives upstream
    renames and outages.
  - **Seed task** (`cargo run -- task seed`): registers 3 buildings /
    4 rooms / 5 cabinets, 6 patients, 9 folders, and 9 initial moves
    across all five services; idempotent; soft-falls-back to placeholder
    UUIDs when an upstream is unreachable.
  - **Hand-written OpenAPI 3** document (`openapi.yaml`) covering all 29
    operations; kept accurate to the mounted routes.
  - **Tests**: 6 unit (NHS Modulus 11) + 49 Postgres-backed request tests
    (`tests/requests/*`) exercising every controller against the
    in-process stubs.

### Notes

- PostgreSQL is a **boot-time-only** dependency: Loco needs a connection
  at startup but the tracker creates no local tables (`migration/` is
  intentionally empty).
- **Demo software.** Not a regulated medical record — see
  [spec/regulatory.md](./spec/regulatory.md) for the production gates.
