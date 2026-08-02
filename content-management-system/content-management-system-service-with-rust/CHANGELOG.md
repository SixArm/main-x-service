# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — loco-rs 1.0.1 (2026-08-02)

- **loco-rs 0.16 → 1.0.1**: sea-orm 1.1 → 2.0, sea-orm-migration →
  2.0, sea-query → 1.0. No feature-list changes (default feature set).
- **`ColType::PkAuto` now generates a 64-bit primary key.** Of this
  crate's 18 tables, exactly one (`audit_logs`) goes through loco's
  schema DSL and moves from `i32` to `i64`; content_types, sites,
  entries, assets, routing, preview, webhooks, and `event_outbox` are
  all raw SQL and stay `i32` — the same split as every other consumer
  app in this migration.
- Two `useless_conversion`s (`models/event_outbox.rs`,
  `controllers/entries.rs`'s soft-delete-cascade-to-variants update)
  and five pre-existing `needless_borrows_for_generic_args`
  (`controllers/assets.rs` ×2, `controllers/entries.rs`,
  `controllers/mod.rs` ×2) — the largest single-crate clippy cleanup
  in this migration, all the same class of finding surfaced by the
  same `cargo clippy` run, not new issues introduced by the bump.
- No behavioural change; verified with the full DB-gated suite (64
  tests, unchanged count) against a freshly migrated Postgres 18. This
  is the last of the seventeen crates in the family-wide loco-rs 1.0.1
  migration.

### Added

- 2026-07-31 — CMS-T18's last residual: an **Atom 1.0 feed** at
  `GET /delivery/{site}/{locale}/feed.xml`. Per-locale, newest first
  by *publication* time, capped at 50, behind the same site-visibility
  check as every other delivery read, and published-only by
  construction. `noindex` pages are excluded — a feed is a syndication
  surface, and a page the site asked crawlers to ignore has not asked
  to be syndicated. The entry id is the entry's `pid`, so a rename
  does not resurface a page as a new item; summaries are declared
  `type="text"` and escaped, because a feed is exactly where inventing
  markup would escape the block model into somebody else's reader; and
  an empty feed falls back to the site's creation time rather than
  "now", which would make an unchanged feed look fresh on every poll.
  New: `seo::render_feed`, `seo::FeedEntry`, `seo::FEED_LIMIT`.
  Verified live: the seeded site's feed parses with a real XML parser,
  18 entries newest-first, no drafts, ids all `urn:uuid:`.

- 2026-07-31 — CMS-T21 (the half that had not landed): the
  **record-level ABAC layer is now enforced**, where before it existed
  only as a library. Five handlers take the second pass — the revision
  read, the revision history, the entry read, `save_revision`, and
  `create_variant` — and the `mask` obligation redacts unpublished
  bodies, editorial notes, and reviewer identities while leaving the
  structure visible. New: `auth::mask_if_required` /
  `mask_if_required_checked` (which **refuse** an obligation this
  build cannot honour rather than serving the record unmasked),
  `auth::proposed_variant_attrs` (a locale-scoped persona has to be
  gated on the variant being *asked for*), and
  `controllers::authz_error`. The enforcement binary now exercises all
  five personas from `spec/auth.md`.


- 2026-07-30 — CMS-T24: the **full synthetic corpus** behind
  `task seed` — 29 entries and 50 variants across `en`/`fr`/`fr-CA`,
  revisions in every workflow state, 25 assets (3 deliberately
  unreferenced), a menu, a four-hop redirect chain, two stale
  translations, an open translation request, a future scheduled
  publish, and the backdated `variant` audit rows the throughput view
  derives from. **One instance of every content-health rule is planted
  on purpose**, each on an entry whose key names the rule, and a
  request test asserts all ten fire — so a rule that stops working
  fails the suite instead of quietly emptying the demo. The corpus
  stays mostly healthy (15 findings against 39 published variants),
  which the same test pins. New: `tasks::seed_corpus`,
  `tests/requests/seed.rs`.

- 2026-07-30 — CMS-T23: **outbound webhooks**, this service's only
  extension mechanism. Per-site subscriptions to event kinds,
  delivered with an HMAC-SHA256 signature over `{timestamp}.{body}`
  — the timestamp is *inside* the signed material, so a captured
  delivery cannot be replayed later. HTTPS only (loopback excepted,
  the family's standing rule for server-side fetches), no redirects
  followed, a 5-second timeout, a capped response read, five attempts
  on a 0/30/120/480/1920s backoff, and a delivery log. A 4xx other
  than 408/429 is not retried; a subscription failing repeatedly is
  deactivated. Retries are *scheduled*, not slept: a failed attempt
  is recorded and picked up by the next dispatch, so nothing is held
  in memory. The signing secret is returned by exactly one response
  and by no read or audit row. Dispatch reads the durable event
  outbox and **refuses with a `422` naming the setting** under the
  default in-memory transport, rather than delivering a subset that
  vanishes on restart. `hmac` 0.12 rather than the family
  `integrity-mac` crate (which protects stored rows with a key the
  service never shares — a different problem) and rather than a
  hand-rolled construction. New: `rules::webhook`,
  `controllers::webhooks`, `task webhook_dispatch`, migration
  `m20260730_000010_webhooks`. 222 DB-free unit tests + 55 request
  tests + 1 enforcement binary + 3 delivery tests (against a real
  loopback receiver that verifies the signature the way a third party
  would) green against Postgres 18.

- 2026-07-30 — CMS-T22 (with the CMS-T21 record-level attribute work):
  **preview tokens** — the one route by which unpublished content
  leaves the service. Scoped to a single (variant, revision), 15
  minutes by default and a day at most, revocable immediately, stored
  only as a SHA-256 hash, and audited on issue, use, and refusal. The
  render is `no-store` + `noindex`, and every refusal (unknown,
  expired, revoked, wrong revision) returns the same 404 so the
  endpoint cannot be used to probe for valid tokens. Also:
  `auth::variant_resource_attrs` (owner / reviewer / status / locale /
  content type / published) and `MASKED_CONTENT_KEYS`, which make the
  five personas expressible as ABAC policy rather than code. New:
  `rules::preview`, `controllers::preview`, migration
  `m20260730_000009_preview`. 215 DB-free unit tests + 53 request
  tests + 1 enforcement binary green against Postgres 18.

- 2026-07-30 — CMS-T20 implementation round (Phase 7, content
  insights): the health view (ten rules — missing alt text, missing
  SEO, broken references, orphan assets, stale content, stale
  translations, stuck in review, approved-not-published,
  needs-migration, route hazards — each finding naming its rule and
  each group shipping the sentence the code applied), the throughput
  view (activity by transition, rates that show numerator and
  denominator and go `null` on a zero denominator, time-in-state
  measured from transition audit rows with percentiles suppressed
  below a sample floor, per-actor counts), and the backlog view
  bucketed by age. All derived on read, ETag-conditional,
  `as_of`-stamped. New: `rules::insight`, `controllers::insights`, and
  a shared `weak_etag` / `matches_etag` pair now used by delivery too.
  207 DB-free unit tests + 49 request tests + 1 enforcement binary
  green against Postgres 18.

- 2026-07-30 — CMS-T16–T19 implementation round (Phase 6, routing,
  delivery, SEO, personalization): addresses normalized to one form
  with a partial-unique current path per site and locale; renaming a
  page leaves a `301` automatically and collapses chains so resolution
  stays one lookup; redirect loops refused at write time; the
  **public delivery surface** — published revisions only, composed with
  locale-honesty fields, one-hop reference summaries, existing
  renditions, the template's region contract and canonical, behind a
  weak ETag (excluding `as_of`) with `304` support; menus that omit
  unpublished targets; `sitemap.xml` derived from published, indexable,
  routable variants with `lastmod` and reciprocal `hreflang`;
  `robots.txt` reflecting the site's visibility; and audience rules
  over an **allow-listed request context** whose evaluation reports
  both matches and the keys consulted, so the ETag and `Vary` cover
  exactly what personalized the response. New: `rules::path`,
  `rules::audience`, `rules::seo`, `controllers::routing`,
  `controllers::delivery`, migration `m20260730_000008_routing`. 198
  DB-free unit tests + 45 request tests + 1 enforcement binary green
  against Postgres 18.

### Changed

- 2026-07-30 — **The public delivery allow-list is open.** The blanket
  guard now defers `GET`/`HEAD` on `/delivery/*` to the delivery
  controller, which checks the site's `visibility` on every request
  (so a site flipped to `restricted` stops answering anonymously
  immediately). Mutating verbs on the same prefix are still refused by
  the guard, and nothing outside the prefix is deferred; the
  enforcement binary pins each case.
- 2026-07-30 — **Unpublishing leaves a `301` or a `410`** rather than a
  bare 404, and republishing clears the marker. **Publishing a routable
  entry now requires an address** (`route_missing`). Both were CMS-T12
  residuals waiting on the routing table.

- 2026-07-30 — CMS-T14/T15 implementation round (Phase 5,
  localization): fallback resolution that always reports the locale it
  actually served, whether a fallback was applied, and the hops walked;
  `strict_locales` that refuse fallback rather than answering in
  another language; the translation workflow (request → claim →
  complete → cancel) whose `request` pins the source revision being
  translated; staleness derived from source-revision drift, reported
  with the count *and* the revision numbers; the per-entry locale
  matrix with its missing-locale list; the site translation queue and
  stale list; and locale coverage with the gap list rather than a bare
  percentage. New: `rules::locale::resolve`, `rules::staleness`,
  `lifecycle::translation`, `controllers::localization`, migration
  `m20260730_000007_translation`, and the per-content-type
  `unpublish_on_stale` opt-in (default off). 169 DB-free unit tests +
  35 request tests + 1 enforcement binary green against Postgres 18.

- 2026-07-30 — CMS-T11–T13 implementation round (Phase 4, editorial
  workflow): the lifecycle machine (draft → in_review → approved →
  published → archived, with reasons required on reject / unpublish /
  archive / restore, and every refusal naming the current state and the
  legal actions); publishing that names a **specific** revision, so a
  save after publishing changes nothing live and `first_published_at`
  survives unpublish and republish; the publish gate wired to the same
  function the `publish-check` read uses; `require_distinct_approver`;
  scheduling with an idempotent sweep that clears the due field in the
  same transaction, skips and records anything a person has overtaken,
  and runs the same gate as a manual publish; advisory locks with
  expiry and reasoned stealing. New: `rules::lifecycle`,
  `controllers::workflow`, the `schedule_sweep` CLI task, the
  `schedules` and `published` site reads, and three metrics. 151
  DB-free unit tests + 30 request tests + 1 enforcement binary green
  against Postgres 18.

### Fixed

- 2026-07-31 — **a policy granting a masked read returned the full
  unpublished body.** `authorize_record` computed the obligation and
  no handler applied it; `variant_resource_attrs`,
  `content_type_resource_attrs`, `mask_json`, and
  `MASKED_CONTENT_KEYS` were defined, unit-tested, and called by
  nothing. Verified live: an unattributed caller now receives
  `blocks`, `fields`, `note`, and `reviewer_ref` as `null` on a record
  an admin reads in full.
- 2026-07-31 — a locale-scoped or owner-scoped persona could not be
  enforced at all: the blanket guard decides before any record is
  loaded, so a rule keyed on `resource.*` never matched on a write.
  Writes now take the record-level pass.
- 2026-07-31 — `create_variant` validated the locale before
  authorizing, so a caller who may not write a locale could learn from
  the error whether the site declared it. Authorization now runs
  first.


- 2026-07-30 — **A newly-required field was invisible to the
  `needs_migration` health rule.** It consulted only
  `validate_values`, which inspects fields that are *present*, so
  content the publish gate would refuse — a field that became required
  after the content was written — looked healthy. It now also consults
  `missing_required`, the same check the gate uses. Found by the
  seeded corpus, pinned by a unit test.
- 2026-07-30 — **Infinite recursion in the transition table.** The
  error message from an illegal transition listed the legal actions,
  and computing that list asked the same function again — an illegal
  transition overflowed the stack. The table (`try_next`) is now
  separate from the message built on top of it.
- 2026-07-30 — **Pool-exhaustion deadlock in the scheduled sweep.** It
  held a transaction on the locked variant row and then asked the pool
  for a second connection to read the entry, revision, and gate inputs.
  `publish_blockers_for` and `content_type_of` are now generic over
  `ConnectionTrait`, so every read inside a transaction goes through
  that transaction. Latent under a busy production pool; immediate
  against the single-connection test pool, which is how it surfaced.

- 2026-07-30 — CMS-T8–T10 implementation round (Phase 3, the asset
  library): uploads on the family `ArtifactStore` seam (local default,
  optional `s3` feature) with SHA-256 content addressing and dedupe,
  per-upload and per-site caps, magic-byte typing that refuses a
  declaration disagreeing with the bytes, and an accepted-format
  allow-list; metadata + tags + filters; declared renditions
  (`declared → produced | failed`, where `produced` requires bytes);
  replace that keeps the asset's identity, its references, and its
  kind, and resets produced renditions; orphan reporting that deletes
  nothing; delete-refusal with the reasoned override; safe delivery
  (`nosniff`, kind-appropriate disposition, documents download).
  New pure cores: `rules::media` (sniffing, allow-list, header-read
  dimensions) and `rules::gate` (publish blockers with remedies),
  exposed as `GET …/publish-check` — the alt-text accessibility gate is
  live as a read and CMS-T12's publish transition will call the same
  function. 139 DB-free unit tests + 20 request tests + 1 enforcement
  binary green against Postgres 18; `cargo deny` clean.

### Changed

- 2026-07-30 — **SVG uploads are refused** rather than sanitized, and
  `../spec/assets.md` was revised to say so. An HTML5 sanitizer is not
  an SVG sanitizer; running SVG through one would look like protection
  while leaving `<script>`, `on*`, `<foreignObject>`, and external
  entities intact. The refusal explains itself and points at a raster
  export; proper SVG support is a roadmap item.

- 2026-07-30 — CMS-T5–T7 implementation round (Phase 2, authoring
  core): entries with one variant per locale and an append-only
  revision chain (numbers allocated under the variant row lock, backed
  by `UNIQUE (variant_pid, number)`); optimistic concurrency — a save
  states its `base_revision_pid` and a stale one is `409` naming the
  competing revision; positional revision diff; restore that writes a
  **new** revision recording what it copied; structured block
  documents with per-kind rules, inline marks that must fit their
  text, and refusal of unknown kinds *and* unknown keys by path;
  write-time HTML sanitization of the one place markup may appear (an
  `embed` block), via an `ammonia` allow-list with a hostile corpus;
  reference extraction into `content_references` in the mutation's
  transaction, driving `usage` for entries and assets and the
  delete-refusal, with a reasoned `?force=true&reason=…` override that
  records every reference it broke. New: `ammonia` dependency, 5
  pure-core modules, `entries` controller, 10 OpenAPI paths,
  `revision_created_total` + `blocks_sanitized_total` metrics. 115
  DB-free unit tests + 13 request tests + 1 enforcement binary green
  against Postgres 18.

- 2026-07-30 — CMS-T1–T4 implementation round (Phase 1): the loco
  scaffold with the family fixtures, 4 migrations (sites, templates,
  content types + audit/outbox side tables), the pure `rules/` core
  (locale + fallback-chain validity, content-type field schemas and
  the `additive | tightening | breaking` compatibility classifier,
  template region contracts), the `sites` and `types` controllers with
  transactional audit + outbox emission and delete-refusal on live
  children, the stub-first upstream client seam, and the phase-1 seed.
  83 DB-free unit tests + 6 request tests + 1 enforcement binary green
  against Postgres 18; clippy-pedantic and `cargo fmt --check` clean.
  `/delivery/*` is deliberately **not** yet public — the visibility
  check that makes an anonymous read safe lands with the delivery
  controller (CMS-T17/T21).

- 2026-07-30 — CMS-T0 specification round: the cross-cutting spec
  (`../spec/`) and this edition's doc scaffold. No code yet; this
  edition is CMS-T1–T24 in the queue.
