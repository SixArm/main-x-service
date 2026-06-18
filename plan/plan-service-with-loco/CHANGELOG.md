# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Changed

- **Authentication model (spec-only; no code yet).** The intended auth
  design is **server-side cookie sessions** for the human session plus
  **offline PASETO v4.public** verification for peers (verified against
  the auth-service's published **Ed25519 key**), and a **BFF** for the
  front-end so the browser holds no token. This replaces any RS256 JWT +
  JWKS framing in the inaugural scaffold below. The `PLAN_REQUIRE_AUTH`
  flag + enforcement semantics are unchanged; only the credential
  changes. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (RS256/JWKS not used). Human-facing docs (README / AGENTS / index)
  updated to match. No code exists to change.

## [0.1.0] - 2026-06-17

### Added

- **Inaugural spec scaffold (spec-only — no code yet).** Documentation
  set for the loco.rs plan registry **and** project-management tool:
  - `spec/index.md` — the §1–§18 single-source-of-truth service spec,
    mirroring the care-pathway service shape. Defines the `plans` JSONB
    row (the API DTO **is** `plan_matcher::Plan`, persisted verbatim,
    matched with no adapter), the operational sub-resources (goals,
    tasks, issues, posts, comments, members) in their own tables keyed by
    the plan `pid` and **excluded from the matcher payload**, the derived
    timeline / burndown read views, CRUD + soft-delete + audit, embedded
    probabilistic + deterministic matching (`POST /match` /
    `/check-duplicates` / `/deduplicate`), real-time create duplicate
    detection (`409`) + review queue, record merge (`Replaces` link +
    transferred snapshot + `Merged` event), `ILIKE` name search, event
    streaming (durable-bus Phase 1 envelope), OpenAPI/Swagger, Prometheus
    metrics, offline RS256 JWT verification + blanket `/api/*` enforcement
    (off by default, gated by `PLAN_REQUIRE_AUTH`), cross-service entity
    links (write side), and bulk import/export (deferred).
  - `README.md` — user-facing intro, route table, quick start, status.
  - `CLAUDE.md` — one-line `@AGENTS.md` include.
  - `AGENTS.md` — agent guide (what this is, API surface, MVP scope,
    golden rules incl. the matcher-partition rule, intended layout).
  - `index.md` — documentation index + worked flow.
- **Adopts the cross-service-linking contract.** Plan is a participating
  service with an `entity_links` write-side table and
  `POST`/`GET`/`DELETE /api/v1/plans/{pid}/links` emitting `linked` /
  `unlinked`; a plan / goal / task / issue can link to **any** index
  entity. Cross-service links are **not** a matcher signal (separate from
  within-payload `relationships`). Contract:
  [`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md).
- **Adopts the bulk-import/export contract** (deferred §13). Async
  `bg_pg` jobs, JSONL/CSV/Parquet, the five endpoints under
  `/api/v1/plans/*`; stable upsert key = a deterministic external PM
  identifier (Jira / Asana / Trello / MS Project / GitHub Project / Linear /
  URI / UUID) or owner-scoped `plan_code` or `pid`; keyless rows →
  dedupe → review queue. Member / person refs are personal data → export
  audited. Contract:
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md).

### Notes

- No Rust / Cargo crate has been generated; every `spec.md §13` task is
  unchecked. Next step is `loco new` (stripped of the auth starter) plus
  the `plans` table + CRUD MVP.
- The canonical `Plan` domain model is owned by the
  [plan entity spec §5](../spec/index.md); this crate spec references it.

[0.1.0]: #010---2026-06-17
