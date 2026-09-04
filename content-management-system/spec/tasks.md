# Tasks — delivery checklist

Status legend: `[x]` done · `[~]` in progress · `[ ]` not started.
Every task traces to design (CMS-D*) and requirement (CMS-R*) ids.
Three-part rule applies: a behavioural change lands as spec edit +
code + tests in one PR.

## Phase 0 — specification

- [x] CMS-T0 Cross-cutting spec round: topic files + SDD trio, both
  edition doc scaffolds, root AGENTS.md wiring. (all CMS-D*,
  CMS-R*) — landed 2026-07-30. No code.

## Phase 1 — service skeleton, sites & content types (CMS-R1, CMS-R2, CMS-R24)

- [x] CMS-T1 Scaffold `content-management-system-service-with-rust`:
  loco app, config, migration crate, family fixtures (forbid-unsafe,
  tracing/OTLP, `/metrics.prom`, OpenAPI + Swagger, `Accepts-version`
  middleware, health routes). (CMS-D17)
- [x] CMS-T2 Sites + templates: CRUD, locales + fallback chains +
  `strict_locales`, `visibility`, base URL, robots defaults; audit +
  event seam (`CMS_EVENT_TRANSPORT=memory`). (CMS-D1, CMS-D6;
  CMS-R1, CMS-R23)
- [x] CMS-T3 Content types: field schema model, per-field validation,
  `schema_version` bump, pure-core compatibility classification
  (`additive | tightening | breaking`) with the confirmation flag +
  reason. (CMS-D2, CMS-D4; CMS-R2)
- [x] CMS-T4 Upstream client seam: worker / organization traits +
  `http` + `stub`, config-selected; display-name cache; stub-mode
  boot test. (CMS-D16)

> Phase 1 landed 2026-07-30 (copy-adapted from the CRM service for the
> family fixtures). 4 migrations (`sites`, `templates`,
> `content_types` + the `audit_logs` / `event_outbox` side tables,
> explicit SQL, partial unique indexes over live rows); pure `rules/`
> core (`locale` — code shape, declared-set membership, fallback chains
> refused unless every hop is declared, acyclic, and ending at the
> default; `schema` — field-kind validation, reserved keys, per-kind
> rules incl. `entity_ref` types checked against the `entity-ref`
> registry, plus the **compatibility classifier**; `template` — region
> contracts; `tokens`); two controllers (`sites`, `types`) with
> transactional audit + outbox emission, delete-refusal on live
> children, and the visibility change audited as its own action;
> `auth.rs` carrying `site_resource_attrs` /
> `content_type_resource_attrs` and the pure `delivery_site_key` half
> of the public-delivery allow-list. **83 DB-free unit tests, 6 request
> tests, 1 enforcement binary — all green against Postgres 18**;
> clippy-pedantic + `cargo fmt --check` clean; live smoke verified
> (migrate → seed → OpenAPI 11 paths → compatibility dry-run →
> unconfirmed breaking edit refused → visibility flip audited →
> `sites_public` gauge tracks reality).
>
> Two deliberate deviations from the queue, both fail-closed:
> **(1)** `/delivery/*` is **not** on the public allow-list yet. The
> visibility check that makes an anonymous read safe needs a database
> lookup, so it belongs with the delivery controller (CMS-T17/T21);
> until then delivery is guarded like every other path, pinned by an
> enforcement test. **(2)** A new site defaults to
> `visibility = restricted`, so nothing becomes world-readable without
> a deliberate edit.
>
> Also landed early: `[~]` **CMS-T24** — the seed task exists with its
> phase-1 slice (one site with a fallback chain, 2 templates, 3 content
> types across most field kinds). The full synthetic corpus needs the
> tables that CMS-T5 onward add.

## Phase 2 — authoring core (CMS-R3–R5)

- [x] CMS-T5 Entries + variants + append-only revisions: monotonic
  numbering under the variant lock, `base_revision_pid` optimistic
  concurrency (`409`), revision diff, restore-as-new-revision.
  (CMS-D3, CMS-D15; CMS-R3)
- [x] CMS-T6 Block documents: allow-listed block kinds + structured
  marks, nest/length/cardinality caps, path-naming `422`s, and the
  write-time HTML sanitizer with its hostile corpus. (CMS-D5;
  CMS-R4)
- [x] CMS-T7 Reference extraction + "where used" + delete refusal
  (`409` with referrers) + reasoned force-delete. (CMS-D8;
  CMS-R5)

> Phase 2 landed 2026-07-30. Migration `m20260730_000005_entries`
> (`entries`, `entry_variants`, append-only `revisions` with
> `UNIQUE (variant_pid, number)`, and `content_references` — named
> around the reserved SQL keyword). Pure core gains `block`
> (per-kind payload rules, structured inline marks whose ranges must
> fit their text, caps, depth bound, and refusal of unknown kinds
> **and unknown keys** by path), `sanitize` (an `ammonia`/html5ever
> allow-list, hostile-corpus tested), `reference` (syntactic
> extraction from blocks and typed field values), `diff` (positional,
> and it says so in the payload), plus `schema::validate_values` and
> `schema::missing_required`. Controller `entries.rs` carries the
> save path (sanitize → validate → extract → insert revision +
> references, all in the mutation's transaction, revision number
> allocated under `SELECT … FOR UPDATE`), diff, restore-as-new-
> revision, per-locale variants, `usage` for entries and assets, and
> the delete-refusal with its reasoned `?force=true&reason=…` override
> (which records every reference it broke, and does **not** override
> the published-variant check). **115 DB-free unit tests, 13 request
> tests, 1 enforcement binary — all green against Postgres 18**;
> clippy-pedantic + `cargo fmt --check` clean; live smoke verified
> (hostile embed reduced to `<p>ok</p>` before storage, stale save
> `409` naming the competing revision, gapless history, metrics).
>
> Three decisions worth not re-litigating:
> **(1) Required fields are not enforced on save**, only at publish
> (CMS-R11) — a draft is allowed to be incomplete, and refusing a
> save because a field is empty makes the editor fight the system all
> the way to publish. `schema::missing_required` is ready for the
> publish gate.
> **(2) "Usage" counts only *current* revisions.** A reference from a
> superseded revision is history, not usage; counting it would make
> every asset permanently undeletable.
> **(3) An `ammonia` dependency rather than a hand-rolled stripper** —
> the same reasoning the family applied to S3 signing: unverified
> security-relevant parsing code that looks finished is worse than a
> dependency (`src/rules/sanitize.rs` documents it).

## Phase 3 — assets (CMS-R6–R8)

- [x] CMS-T8 `ArtifactStore` wiring (local default, `s3` feature),
  SHA-256 content addressing + dedupe, byte/quota caps, MIME sniff
  match, media-type allow-list, ~~SVG sanitization~~ **SVG refusal**
  (see below), `nosniff` delivery. (CMS-D9; CMS-R6)
- [x] CMS-T9 Asset metadata + tags + listing filters + declared
  renditions (state machine `declared → produced | failed`,
  production seam documented). (CMS-D9; CMS-R7)
- [x] CMS-T10 Asset replace (references preserved), orphan
  reporting, and the alt-text publish gate. (CMS-D9, CMS-D13;
  CMS-R8, CMS-R11)

> Phase 3 landed 2026-07-30. Migration `m20260730_000006_assets`
> (`assets` content-addressed by SHA-256, `renditions`);
> `src/storage.rs` is the family `ArtifactStore` copied from
> care-pathway (local default + optional `s3` feature, base-directory
> confinement); pure `rules/media` (magic-byte sniffing, the accepted
> allow-list, header-read dimensions for PNG/JPEG/GIF, never panicking
> on hostile bytes) and `rules/gate` (publish blockers, each with a
> remedy); `controllers/assets` (upload → metadata → renditions →
> replace → orphans → quota) plus `GET …/publish-check` on the entry
> controller. **139 DB-free unit tests, 20 request tests, 1
> enforcement binary — all green against Postgres 18**;
> clippy-pedantic + `cargo fmt --check` + `cargo deny` clean.
>
> **One spec revision, made deliberately: SVG is refused, not
> sanitized.** [assets.md](assets.md) originally allowed SVG "when
> sanitized against an allow-list", which assumed a sanitizer this
> project does not have — an HTML5 sanitizer is not an SVG sanitizer,
> and running SVG through one would look like protection while leaving
> `<script>`, `on*`, `<foreignObject>`, and external entities intact.
> The spec now records the refusal, the reasoning, and the roadmap
> item; the refusal message names the reason and points at a raster
> export.
>
> Other decisions worth not re-litigating:
> **(1) Renditions are declarations.** `produced` requires a
> `storage_ref`, and a replace resets produced renditions to
> `declared` — a rendition that claims bytes which no longer exist is
> a 404 in a delivery payload. Producing pixels (a decoder on
> attacker-supplied bytes) is its own hardening round.
> **(2) Replace must keep the asset's kind.** Swapping an image for a
> PDF would break every layout showing it.
> **(3) Deleting an asset does not delete its bytes.** Other assets
> may share the same content address, and a soft delete is meant to be
> reversible; reclaiming storage is a separate, deliberate sweep.
> **(4) Orphans are reported, never auto-deleted** — "unreferenced
> today" and "safe to destroy" are different claims.

## Phase 4 — editorial workflow (CMS-R9–R12)

- [x] CMS-T11 Editorial lifecycle machine: transitions, reason
  requirements, `require_distinct_approver`, reviewer assignment.
  (CMS-D4; CMS-R9)
- [x] CMS-T12 Publish/unpublish: publish names a revision,
  `first_published_at` preservation, unpublish leaves a redirect or
  `410`, publish gates (required fields, alt text, valid unique path,
  live reference targets). (CMS-D3, CMS-D7; CMS-R10, CMS-R11)
  — **residuals closed with Phase 6**, which brought the routing table
  they needed.
- [x] CMS-T13 Scheduling + locks: sweep idempotent per (variant,
  scheduled_at) with skip-and-record, advisory locks with expiry +
  reasoned steal, publish race serialization. (CMS-D14, CMS-D15;
  CMS-R12)

> Phase 4 landed 2026-07-30. Pure `rules/lifecycle` (the transition
> table, its reason requirements, and the legal-actions listing that
> every refusal carries); `controllers/workflow` (transitions,
> publish/unpublish, scheduling, locks, the sweep, plus the
> `schedules` and `published` reads); `tasks/schedule_sweep` as the
> CLI surface a system scheduler drives. **151 DB-free unit tests, 30
> request tests, 1 enforcement binary — all green against Postgres
> 18**; clippy-pedantic + `cargo fmt --check` + `cargo deny` clean;
> live smoke verified (out-of-order approve refused by name,
> submit→approve→publish, a save after publishing leaving the live
> pointer alone, unpublish preserving `first_published_at`, and the
> reason requirement).
>
> **Two residuals, both deferred to CMS-T16 rather than faked.**
> Unpublish should leave a redirect to a declared replacement or a
> `410 Gone` marker, and the publish gate should refuse a routable
> type with no valid unique path. Both need the routing table that
> does not exist yet; unpublish records the declared replacement in
> its audit row now, and `publish-check` states the missing check in
> its own payload rather than implying completeness.
>
> **Two bugs found by the tests, worth recording:**
> **(1) Infinite recursion in the transition table.** `next()`'s error
> message listed the legal actions, and `legal_actions()` asked
> `next()` for every action — so an illegal transition recursed until
> the stack ran out. Fixed by splitting the table (`try_next`) from
> the message built on top of it; the test that caught it is the one
> asserting the refusal explains itself.
> **(2) A pool-exhaustion deadlock in the sweep.** It held a
> transaction on the locked variant row and then asked the pool for a
> second connection to read the entry, the revision, and the gate
> inputs. With the single-connection test pool that timed out
> immediately; with a production pool it is a latent deadlock under
> load. Fixed by making `publish_blockers_for` and `content_type_of`
> generic over `ConnectionTrait` so every read inside a transaction
> goes through **that** transaction.
>
> Decisions worth not re-litigating:
> **(1) Publishing names a revision**, so a save after publishing
> changes nothing live until the next publish, and
> `first_published_at` survives unpublish and republish.
> **(2) Direct-publish from `draft` is the same transition**, gated by
> policy — not a second code path that would drift from the gates.
> **(3) A manual publish/unpublish clears any pending schedule**, so
> the clock cannot later overrule the person who acted; a schedule
> whose variant has moved is skipped, recorded, and cleared rather
> than retried forever.
> **(4) The scheduled path runs the same gate as the manual one** — a
> clock is not a reason to publish a page with no alt text.
> **(5) Locks are advisory and say so in their own response.** The
> authoritative protection against lost work is the
> `base_revision_pid` check on save; a test pins that a lock does not
> block a save, so the lock cannot quietly be mistaken for a mutex.

## Phase 5 — localization (CMS-R13–R15)

- [x] CMS-T14 Locale variants + fallback chain resolution +
  `strict_locales` + locale-honesty fields
  (`locale_requested`/`served`/`fallback_applied`). (CMS-D1;
  CMS-R13, CMS-R14)
- [x] CMS-T15 Translation workflow + derived staleness (revisions
  behind, which ones) + the opt-in `unpublish_on_stale`. (CMS-D4,
  CMS-D13; CMS-R15)

> Phase 5 landed 2026-07-30. Migration
> `m20260730_000007_translation` (per-variant translation status,
> requester, translator, due date; `content_types.unpublish_on_stale`
> defaulting **false**). Pure core gains `locale::resolve` (walks a
> chain against what is actually published and reports
> `locale_requested` / `locale_served` / `fallback_applied` / the hops
> walked / the refusal) and `rules::staleness` (how far behind, and
> which revision numbers); `lifecycle::translation` adds the
> request → claim → complete workflow beside the editorial one.
> `controllers::localization` serves resolution, the per-entry locale
> matrix, the translation actions, the site queue + stale list, and
> locale coverage with the **gap list**. **169 DB-free unit tests, 35
> request tests, 1 enforcement binary — all green against Postgres
> 18**; clippy-pedantic + `cargo fmt --check` + `cargo deny` clean;
> live smoke verified (fr-CA → fr → en walked and reported, the walk
> shortening to `["fr"]` once French publishes, and a translation
> going stale by 1 revision naming revision 2).
>
> Decisions worth not re-litigating:
> **(1) `request` pins the source revision**, not `complete`. Staleness
> is then computable for work that is still in progress, and a
> translator can see what they are translating.
> **(2) The translated text arrives through the ordinary save
> endpoint.** `complete` only flips the status, so there is one write
> path for revisions rather than two that can drift.
> **(3) Missing provenance is reported as `unknown`, not "fresh".**
> Claiming freshness for content whose source was never recorded is
> the more dangerous of the two guesses.
> **(4) Stale content is never auto-unpublished.** A content type may
> opt in (`unpublish_on_stale`), and even then the site view lists the
> affected entries under `would_unpublish` so the decision is visible
> before anything acts on it — the endpoint reports
> `auto_unpublished: false` in as many words.
> **(5) Translation status is orthogonal to editorial status** — a
> translated variant still goes through review and publish, so
> "translated" cannot quietly come to mean "approved".

## Phase 6 — delivery, routing & SEO (CMS-R16–R20)

- [x] CMS-T16 Routes + redirects: path normalization, unique current
  path, auto-`301` on slug change, write-time loop refusal + chain
  collapse, bounded resolution. (CMS-D10; CMS-R17)
- [x] CMS-T17 Delivery API: pure-core composition (variant +
  fallback + one-hop reference summaries + existing renditions +
  template regions), ETag/`as_of`, published-only, menus.
  (CMS-D6, CMS-D7; CMS-R16, CMS-R18)
- [x] CMS-T18 SEO artifacts: per-revision SEO block, `sitemap.xml`
  with `lastmod` + `hreflang`, canonical resolution, `robots.txt`,
  and the Atom feed (the residual, closed 2026-07-31). (CMS-D13;
  CMS-R19)
- [x] CMS-T19 Audience rules: allow-listed context predicate,
  pure evaluation with matched-rule reporting, ETag/`Vary`
  variation. (CMS-D11; CMS-R20)

> Phase 6 landed 2026-07-30. Migration `m20260730_000008_routing`
> (`routes` with a partial unique index over the current path,
> `redirects` where a null target means `410`, `menus`,
> `audience_rules`). Pure core gains `path` (one normal form; bounded,
> loop-free resolution; write-time cycle refusal; chain collapse),
> `audience` (the allow-listed request-context predicate, reporting
> both matches and the keys consulted), and `seo` (canonical, XML-safe
> sitemap rendering, `robots.txt`). Controllers `routing` and
> `delivery`. **198 DB-free unit tests, 45 request tests, 1
> enforcement binary — all green against Postgres 18**;
> clippy-pedantic + `cargo fmt --check` + `cargo deny` clean; live
> smoke verified (the route gate refusing a publish, anonymous
> delivery of a public site with a weak ETag and a 304, a rename
> answering 301, unpublish answering 410, republish answering 200,
> and the sitemap/robots pair).
>
> **The public allow-list is now open — and only this far.** The
> blanket guard defers `GET`/`HEAD` on `/delivery/*` to the delivery
> controller *because the decision needs a database read*: the
> controller checks the site's `visibility` on **every** request, so
> flipping a site to `restricted` takes effect on the next request
> rather than the next restart. The enforcement binary pins all of it:
> a restricted site refuses an anonymous read, a public one answers,
> a mutating verb on the same prefix is still refused by the guard,
> and nothing outside the prefix is deferred.
>
> **Both CMS-T12 residuals closed**: unpublish now leaves a `301` to a
> declared replacement or a `410` marker (republishing clears it), and
> the publish gate refuses a routable type with no address
> (`route_missing`).
>
> **The residual is closed (2026-07-31): the feed is built.**
> `GET /delivery/{site}/{locale}/feed.xml` serves Atom 1.0 —
> per-locale, newest first, capped at 50, behind the same
> site-visibility check as every other delivery read, and published-
> only by construction (it reads `published_revision_pid`, so a draft
> cannot reach it). Pure `seo::render_feed` + `seo::FeedEntry`.
>
> Decisions worth not re-litigating:
> **(1) Atom, not RSS.** Atom *requires* a stable `id` and an
> unambiguous `updated`; both are things this service can supply
> honestly, where RSS's `guid`/`pubDate` are conventions.
> **(2) The entry id is the entry's `pid`, not its URL**, so renaming
> a page does not resurface it as a new item in every reader.
> **(3) Summaries are declared `type="text"` and escaped.** The
> service stores blocks, not HTML, and a feed is exactly where
> inventing markup would escape the block model into somebody else's
> reader.
> **(4) `noindex` pages are excluded.** A feed is a syndication
> surface; a page the site asked crawlers to ignore has not asked to
> be syndicated either.
> **(5) The timestamp is the *publication* time**, not the revision's
> write time: a page drafted in March and published in July belongs at
> July in a feed of what is new. An empty feed falls back to the
> site's creation time rather than "now", which would make an
> unchanged feed look fresh on every poll.
>
> One test corrected along the way: a first draft asserted that a
> restricted site refuses its feed, which is **false with
> `CMS_REQUIRE_AUTH` off** — the shipped default leaves every delivery
> read open, feed included. That property belongs to an *activated*
> deployment, so it is pinned in the enforcement binary instead, and
> the request test now states what is actually true.
>
> Decisions worth not re-litigating:
> **(1) Redirect chains collapse on creation**, so resolution stays
> one lookup however many times a page is renamed, and a rename
> repoints everything that aimed at the old address.
> **(2) Loops are refused at write time**, not discovered at request
> time — a request-time hang is the worst way to learn about one.
> **(3) Paths are refused, never silently rewritten.** `/a/../b`,
> whitespace, percent escapes, and query strings are `422`s: a page at
> an address the editor did not choose is worse than an error.
> **(4) Personalization varies its own cache key.** The evaluation
> reports every context key any rule consulted, the ETag mixes exactly
> those in, and the response declares `Vary` — a personalized page
> cached under a URL-only key is a data-leak mechanism.
> **(5) Menus omit unpublished targets** rather than linking into a
> 404, and delivery expands references **one hop** as summaries — a
> `DoS` boundary as much as a design one.

## Phase 7 — insights (CMS-R21)

- [x] CMS-T20 Content health + editorial throughput: every finding
  with its rule key; time-in-state from transition events; honesty
  rules (numerator/denominator, `null` denominators, percentile
  sample floor); ETag-conditional, `as_of`. (CMS-D13; CMS-R21)

> Phase 7 landed 2026-07-30. Pure `rules::insight` (ratios that carry
> their numerator and denominator and go `null` on a zero denominator;
> duration summaries that refuse to be percentiles below a sample floor
> and return the raw observations instead; the health-rule table with
> its explanations) and `controllers::insights` (health, throughput,
> backlog — all ETag-conditional and `as_of`-stamped). The ten health
> rules are live: missing alt text, missing SEO, broken references,
> orphan assets, stale content, stale translations, stuck in review,
> approved-not-published, `needs_migration`, and route hazards.
> **207 DB-free unit tests, 49 request tests, 1 enforcement binary —
> all green against Postgres 18**; clippy-pedantic + `cargo fmt
> --check` + `cargo deny` clean; live smoke verified (a finding with
> its explanation, a 304 on the conditional read, and a `null`
> approval rate showing `0 / 0` rather than a flattering percentage).
>
> Decisions worth not re-litigating:
> **(1) The response ships the rule explanations**, so a dashboard
> shows the same sentence the code applied rather than inventing its
> own wording.
> **(2) No severity score is invented.** Findings group by rule and
> the count is the count; ranking them would be a judgement the data
> does not support.
> **(3) Time-in-state is measured from transition audit rows**, never
> from `updated_at` — a column that moves for unrelated reasons and
> would quietly turn "time in review" into "time since anything
> happened".
> **(4) Percentiles below the floor are suppressed**, not smoothed: a
> p90 over three observations is not a p90, so the raw durations are
> returned with a note saying why.
> **(5) There is no reader analytics here, and a test asserts the
> payload never mentions visits, visitors, page views, or sessions** —
> the service records none and holds no visitor identity to attach
> them to.

## Phase 8 — auth, audit & extension surface (CMS-R22–R24)

- [x] CMS-T21 `auth.rs`: offline PASETO verify + blanket
  `CMS_REQUIRE_AUTH` guard (guard-all / deny-unless-public) + the
  **narrow public-delivery allow-list** (GET/HEAD, `public` sites,
  published only) + ABAC record-level attrs
  (`resource.owner`/`site`/`status`/`locale`/`content_type`) +
  `$sub` ownership + `mask` obligation over unpublished bodies,
  notes, and reviewer identity; the five-persona matrix in its own
  enforcement binary. (CMS-D7, CMS-D17; CMS-R22)

> CMS-T21 completed 2026-07-31. The guard, the verifier, the
> allow-list, and the attribute builders landed alongside CMS-T22 on
> 2026-07-30; **this closes the half that had not**.
>
> The gap, found by checking the box's claims against the code before
> ticking it: `authorize_record`, `variant_resource_attrs`,
> `content_type_resource_attrs`, `mask_json`, and
> `MASKED_CONTENT_KEYS` were all defined, unit-tested, and **called by
> nothing**. The record-level layer existed as a library and protected
> no request — a policy granting a *masked* read returned the full
> unpublished body. The earlier note claiming that work "landed" was
> true of the library and false of the enforcement.
>
> What closed it:
> **(1) Three reads now take the record-level pass** —
> `GET /api/revisions/{pid}` (an unpublished body, the most sensitive
> read in the service), the revision history (which carries editorial
> `note`s), and `GET /api/entries/{pid}` (whose variants carry
> `reviewer_ref`). A refused variant is **omitted rather than
> reported**, since naming it would leak the shape of what the caller
> may not see.
> **(2) Two writes take it too** — `save_revision` and
> `create_variant` — which is what makes a locale-scoped or
> owner-scoped persona enforceable at all: the blanket guard decides
> before any locale or owner is known. `auth::proposed_variant_attrs`
> describes the variant being *asked for*, since it does not exist
> yet.
> **(3) `mask_if_required` pairs the obligation with the keys**, so a
> handler cannot compute an obligation and then not honour it — which
> is exactly what had happened. An obligation this build does not
> implement is **refused, not ignored**: silently serving the full
> record because the policy asked for something unrecognised would
> turn a stricter policy into a weaker one.
> **(4) Authorization runs before validation** in `create_variant`, so
> a caller who may not write a locale does not learn from the error
> whether the site declares it.
> **(5) The enforcement binary now carries all five personas**
> (author / editor / translator / admin / delivery), each expressed
> purely as policy.
>
> One policy-language finding worth keeping: a `when` value list means
> **any of**, so `["!fr", "!fr-CA"]` reads as "not-fr OR not-fr-CA",
> which is true of every locale — negation cannot express "none of
> these". The translator persona is therefore two positive rules, with
> the deny keyed on `resource.status` (a key every record has and no
> coarse request does) so it fires only on the record-level pass.
>
> Verified live with enforcement **on** against the seeded database:
> an `access=admin` caller reads an unpublished body in full; a caller
> with no attributes gets the same record with `blocks`, `fields`,
> `note`, and `reviewer_ref` `null` and the structure — number, title,
> status — intact. 226 unit + 58 request + 1 enforcement + 3 delivery
> tests green.
- [x] CMS-T22 Preview tokens: short-lived, one-(variant, revision)
  scope, revocable, `no-store`, audited on issue and use, excluded
  from sitemaps. (CMS-D7; CMS-R22)

> CMS-T22 landed 2026-07-30, together with the record-level attribute
> work CMS-T21 needed (`auth::variant_resource_attrs` — owner,
> reviewer, status, locale, content type, published — plus
> `MASKED_CONTENT_KEYS`, which is what makes the five personas
> expressible as policy rather than code). Migration
> `m20260730_000009_preview` stores **hashes, never tokens**; pure
> `rules::preview` mints 256-bit tokens, hashes them, clamps
> lifetimes, and decides refusals; `controllers::preview` issues,
> lists, revokes, and renders. **215 DB-free unit tests, 53 request
> tests, 1 enforcement binary — all green against Postgres 18**;
> clippy-pedantic + `cargo fmt --check` + `cargo deny` clean; live
> smoke verified (a 64-hex token, a `no-store` + `noindex` render, the
> stored column proven *not* to equal the token, and revoked vs
> unknown producing byte-identical refusals).
>
> Decisions worth not re-litigating:
> **(1) A token is scoped to one (variant, revision)**, so a share
> never follows the content forward into something nobody meant to
> send — the test rewrites the story and shows the old link still
> renders the old revision.
> **(2) Only the hash is stored.** A stolen database yields no working
> links, and the token appears in exactly one response and in no audit
> row (a test asserts the audit trail never contains it).
> **(3) Every refusal is byte-identical** — unknown, expired, revoked,
> wrong-revision — so the endpoint cannot be used to probe whether a
> guessed token ever existed. The *reason* is audited; the caller
> learns nothing.
> **(4) The guard defers preview paths for a different reason than
> delivery reads**: delivery is authorized by site visibility, preview
> by the token itself. `auth::is_preview_path` names the distinction
> so the guard stays readable.
> **(5) Preview responses are `no-store` + `noindex`**, and only
> published revisions ever reach a sitemap.
- [x] CMS-T23 Webhooks: per-site subscriptions from the event
  record, HTTPS-only non-redirecting client, signed bodies,
  timeout + size cap + bounded retries + delivery log. (CMS-D12;
  CMS-R23)

> CMS-T23 landed 2026-07-30. Migration `m20260730_000010_webhooks`
> adds `webhooks` + `webhook_deliveries` (unique on
> `(webhook_pid, event_id, attempt)`, which is what makes a rerun
> safe). Pure `rules::webhook` does the signing, the URL policy, the
> backoff schedule, and the retryable-status table;
> `controllers::webhooks` registers, lists, withdraws, logs, and
> dispatches; `task webhook_dispatch` is the same function without an
> HTTP round trip. **222 DB-free unit tests, 55 request tests, 1
> enforcement binary, 3 delivery tests — all green against Postgres
> 18**; clippy-pedantic + `cargo fmt --check` + `cargo deny` clean.
>
> Decisions worth not re-litigating:
> **(1) The timestamp is inside the signature** (`{timestamp}.{body}`,
> HMAC-SHA256), so a captured delivery cannot be replayed later
> against a receiver that checks freshness. Signing the body alone
> would leave exactly that hole, and a test asserts a shifted
> timestamp fails to verify.
> **(2) Not the family `integrity-mac` crate, and not hand-rolled.**
> `integrity-mac` protects *stored rows* with a key the service alone
> holds and never shares; a webhook secret must be handed to a third
> party so they can verify — a different problem with a different key
> lifecycle. `hmac` 0.12 (the RustCrypto pairing for the `sha2`
> already present) is used rather than a hand-rolled construction,
> because HMAC is short enough to look easy and exactly the kind of
> code that fails silently.
> **(3) Dispatch refuses under the in-memory transport** rather than
> delivering a subset that disappears on restart. `422` names the
> setting that fixes it (`CMS_EVENT_TRANSPORT=outbox`). A delivery
> path that quietly drops events is worse than no delivery path.
> **(4) HTTPS only, loopback excepted** — the family's standing rule
> for server-side fetches (`security.md` invariant 7). The exception
> is not a testing convenience dressed up as policy: plain HTTP to
> `127.0.0.1` never leaves the host, so the confidentiality argument
> does not apply. It *also* makes the success path testable against a
> real receiver, which is why these tests verify the signature the way
> a third party would rather than asserting about a mock.
> **(5) Retries are scheduled, not slept.** A failed attempt is
> recorded and picked up by the next dispatch once its backoff has
> elapsed (0/30/120/480/1920s, five attempts); nothing is held in
> memory, so nothing is lost on restart. A 4xx other than 408/429 is
> **not** retried — the receiver understood and refused, and repeating
> it unchanged is noise.
> **(6) The secret is stored recoverably** (unlike a preview token,
> which is hashed) because the receiver must hold the same secret to
> verify. It is returned by exactly one response and by no read, and a
> test asserts it appears in neither a listing nor an audit row.
> **(7) Registration is guarded like any other mutation** — a
> subscription is an outbound disclosure channel, so the enforcement
> binary pins anonymous `401`, reader `403`, and dispatch `401`.
- [x] CMS-T24 Seed task: a synthetic site (~3 content types, ~40
  entries across 3 locales, revisions in every workflow state, ~25
  assets incl. one orphan, a menu, a redirect chain, two stale
  translations, planted health findings) — synthetic data only.
  (CMS-R24)

> CMS-T24 completed 2026-07-30. The declaration slice landed with
> CMS-T1; `tasks::seed_corpus` now adds the content: **29 entries, 50
> variants across `en`/`fr`/`fr-CA`, 25 assets (3 orphans), a menu, a
> four-hop redirect chain, two stale translations, an open translation
> request, a future scheduled publish, and the backdated `variant`
> audit rows the throughput view is derived from**. 223 DB-free unit
> tests + 58 request tests + 1 enforcement binary + 3 delivery tests
> green against Postgres 18.
>
> Decisions worth not re-litigating:
> **(1) One instance of every health rule is planted on purpose**, on
> an entry whose key names the rule (`plant-stale-content`,
> `plant-no-alt-text`, …). A demo whose insights are empty teaches
> nothing and one whose findings appear by accident teaches the wrong
> thing. A request test asserts all ten fire, so a rule that stops
> working fails the suite rather than quietly emptying the demo.
> **(2) The corpus is mostly healthy** — 15 findings against 39
> published variants — and the test pins that ratio, because a fixture
> that is all findings misrepresents what the tool is for.
> **(3) Rows are backdated explicitly.** Half the rules and every
> duration percentile are *about* elapsed time; a fixture whose rows
> are all a second old cannot show a stale page or a stuck review.
> **(4) Asset bytes are not written.** The rows describe files that
> were never uploaded, so a seeded asset serves metadata but not
> content. That is the one place the demo diverges from a real upload,
> and the task logs it rather than leaving it to be discovered.
> **(5) The scheduled publish is in the future**, so the first sweep
> does not quietly change the demo.
>
> Two defects the fixture exposed, both invisible to a row count:
> **(a)** `add_variant` minted its own pid while the caller had
> already minted one for the revisions to name, so every French route
> and reference pointed at a variant that was never inserted — visible
> only because the orphan count came out at 12 instead of 3.
> **(b)** the `needs_migration` health rule consulted only
> `validate_values`, which inspects fields that are *present*, so the
> commonest migration of all — a field that became required after the
> content was written — went unreported even though the publish gate
> refuses that content. The rule now consults `missing_required` too.

## Phase 9 — front-end (all CMS-R*)

- [x] CMS-T25 Scaffold
  `content-management-system-front-end-with-svelte`: SvelteKit 2 +
  Svelte 5 runes SPA, BFF proxy + session flow, 13-locale i18n from
  the start, typed API client. (CMS-D17)

> CMS-T25 landed 2026-07-31: the app shell, the BFF proxy + magic-link
> session flow, the typed API client, 13-locale i18n with its parity
> test, the Lily locale/theme pickers, and a dashboard over the
> content-health and backlog views. **12 vitest tests + 2 Playwright
> specs**, `svelte-check` clean, verified live end to end against the
> seeded service.
>
> Decisions worth not re-litigating:
> **(1) The proxy refuses the preview-token surface.**
> `POST …/variants/{locale}/preview` returns a credential that renders
> unpublished content; forwarding it would put that credential in
> browser JavaScript. The refusal is a `403` naming the alternative,
> not a silent drop — a quiet failure sends the next contributor to
> debug the service for a decision made in the client.
> **(2) Preview is a server round trip**: `/preview/{pid}/{locale}`
> mints, renders, and **revokes**, returning only the render with
> `no-store` + `noindex`. Revoking rather than letting the token
> expire matters because it has already been spent by the time the
> response returns; a live credential kept for no reason is exposure.
> **(3) Endpoint paths were checked against the running service's
> OpenAPI document**, not merely written to look right — which is how
> `transition` was found to be singular where the obvious guess was
> `transitions`. A unit test pins the strings; only the comparison
> catches a plausible-but-wrong path.
> **(4) The client formats, it does not compute.** Every number on the
> dashboard comes from the API, so the UI cannot disagree with the
> service, and a `null` ratio renders as "no data yet" rather than
> `0%`.
>
> Two things found along the way:
> **(a)** Lily renamed its helper packages `*-select` → `*-picker`, so
> the fifteen existing front-ends' `file:` dependency paths no longer
> resolve — copy-adapting a sibling's `package.json` fails to install.
> The prop contracts are unchanged, so adapting is a rename.
> **(b)** the i18n holder read `localStorage` whenever `browser` was
> true. That is not safe (Safari private mode throws on access), and
> because it runs in a module-level constructor it took the whole app
> down rather than degrading the locale switcher. Both reads and
> writes are now guarded.
- [x] CMS-T26 Views: entry list + structured block editor, revision
  history + diff + restore, review queue + workflow actions,
  schedule calendar, asset library (usage + alt-text gate),
  translation dashboard with staleness, site settings (locales,
  fallback, templates, menus, redirects), delivery preview panel,
  content-health + throughput insights; vitest +
  `page.route`-stubbed Playwright. (CMS-D6, CMS-D17)

> CMS-T26 landed 2026-07-31. Seven views (`/entries`,
> `/entries/{pid}`, `/assets`, `/workflow`, `/translations`,
> `/insights`, `/settings`) plus the dashboard, over a pure block
> model and pure formatters. **28 vitest tests + 7 Playwright specs**,
> `svelte-check` and prettier clean, and every view rendered in a real
> browser against the seeded service with no console or page errors.
>
> Decisions worth not re-litigating:
> **(1) A lost save is a comparison, never a retry.** A `409` renders
> the competing revision and its author; a retry button would silently
> discard whoever won the race. A Playwright spec drives the `409`.
> **(2) The block editor edits blocks.** No `contenteditable`, nothing
> that serializes to markup, and no `{@html}` anywhere — the service's
> sanitizer is a boundary control, not permission to trust its output
> in the client. There is deliberately no `toHtml`/`fromHtml` helper,
> because a helper like that gets used and the round trip stops being
> lossless the moment it is.
> **(3) Refusals carry their remedy.** Publish blockers show the rule
> *and* what to do; an image block says alt text is missing in terms
> of the consequence ("before this page can be published"). A refusal
> an author cannot act on is a locked door.
> **(4) Restore says it writes a new revision** before doing it,
> because "restore" reads like undo and it is not.
> **(5) The client formats, it never computes.** Percentages come from
> `value`, counts from the payload. A `null` ratio renders as no-data:
> the live check found 6 no-data cells and **zero** literal `0%`.
> Staleness keeps three outcomes — up to date, N behind, and
> *unknown* — because collapsing "unknown" into "up to date" tells a
> translator their page is fine when nobody knows.
>
> Three defects found by checking rather than assuming:
> **(a)** the T25 path check compared paths but **not verbs**, and had
> passed a `GET` on `/api/entries/{pid}/variants` — which the service
> serves for `POST` only. There is no variants listing; the entry read
> returns them. Re-verified with methods, every call now matches.
> **(b)** `listRevisions` and `publishCheck` had the wrong return
> types (the history endpoint returns summaries without bodies);
> `svelte-check` caught both once a view consumed them.
> **(c)** the insights view keyed an `{#each}` on rule+subject, and two
> `broken_reference` findings on one page share both — a duplicate-key
> crash that emptied the whole view. Only rendering it against real
> data showed it; the stubbed spec had one finding per rule. Keys are
> now positional wherever the item has no id.

## Production gates (before any non-demo exposure)

- [ ] CMS-G1 Activate `CMS_REQUIRE_AUTH` + mount a real ABAC
  policy; verify the five-persona matrix **and the public-delivery
  allow-list** against the deployment's attributes and site
  visibilities.
- [ ] CMS-G2 Public-surface hardening review: CSP / CORS /
  rate limits / CDN + WAF in front of `/delivery`, asset-serving
  headers, and preview-token scope
  ([regulatory.md](regulatory.md)).
- [ ] CMS-G3 Accessibility, records-management, and rights review:
  WCAG 2.2 AA audit of the rendering channel, retention schedules,
  image consent + asset licensing, erasure-by-redaction procedure
  ([regulatory.md](regulatory.md)).

- [ ] CMS-T27 **Auth activation surface (the code side of CMS-G1).**
  Unlike its two sibling apps (WPM-T31, CRM-T22), CMS ships no
  reference ABAC policy at all (verified: `find config -iname
  "*abac*"` is empty, vs. WPM's and CRM's own
  `config/abac-policy.reference.json`); CMS-G1 above is still fully
  `[ ]`. **Acceptance:** a `config/abac-policy.reference.json`
  encodes the `auth.md` personas (svc/admin, editor write + masked
  read, `resource.owner = $sub` self-read where applicable,
  masked-read fallback) plus an activation runbook section in
  `auth.md`; `tests/enforcement.rs` mounts the shipped file and pins
  at least a masked-vs-unmasked read and one destructive-POST
  admin/svc-only case; `cargo test` plus the DB-gated enforcement
  suite green; clippy pedantic clean. (CMS-G1)

- [ ] CMS-T28 **Wire record-level ABAC into the sites/content-types
  handlers.** `auth::site_resource_attrs` and
  `auth::content_type_resource_attrs` are defined and unit-tested in
  `auth.rs`, but neither is called from `controllers/sites.rs` or
  `controllers/types.rs` (verified: `grep -rn
  "site_resource_attrs\|content_type_resource_attrs"
  src/controllers/*.rs` returns nothing, while `controllers/entries.rs`
  correctly calls `auth::variant_resource_attrs` +
  `authorize_record` on every read/write) — the same
  defined-but-unwired pattern `auth.rs`'s own test docstring warns
  against ("`authorize_record` computing an obligation nobody applies
  is exactly the failure this helper exists to prevent"). **Acceptance:**
  the site and content-type GET/PUT/DELETE handlers call
  `authorize_record` with their resource attrs; the enforcement
  matrix gains an owner-vs-non-owner pin for at least one site and
  one content-type endpoint; `cargo test` plus the DB-gated
  enforcement suite green; clippy pedantic clean. (CMS-D17)

- [ ] CMS-T29 **Subject rights & retention (the code side of
  CMS-G3).** CMS has no retention or subject-rights code at all — no
  `CMS_RETENTION_DAYS`, no sweep task, no subject-access/erase
  endpoint (verified: `grep -rln "retention\|subject_access\|erase"
  src/` matches nothing) — unlike WPM-T30 and CRM-T21, which both
  landed this for their personal-data surface. CMS's personal-data
  surface is narrower but real: `author_ref`/`reviewer_ref` (worker
  `EntityRef` URNs, confirmed in
  `migration/src/m20260730_000005_entries.rs`) on `entries`/
  `revisions`. **Acceptance:** `rules/privacy.rs` (pure: the floored
  retention horizon + a sweep-list pin) and `controllers/privacy.rs`
  (a subject-access listing of every entry/revision naming a given
  worker as author or reviewer, and a scrub of the `author_ref`/
  `reviewer_ref` fields on request, refused while the content is in
  an active, non-terminal editorial state) join
  `DESTRUCTIVE_POST_SUFFIXES`; a DB-gated round-trip test; `cargo
  test` green; clippy pedantic clean; fmt clean. (CMS-G3)

- [ ] CMS-T30 **`require_ref` EntityType coverage.**
  `controllers/localization.rs`, `entries.rs`, `workflow.rs`, and
  `sites.rs` pass both `EntityType::Worker` and
  `EntityType::Organization` to the shared `require_ref` helper, but
  `validation.rs`'s own `ref_rules` test only ever passes
  `EntityType::Worker` (verified: `grep -n "EntityType::"
  src/validation.rs` vs `grep -rn "entity_ref::EntityType::"
  src/controllers/*.rs`). **Acceptance:** `ref_rules` exercises the
  wrong-type branch for `Organization` too; `cargo test` green;
  clippy pedantic clean. (CMS-D16)

- [ ] CMS-T31 **Front-end sign-in gate.** No `+layout.server.ts`
  exists under `src/routes` (verified: `find src/routes -iname
  "+layout.server.ts"` returns nothing); every authoring/asset/
  workflow view is reachable signed-out and only fails silently at
  the API layer. **Acceptance:** a root or per-protected-route guard
  redirects a session-less visitor to sign-in (excluding sign-in/
  verify and any deliberately public `/delivery`/`/preview`
  surface); a Playwright spec pins it; svelte-check 0. (CMS-D17)
