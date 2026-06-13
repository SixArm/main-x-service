## 13. Tasks

Entity-level (cross-subproject) work breakdown. Crate-internal work
belongs in the owning subproject's §13 — link it from here only when
it blocks an entity-wide goal. Tick the box when an automated test or
clearly described manual check confirms the acceptance criterion.

- [x] **T-1 — Repair post-nesting relative links in subproject docs.**
  - [x] The repo re-nested each entity trio under an entity directory
    (`thing/`, `person/`, …), so subproject links written for the flat
    layout now dangle: e.g. service
    [`spec/17-references.md`](../thing-service-rust-crate/spec/17-references.md)
    points at `../../person-service-rust-crate/` and service
    [`AGENTS/index.md`](../thing-service-rust-crate/AGENTS/index.md)
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
    service [`AGENTS/restful.md`](../thing-service-rust-crate/AGENTS/restful.md)
    says `POST /api/things/duplicates`.
  - **Acceptance:** restful.md matches the routes in
    `src/api/rest/mod.rs`.
  - *Done 2026-06-13: verified against `src/api/rest/mod.rs` route
    table and the utoipa path annotation; restful.md corrected.*
- [x] **T-3 — De-drift copy-pasted sibling prose.**
  - [x] Matcher [`AGENTS.md`](../thing-matcher-rust-crate/AGENTS.md)
    quick-orientation table describes "geographic-place records",
    `Place` / `Address` types, and place-matcher rules.
  - [x] Service [`AGENTS/spec-driven-development.md`](../thing-service-rust-crate/AGENTS/spec-driven-development.md)
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
- [ ] **T-5 — Verify the front-end build and run a live walkthrough.**
  - [ ] `pnpm install` and `pnpm test` verified (front-end §14 marks
    both ❌).
  - [ ] Operator walkthrough of every route against a running thing
    service.
  - **Acceptance:** front-end §14 rows flip to ✅ with the command
    output or walkthrough notes linked.
- [ ] **T-6 — Entity-wide SSO enforcement.**
  - [ ] Service JWT middleware with editor / read-only / service roles
    (service §13 T-4), verifying offline against the authentication
    entity's JWKS.
  - [ ] Front-end sign-in flow + token attachment (front-end §15 v0.3).
  - **Acceptance:** unauthenticated REST request → `401`;
    authenticated operator completes a create through the UI.
- [ ] **T-7 — Wire the four unrouted endpoints into the operator UI.**
  - [ ] `check-duplicates` preview on the create form; batch
    `deduplicate` results view; masked-view toggle; GDPR-export
    download (front-end §13 T-17–T-20).
  - **Acceptance:** Playwright e2e covers each new route.
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
    [`AGENTS/matching.md`](../thing-service-rust-crate/AGENTS/matching.md)
    ("Relationship to the embedded matcher's confidence bands"), with
    a pointer from entity [`AGENTS/matching.md`](../AGENTS/matching.md).*
  - *Progress 2026-06-13 (code): confirmed the service re-derives
    `MatchConfidence` solely via `MatchConfidence::from_score` from the
    raw score — `compute_match` (`src/matching/scoring.rs`) and
    `confidence_label`/`score` (`src/matching/mod.rs`) never translate
    the matcher's `Confidence` label; the adapter (`adapter.rs`) carries
    only the domain record. Added §5.3 normative note + boundary unit
    test `test_confidence_boundary_pins`. Remaining open: the
    API-facing-vocabulary decision (OQ-2).*
