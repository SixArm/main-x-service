# Content Management System service — edition spec

Stack-specific specification for the Loco edition. The
**cross-cutting spec at [`../../spec/`](../../spec/index.md) is the
single source of truth** for the domain (model, the six modules,
state machines, auth posture); this file adds only what is specific
to this edition. It will grow topic files (routes, api-contract,
database, examples) as implementation phases land — each CMS-T*
task adds its spec detail here in the same PR as the code.

## Stack

Per [architecture](../../spec/architecture.md) and the family
[rust-loco-stack](../../../agents/share/rust-loco-stack.md):
Rust 2024, Loco (Axum + SeaORM), PostgreSQL 18, crate-root
`migration/`, loco-idiomatic `src/controllers/` layout, pure
`src/rules/` core, `bg_pg` jobs (schedule sweep, sitemap build,
reference check, webhook delivery), the family `ArtifactStore` seam
for asset bytes, stub-first upstream clients, offline PASETO +
ABAC, OpenAPI/Swagger, `Accepts-version` header versioning, tracing
+ OTLP, Podman.

## Edition-specific decisions

Variables and seams below are declared for the whole edition; each
becomes live with the phase that implements it (the artifact store with
CMS-T8, preview tokens with CMS-T22, and so on).

- **Config**: `config/{development,test,production}.yaml`;
  development and test default upstream clients to `stub`,
  `CMS_EVENT_TRANSPORT=memory`, and the `local` artifact store.
- **Env vars**: `CMS_REQUIRE_AUTH`, `CMS_PASETO_KEYS[_URL]`,
  `CMS_ABAC_POLICY[_FILE]`, `CMS_EVENT_TRANSPORT`, upstream base
  URLs (`CMS_WORKER_SERVICE_URL`, `CMS_ORGANIZATION_SERVICE_URL`),
  `CMS_UPSTREAM_MODE` (default `stub`),
  `CMS_BULK_ARTIFACT_BACKEND` / `CMS_ARTIFACT_DIR` /
  `CMS_S3_*` (the care-pathway store's variable shape),
  `CMS_MAX_UPLOAD_BYTES` (default 25 MiB), `CMS_SITE_QUOTA_BYTES`
  (default 1 GiB), `CMS_PREVIEW_TOKEN_TTL_SECS` (default 900),
  `CMS_REDIRECT_MAX_HOPS` (default 5),
  `CMS_STALE_CONTENT_DAYS` (default 365),
  `CMS_REVIEW_SLA_DAYS` (default 7).
- **Identifiers**: public UUID `pid` on every owned record;
  EntityRef URNs for all upstream references; assets addressed by
  SHA-256 in the artifact store.
- **Table names all-plural** (the loco `create_table`
  pluralization lesson); `event_outbox` as explicit SQL.
- **Append-only tables** (`revisions`, `consent`-style history
  analogues, audit) carry **no `deleted_at`** — nothing soft-deletes
  history.
- **Public routes**: `/delivery/*` is the only non-family public
  prefix, and only for `public` sites, GET/HEAD, published
  revisions ([../../spec/auth.md](../../spec/auth.md)). The site's
  visibility is read **per request**, not cached at router
  construction (unlike `CMS_REQUIRE_AUTH`, which is boot-read per
  the family convention).
- **Enforcement tests** live in `tests/enforcement.rs` (own
  process, OnceLock lesson); request tests are `#[ignore]`d — run
  with `cargo test -- --ignored`.

## Edition-specific implementation notes (as landed, CMS-T1–T22)

- **Layout**: `src/{app,auth,clients,metrics,openapi,streaming,
  validation,version}.rs`, `src/rules/` (pure core: `locale`,
  `schema`, `template`, `tokens`), `src/models/` (+`_entities/`, 5
  tables), `src/controllers/{sites,types,audits,docs,metrics}.rs`,
  `src/tasks/seed.rs`, crate-root `migration/` (4, explicit SQL).
- **Uniqueness is partial-indexed over live rows**
  (`… WHERE deleted_at IS NULL`), so a soft-deleted key can be reused
  rather than being retired forever by a row nobody can see.
- **A site is `restricted` by default**, and flipping it writes a
  `visibility_changed` audit action rather than a generic `updated` —
  it is the one edit that changes who may read the site's published
  content without a credential.
- **Deletes refuse on live children**: a site with templates or
  content types, and a template a content type still names, return
  `409` naming what is in the way (CMS-D8's posture, applied to the
  declaration layer). Content types will gain the same refusal
  against entries with CMS-T5.
- **`schema_version` bumps only when the field set actually
  changed**, so the number means "the declaration moved", not "someone
  pressed save".
- **The compatibility classifier gates the write and backs the dry
  run** (`POST /api/content-types/{pid}/compatibility`) — the same
  function, so the preview cannot disagree with the gate.
- **`/delivery/*` is not yet public.** `auth::delivery_site_key` is
  the pure half (recognise the request, name its site); the
  visibility lookup that would make an anonymous read safe lands with
  the delivery controller (CMS-T17/T21). An enforcement test pins that
  delivery is currently guarded.
- **The `sites_public` gauge is recounted from the database** after
  every site mutation rather than incremented, so it cannot drift into
  reporting fewer exposed sites than exist.

### Phase 2 (authoring core)

- **`content_references`, not `references`** — the latter is a
  reserved SQL keyword that would need quoting at every use.
- **Revision numbers are allocated under `SELECT … FOR UPDATE` on the
  variant row**, with `UNIQUE (variant_pid, number)` as the backstop:
  the lock makes it correct, the index makes it provable.
- **The save path is sanitize → validate → extract → insert**, all in
  one transaction. Sanitizing first means what is validated is what is
  stored, and the response reports `blocks_sanitized` so a caller is
  never told their markup was stored verbatim when it was not.
- **Required fields are checked at publish, not at save**
  (`schema::missing_required` is ready for CMS-T12): a draft is
  allowed to be incomplete.
- **"Usage" counts only current revisions** — a reference from a
  superseded revision is history, not usage.
- **`ammonia` (html5ever) for the sanitizer**, configured with an
  explicit allow-list; the tests pin the policy, not the parser
  (`src/rules/sanitize.rs` documents why a hand-rolled stripper was
  refused).
- **The diff is positional and says so in its payload**, rather than
  implying a content-aligned precision it does not have.

### Phase 3 (assets)

- **`src/storage.rs` is the care-pathway `ArtifactStore`**, copied
  rather than re-invented: local backend by default
  (`CMS_ARTIFACT_DIR`, base-confined), S3-compatible behind the
  optional `s3` feature, asking for `s3` without the feature is an
  error rather than a silent fallback.
- **The storage key is the SHA-256 of the content**, sharded
  (`sha256/ab/cd/…`) so a local directory does not accumulate a million
  siblings. An uploader can never choose a key, and identical bytes
  deduplicate.
- **Uploads are typed from their bytes** (`rules::media`); a declared
  content type that disagrees is a refusal, not a correction. The
  allow-list is PNG/JPEG/GIF/WebP/MP4/WebM/MP3/WAV/PDF, and recognised
  dangerous formats are refused **by name with a reason**.
- **SVG is refused** (spec revised with this phase — see
  `../../spec/assets.md`).
- **Dimensions are read from headers, never decoded.** A malformed file
  costs a bounds check, not a decoder bug; unknown dimensions report
  `None` rather than a guess.
- **`produced` renditions require a `storage_ref`**, and `replace`
  resets them to `declared` — a rendition claiming bytes that no longer
  exist is a 404 in a delivery payload.
- **Deleting an asset does not delete its bytes** (other assets may
  share the content address, and a soft delete is reversible).
- **The publish gate lives in `rules::gate`** and is exposed both as a
  read (`publish-check`) and as the refusal inside the publish
  transition — one function, so the preview and the gate cannot
  disagree.

### Phase 4 (editorial workflow)

- **The transition table (`try_next`) is separate from the error
  message built on it.** Merging them caused infinite recursion, since
  the message lists the legal actions and computing that list asks the
  table for every action.
- **Reads inside a transaction go through that transaction.**
  `publish_blockers_for` and `content_type_of` are generic over
  `ConnectionTrait` for exactly this reason: holding a transaction and
  asking the pool for a second connection deadlocks (immediately
  against the single-connection test pool, latently in production).
- **A manual publish or unpublish clears any pending schedule**, so the
  sweep cannot later overrule the person who acted.
- **The sweep clears the due field inside the transaction that applies
  the transition** — that, plus the row lock, is what makes it
  idempotent rather than merely usually-correct.
- **Locks are advisory and say so in their own response body**; the
  authoritative check remains `base_revision_pid` on save, and a test
  pins that a lock does not block a save.
- **The unpublish marker and the routable-path gate landed with
  Phase 6**, when the routing table they need arrived.

### Phase 5 (localization)

- **`locale::resolve` is the one resolution function**, and the
  delivery composer (CMS-T17) will call it rather than reimplementing
  the walk. Every answer carries the requested locale, the served
  locale, whether a fallback was applied, the hops walked, and the
  refusal reason when nothing can be served.
- **`request` pins the source revision**, not `complete` — staleness
  is computable while the work is in progress.
- **`complete` does not write the translated text**; the translator
  saves through the ordinary revision endpoint, so there is one write
  path rather than two that can drift.
- **Staleness with no recorded source is `unknown`, not fresh.**
- **`unpublish_on_stale` is per content type and off by default**;
  even when on, the site view only *lists* what would be unpublished
  (`would_unpublish`) and reports `auto_unpublished: false`.

### Phase 6 (routing, delivery, SEO, personalization)

- **The guard defers delivery reads; the controller decides.** Whether
  a site is `public` is a database read, so `enforce` cannot answer it.
  `GET`/`HEAD` on `/delivery/*` is deferred and the controller checks
  `visibility` per request — a site flipped to `restricted` stops
  answering anonymously on the next request. Nothing else is deferred.
- **Chains collapse on creation**, so resolution is one lookup, and a
  rename repoints everything that pointed at the old address.
- **Paths are refused, never rewritten**: `..`, whitespace, percent
  escapes, and query strings are `422`s.
- **A `410` marker is a terminus in the resolver**, not a hop —
  unpublishing leaves one (or a `301` to a declared replacement), and
  republishing clears it.
- **The ETag excludes `as_of` and includes the consulted personalization
  context**, and the response declares `Vary`; a personalized page must
  not share a cache entry with an unpersonalized one.
- **References expand one hop, as summaries.**
- **The Atom feed is built** — `GET /delivery/{site}/{locale}/feed.xml`
  serves Atom 1.0, reusing the sitemap derivation, `noindex`-excluded,
  falling back cleanly to an empty feed. CMS-T18's feed residual closed
  2026-07-31 (CMS-R19; `../../spec/tasks.md` CMS-T18).
  <!-- PRO-H8, 2026-08-28: this bullet previously read "No feed yet" —
  stale; the feed landed 2026-07-31 per spec/tasks.md CMS-T18. -->

### Phase 7 (insights)

- **The honesty rules live in the types**: `Ratio` always carries its
  numerator and denominator and is `null` on a zero denominator;
  `DurationSummary` refuses percentiles below a sample floor and
  returns the raw observations with a note instead.
- **Findings ship their rule's explanation**, so a dashboard shows the
  sentence the code applied.
- **No severity score** is invented — findings group by rule, and the
  count is the count.
- **Time-in-state comes from transition audit rows**, not `updated_at`.
- **`weak_etag` / `matches_etag`** are shared by insights and delivery,
  so conditional reads behave identically in both.

### Preview tokens (CMS-T22)

- **The stored column is a hash**; the token exists in one response and
  nowhere else — not in a log, not in an audit row (a test asserts it).
- **Scope is (variant, revision)**, so a share cannot follow the
  content forward.
- **Refusals are byte-identical** across unknown / expired / revoked /
  wrong-revision; the reason is audited, the caller learns nothing.
- **The guard defers preview paths for a different reason than public
  delivery**: delivery is authorized by site visibility, preview by the
  token. `auth::is_preview_path` names the distinction.

## Delivery

The queue is [../../spec/tasks.md](../../spec/tasks.md): **CMS-T1–T24,
this edition's full scope, are all delivered** — T1–T22 landed
2026-07-30 (T18's feed residual closed 2026-07-31), CMS-T23 (webhooks)
landed 2026-07-30, and CMS-T24 (the full seed corpus: a synthetic
site, ~3 content types, ~40 entries per `tasks.md`, realized as 29
entries / 50 variants / 90 revisions) completed 2026-07-30. Nothing
remains open for this edition (CMS-T25/T26, Phase 9, are the sibling
front-end edition's tasks — see
[../../content-management-system-front-end-with-svelte/spec/index.md](../../content-management-system-front-end-with-svelte/spec/index.md)).
Tests per [../../spec/testing.md](../../spec/testing.md)
— 231 DB-free unit tests, 60 request tests, plus 3 webhook-delivery
integration tests and 1 enforcement binary, all green against
Postgres 18 (counts verified 2026-08-28 via `cargo test --lib -- --list`
and `cargo test --test '*' -- --list`).
<!-- PRO-H8, 2026-08-28: this section previously said CMS-T23/T24
"remain" and cited stale 215/53 test counts; both tasks were already
complete per spec/tasks.md (CMS-T23 2026-07-30, CMS-T24 2026-07-30) and
the current counts are higher. Corrected during the professionalization
sweep. -->
