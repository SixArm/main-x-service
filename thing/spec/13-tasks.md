## 13. Tasks

Entity-level (cross-subproject) work breakdown. Crate-internal work
belongs in the owning subproject's §13 — link it from here only when
it blocks an entity-wide goal. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **T-1 — Repair post-nesting relative links in subproject docs.**
  - [x] The repo re-nested each entity trio under an entity directory
    (`thing/`, `person/`, …), so subproject links written for the flat
    layout now dangle: e.g. service
    [`spec/17-references.md`](../thing-service-with-loco/spec/17-references.md)
    points at `../../person-service-with-loco/` and service
    [`agents/index.md`](../thing-service-with-loco/agents/index.md)
    points at `../../agents/share/` — both resolve inside `thing/`
    and miss.
  - **Acceptance:** a link-checker pass over `thing/**/*.md` reports
    zero broken relative links.
  - *Done 2026-06-13: 59 broken links repaired (root `../`→`../../`
    and `../../`→`../../../` hops, cross-entity sibling paths, renamed
    shared docs `stack-for-rust-loco`→`rust-loco-stack`,
    `observability-for-rust-loco`→`rust-tracing-opentelemetry-stack`,
    `technology`→`loco`, and service `CLAUDE.md` `@`-includes).
    Link-checker pass reports zero broken relative links and all
    `@`-includes resolve.*
- [x] **T-2 — Fix duplicate-check endpoint doc drift.**
  - [x] Code and OpenAPI use `POST /api/things/check-duplicates`;
    service [`agents/restful.md`](../thing-service-with-loco/agents/restful.md)
    says `POST /api/things/duplicates`.
  - **Acceptance:** restful.md matches the routes in
    `src/api/rest/mod.rs`.
  - *Done 2026-06-13: verified against `src/api/rest/mod.rs` route
    table and the utoipa path annotation; restful.md corrected.*
- [x] **T-3 — De-drift copy-pasted sibling prose.**
  - [x] Matcher [`AGENTS.md`](../thing-matcher-rust-crate/AGENTS.md)
    quick-orientation table describes "geographic-place records",
    `Place` / `Address` types, and place-matcher rules.
  - [x] Service [`agents/spec-driven-development.md`](../thing-service-with-loco/agents/spec-driven-development.md)
    section-mapping table references Event-service concepts
    (`Location` / `Party` / `Offer`, time window, iCalendar, FHIR §6.8).
  - [x] Front-end [`README.md`](../thing-front-end-with-svelte/README.md)
    route table and spec §13 T-15 mention "addresses, telecom,
    emergency contacts" — person-service fields a Thing does not have.
  - **Acceptance:** each doc describes only Thing concepts; spot-check
    against [§5](05-domain-model.md).
  - *Done 2026-06-13: all three rewritten against actual code
    (matcher `src/lib.rs` / `src/matcher.rs`, service spec §5–§6,
    front-end `src/`). Note: the matcher's `CHANGELOG.md` still
    carries heavily place/person-flavoured historical entries —
    left as-is because true pre-nesting history is not in this repo.*
- [x] **T-4 — Align matcher spec version banner with the shipped crate.**
  - [x] [`thing-matcher spec/index.md`](../thing-matcher-rust-crate/spec/index.md)
    says "Version targeted: 0.4.0"; `Cargo.toml` ships `0.6.1` (and
    the service depends on `0.6.1`).
  - **Acceptance:** spec banner, `Cargo.toml`, and the service's
    pinned dependency agree.
  - *Done 2026-06-13: banner → `0.6.1`; install snippet in matcher
    `index.md` (= `README.md`) → `0.6.1`; spec §7.3 stability note
    updated. All three now agree with the service's pin. Open: the
    matcher CHANGELOG's latest entry is headed "0.6.0" with no
    `0.6.1` entry — not reconstructable from this repo's history.*
- [x] **T-5 — Verify the front-end build and run a live walkthrough.**
  *(Re-verified 2026-09-08 — the premise was stale (front-end §14's
  `pnpm install`/`pnpm test` rows have read ✅ since 2026-08-04) and
  the live walkthrough itself had never actually been run.)*
  - [x] `pnpm install` and `pnpm test` verified (front-end §14: ✅
    since 2026-08-04, 63/63 then, 94/94 now).
  - [x] Operator walkthrough of every route against a running thing
    service. **Done 2026-09-08**: a real `thing-service` (Postgres via
    `scripts/test-db.sh`) + the front-end dev server, driven through
    every route (dashboard, list/search, detail, masked toggle, GDPR
    export, per-thing audit, match form, sign-in, and every
    sign-in-guarded route confirmed redirecting rather than crashing).
    Found and fixed a real, previously-undiscovered defect this way —
    front-end T-30 (`data.items` vs the service's real `data.results`
    field name crashed the list page on every real search) — and found
    a second one, at the time left open as service T-15 (the list
    page's `q="*"` "list everything" default never returns anything
    against the real service; a service-side gap, not fixed in that
    pass). **T-15 landed 2026-09-09**: a real `GET /api/things`
    collection-list endpoint (database-backed, not the search index —
    same shape as `person-service`'s reference `GET /api/persons`),
    with the family pagination headers, and the front-end's `/things`
    page now calls it (`ThingRepository.list`) whenever the query box
    is empty instead of faking a `q="*"` search. Verified live: the
    old bug reproduces exactly as before on `/things/search?q=*`
    (untouched, still zero hits) while the new `GET /api/things`
    returns the seeded record with correct `X-Total-Count`/`X-Limit`/
    `X-Offset` headers, and `GET /api/things` (no query) now answers
    `200` where it used to be a bare `405`.
  - **Acceptance met**, and then some: the walkthrough surfaced two
    real defects a green test suite had been hiding, exactly the
    property this task existed to check for.
- [x] **T-6 — Entity-wide SSO enforcement.** *(Re-verified 2026-09-08
  — already landed; the JWT/JWKS wording below predates the family's
  JWT→PASETO pivot, agents/share/authentication-sessions.md.)*
  - [x] Service auth: `src/api/rest/auth.rs` verifies offline PASETO
    v4.public tokens against the authentication service's published
    Ed25519 keys, behind the blanket `THING_REQUIRE_AUTH` guard
    (default off, per family convention) — not JWT/JWKS, which this
    family decommissioned family-wide.
  - [x] Front-end sign-in flow + token attachment: this crate's own
    BFF (magic-link `/signin`+`/verify`, session cookie → PASETO
    exchange via `src/lib/server/`, confirmed present) with a
    page-visit guard (PRO-H10) redirecting an anonymous visitor away
    from every mutating route — confirmed live in the T-5 walkthrough
    above (`/things/new`, `/things/{id}/edit`, `/things/merge`,
    `/review` all redirected to `/signin` when unauthenticated).
  - **Acceptance met:** unauthenticated REST request → `401` when the
    guard is on (confirmed against `src/api/rest/auth.rs`); an
    unauthenticated operator is redirected before reaching a create
    form, confirmed live.
- [x] **T-7 — Wire the four unrouted endpoints into the operator UI.**
  *(Re-verified 2026-09-08 — already landed for all four; this row
  was simply never checked off.)*
  - [x] `check-duplicates` preview on the create form — superseded by
    root `tasks.md` WEB-6's family-wide finding: every `/new` page
    (including this one, confirmed by grep) already renders the
    `409` candidates the create handler returns, so a separate
    "preview" surface was never needed.
  - [x] Batch `deduplicate` results view — `/review` (the review-board
    route), confirmed present and covered by
    `tests/e2e/things.spec.ts`.
  - [x] Masked-view toggle — confirmed on the detail page, and
    exercised live in the T-5 walkthrough.
  - [x] GDPR-export download — confirmed on the detail page
    (`repo.exportGdpr`) and exercised live in the T-5 walkthrough
    (downloaded a real file from a real service); front-end
    `AGENTS.md` incorrectly still listed this as out of scope —
    corrected in the same pass (front-end T-30 / CHANGELOG).
  - **Acceptance met:** Playwright e2e covers all four
    (`tests/e2e/things.spec.ts`: masked toggle, GDPR export download,
    review board, and the create form's duplicate-candidate render).
- [x] **T-8 — Reconcile the two match-confidence vocabularies.**
  - [x] Service responses use Certain / Probable / Possible / Unlikely
    (thresholds 0.95 / 0.80 / 0.60); the embedded matcher returns
    High / Medium / Low (0.90 / 0.75). Resolved by re-classifying from
    the raw `f64` score (never label→label) at the scoring boundary;
    final API-facing-vocabulary choice still tracked in §16 OQ-2.
  - **Acceptance:** §5.3 documents the mapping; a bridge test pins it.
    ✓ §5.3 now carries the normative "Confidence-vocabulary bridge"
    note; `MatchConfidence::from_score`'s `test_confidence_boundary_pins`
    unit test pins the exact cut points (0.95, 0.90, 0.80, 0.75, 0.60).
  - *Progress 2026-06-13 (documentation): the two scales have
    **no 1:1 label mapping** — the cut points interleave
    (0.95/0.80/0.60 vs 0.90/0.75), so e.g. matcher High spans service
    Certain plus the top of Probable. A score-range overlay table is
    documented in service
    [`agents/matching.md`](../thing-service-with-loco/agents/matching.md)
    ("Relationship to the embedded matcher's confidence bands"), with
    a pointer from entity [`agents/matching.md`](../agents/matching.md).*
  - *Progress 2026-06-13 (code): confirmed the service re-derives
    `MatchConfidence` solely via `MatchConfidence::from_score` from the
    raw score — `compute_match` (`src/matching/scoring.rs`) and
    `confidence_label`/`score` (`src/matching/mod.rs`) never translate
    the matcher's `Confidence` label; the adapter (`adapter.rs`) carries
    only the domain record. Added §5.3 normative note + boundary unit
    test `test_confidence_boundary_pins`. Remaining open: the
    API-facing-vocabulary decision (OQ-2).*
