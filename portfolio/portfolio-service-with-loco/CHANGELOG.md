# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md), [README.md](./README.md), [AGENTS.md](./AGENTS.md).

## [Unreleased]

### Added

- **Boot-time paseto-keys-over-HTTP fetch** (the spec §13 follow-up, done
  2026-07-04). New optional env var `PORTFOLIO_PASETO_KEYS_URL`: when set
  (non-blank), `auth::init` — called from `App::after_routes`, before the
  app serves traffic — fetches the auth-service's published Ed25519 key
  set once over HTTP via `Verifier::from_paseto_keys_url` (the
  `authentication-verifier` crate's `fetch` feature, now enabled). On
  success the fetched key set **wins** over the `PORTFOLIO_PASETO_KEYS`
  env key set (`tracing::info!`); on failure the service logs a
  `tracing::warn!` and falls back to the env path, so it **always
  boots**. Unset/blank ⇒ prior behaviour unchanged (env key set, else
  empty reject-all). Fetch is once-at-boot only — no refresh loop
  (rotation-triggered refetch is tracked in spec §16). The seeding is
  idempotent (`OnceLock`), and the fetch-or-fallback helper
  (`auth::fetch_or`) is dependency-injected (URL / issuer / audience /
  fallback passed in) so tests cover it without the process global: a
  `#[tokio::test]` local ephemeral-port HTTP listener proves a token
  signed by the served key verifies via the fetch-built verifier, and a
  fast-failing URL (`http://127.0.0.1:1/`) proves fallback without
  panic. Existing env-key auth tests unchanged and green.

## [0.1.0] - 2026-06-18

### Added

- **Inaugural spec scaffold (spec-only — no code yet).** Documentation
  set for the loco.rs work-item registry **and** project-management tool:
  - `spec/index.md` — the §1–§18 single-source-of-truth service spec,
    mirroring the care-pathway service shape. Defines the **four matchable
    collections** (`portfolios`, `projects`, `products`, `programs`) — one
    JSONB row table per kind, sharing one parameterised controller core
    (the API DTO **is** `portfolio_matcher::WorkItem`, persisted verbatim,
    matched with no adapter); **within-kind matching only** (the matcher's
    R-GATE makes a project never match a product); the umbrella hierarchy
    (Projects / Products / Programs carry a `portfolio_ref` to their parent
    portfolio); the operational sub-resources (goals, tasks, issues) in
    their own tables keyed by the parent `(kind, pid)` and **excluded from
    the matcher payload** (goal titles bridge via `data.goals[]`); the
    derived timeline / burndown read views; CRUD + soft-delete + audit;
    embedded probabilistic + deterministic matching (`POST /match` /
    `/check-duplicates` / `/deduplicate`); real-time create duplicate
    detection (`409`) + review queue; record merge (`Replaces` link +
    transferred snapshot + `Merged` event, same-kind only); `ILIKE` name
    search; event streaming (durable-bus Phase 1 envelope); OpenAPI/Swagger;
    per-collection Prometheus metrics; offline PASETO v4.public verification
    + blanket `/api/*` enforcement (off by default, gated by
    `PORTFOLIO_REQUIRE_AUTH`); cross-service entity links (write side); and
    bulk import/export (deferred).
  - `README.md` — user-facing intro, route table, quick start, status.
  - `CLAUDE.md` — one-line `@AGENTS.md` include.
  - `AGENTS.md` — agent guide (what this is, API surface, MVP scope,
    golden rules incl. four-kinds-one-core, within-kind matching, and the
    matcher-partition rule, intended layout).
  - `index.md` — documentation index + worked flow.
- **Auth model is PASETO v4.public + cookie sessions (spec-only).** The
  intended auth design is **server-side cookie sessions** for the human
  session plus **offline PASETO v4.public** verification for peers
  (verified against the auth-service's published **Ed25519 key**), and a
  **BFF** for the front-end so the browser holds no token. The
  `PORTFOLIO_REQUIRE_AUTH` flag + enforcement semantics follow the family
  contract. Source of truth:
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
  (RS256/JWKS not used).
- **Adopts the cross-service-linking contract.** Portfolio is a
  participating service with an `entity_links` write-side table and
  `POST`/`GET`/`DELETE /api/v1/{collection}/{pid}/links` emitting `linked`
  / `unlinked`; a work item / goal / task / issue can link to **any** index
  entity. Cross-service links are **not** a matcher signal (separate from
  within-payload `relationships`). Contract:
  [`agents/share/cross-service-linking.md`](../../agents/share/cross-service-linking.md).
- **Adopts the bulk-import/export contract** (deferred §13). Async
  `bg_pg` jobs, JSONL/CSV/Parquet, the five endpoints under
  `/api/v1/{collection}/*`; stable upsert key = a deterministic external PM
  identifier (Jira / Asana / Trello / MS Project / GitHub Project / Linear /
  URI / UUID) or owner-scoped `code` or `pid`; keyless rows → dedupe →
  review queue (within-collection). Lead / person refs are personal data →
  export audited. Contract:
  [`agents/share/bulk-import-export.md`](../../agents/share/bulk-import-export.md).

### Notes

- No Rust / Cargo crate has been generated; every `spec.md §13` task is
  unchecked. Next step is `loco new` (stripped of the auth starter) plus
  the four work-item tables + the shared CRUD MVP.
- The canonical `WorkItem` domain model is owned by the
  [portfolio entity spec §5](../spec/index.md); this crate spec references
  it.
- Copy-adapted from the (deleted) `plan` service template; the headline
  differences are the **four distinct matchable kinds** (vs plan's single
  `plan_type` field), the within-kind match **gate** (R-GATE), and the
  dropped `posts` / `comments` / `members` sub-resources (now deferred
  roadmap).

[Unreleased]: #unreleased
[0.1.0]: #010---2026-06-18
</content>
