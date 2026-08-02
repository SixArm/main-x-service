# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. This crate owns no domain tables (it's a pure
  aggregator over five upstream services; `migration/` is an empty
  migrator kept only so loco's boot works), so the PK-width and
  raw-Statement fallout that hit every entity-registry crate in this
  migration doesn't apply here — no source change beyond the two
  `Cargo.toml`s.
- No behavioural change; verified with the full DB-gated suite (50
  tests, unchanged count) — Postgres only needs to be reachable, no
  tables are asserted against.

### Fixed

- `src/auth/mod.rs` had rustfmt drift that broke the crate's
  `cargo fmt --check` gate. Reformatted with `cargo fmt`; no
  behavioural change, tests unchanged and green.

### Changed

- **Doc/test harmonization pass.** Added the missing `CLAUDE.md`
  (one-line `@AGENTS.md` include, matching every sibling subproject).
  Rewrote the `AGENTS.md` "Sibling projects" bullet to match the spec:
  the Svelte sibling is a **client** of this API and its Playwright
  suite runs against this service in stub mode (`USE_UPSTREAM_STUBS=1`)
  — dropped the stale "own back-end shim / future P1" wording.
  Renumbered the duplicate `### 8.` working rule to `### 9.`. Corrected
  the inaugural v0.1.0 "Added" Tests bullet (it still said "6 unit");
  it now reads 14 unit + the live request count. Added the missing
  `POST /api/auth/logout` request test
  (`logout_clears_the_session_cookie`: `204` + cleared `cts_session`
  cookie), bumping request tests 49 → 50. Added worked `examples.md`
  recipes for the volume-move flow (`POST /api/volumes/{id}/move` plus
  the assign/remove/rename companions) and for `GET /api/alerts`
  (geofence breaches). No behaviour change.


- **LT-16 / T-16 — Geofence-breach derivation made pure + unit-tested.**
  Extracted the cross-building alert logic behind `GET /api/alerts` from the
  controller into a pure `detect_geofence_breaches` function (plus a
  `cabinet_buildings` cabinet→room→building resolver). Added 6 in-crate tests
  covering every branch of the boundary-crossing rule (cross-building breach,
  same-building suppression, `None`-endpoint in-transit/created-in-place,
  unknown cabinet, orphan-room hierarchy, mixed-log filtering). Lib tests
  8→14; no behaviour change — the logic was previously reachable only through
  Postgres-gated request tests. Also corrected stale doc counts in
  `spec/testing.md` and `README.md` (unit 6→14, request 29→49).
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
  - **Tests**: 14 unit (8 NHS Modulus 11 + 6 geofence breach) + 50
    Postgres-backed request tests (`tests/requests/*`) exercising every
    controller against the in-process stubs. (The unit count started at
    6 in this inaugural cut and was raised to 14 by the LT-6 / LT-16
    entries above; request tests grew from 29 → 49 → 50.)

### Notes

- PostgreSQL is a **boot-time-only** dependency: Loco needs a connection
  at startup but the tracker creates no local tables (`migration/` is
  intentionally empty).
- **Demo software.** Not a regulated medical record — see
  [spec/regulatory.md](./spec/regulatory.md) for the production gates.
