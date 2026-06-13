## 13. Tasks

Entity-level work queue (cross-subproject items; single-subproject
work lives in the owner's §13). Tick the box when an automated test
or clearly described manual check confirms the acceptance criterion.
Prefix `ET-` distinguishes these from per-crate task numbers.

- [x] **ET-1 — Rewrite the matcher spec for the 0.5.0 event surface.**
  `event-matcher-rust-crate/spec/` §1, §3, §5, §7 and the worked
  examples still describe the 0.4.x **place** matcher; the index
  carries a "partially superseded" notice.
  - [x] Rewrite data-model, pipeline, and weight sections against `src/`.
  - [x] Remove the supersession notice.
  - **Acceptance:** the matcher's spec-drift check script passes and
    no spec section names `Place` types.
  - **Done 2026-06-13.** §1–§3, §5–§7, §10–§12 rewritten against
    `src/` (0.6.1); §4 gained §4.7 (ISO 8601 parsing) and lost its
    stale phone-scoring cross-ref; §8/§9 place mentions fixed;
    version banner now `0.6.1` / "living"; supersession notices
    removed from `spec/index.md` and `AGENTS.md`. Remaining `Place`
    mentions are schema.org vocabulary (`schema:Event.location`
    union) and the historical 0.5.0 note in §9 — no crate types.
    Spec-drift check passes trivially (no `src/matcher.rs` change).
- [x] **ET-2 — Purge person-entity copy drift from the front-end.**
  The front-end README describes the detail route as "identity,
  identifiers, addresses, telecom, emergency contacts", its layout
  doc names `HumanName` / `HumanNameInput`, and its task T-15 says
  "emergency-contact edit" — all person-service leftovers.
  - **Acceptance:** front-end README + spec §13 describe Event
    fields (time window, locations, parties, offers) only.
  - **Done 2026-06-13.** README route table + layout listing now
    match `src/` (no `HumanName` / `HumanNameInput` — those files
    never existed here); spec §2.1 and FR-4 / FR-5 rewritten against
    the real detail page (identity, locations, organizers,
    performers, identifiers, offers) and `EventForm` validation;
    T-15 rewritten as sub-record edit for identifiers / locations /
    parties / offers.
- [x] **ET-3 — Repair cross-entity links broken by monorepo nesting.**
  Service spec §17 links sibling specs as
  `../../person-service-rust-crate/spec/index.md`, which now resolves
  inside `event/` since entities moved into per-entity directories
  (correct: `../../../person/person-service-rust-crate/...`).
  - **Acceptance:** a link-checker pass over
    `event/**/spec/*.md` and `event/**/AGENTS*` reports no dead
    relative links.
  - **Done 2026-06-13.** Link-checker pass over all `event/**/*.md`
    (excluding `target/`, `node_modules/`) reports 0 dead relative
    links. Fixed: cross-entity links in service spec §17; repo-root
    `../../AGENTS.md` / `../../agents/share/…` → `../../../…` in
    spec + AGENTS files of service and front-end; `../agents/share/…`
    → `../../…` in service `AGENTS.md`; service `CLAUDE.md`
    `@agents/share/…` → `@../../agents/share/…`; renamed shared docs
    (`stack-for-rust-loco.md` → `rust-loco-stack.md`,
    `observability-for-rust-loco.md` →
    `rust-tracing-opentelemetry-stack.md`, `technology.md` →
    `loco.md`); plus pre-nesting rot in the service README/index
    (`ARCHITECTURE.md` → `spec/08-architecture.md`, `API_GUIDE.md` →
    `AGENTS/restful.md`, `task-10.md` → `AGENTS/testing.md`,
    dead `LICENSE*` file links de-linked).
- [ ] **ET-4 — Decide convergence of the two matching algorithms** (§6.1, EOQ-1).
  - [ ] Decide: route `/events/match` scoring through the embedded
    matcher, keep both, or fold in-service components (attendee,
    window-overlap) into the matcher.
  - **Acceptance:** decision recorded here + in service spec §6.2;
    bridge tests updated to pin the chosen path.
- [ ] **ET-5 — SSO across the trio** (service T-8 + front-end).
  - [ ] Service: JWT middleware on `/api/v1/*`, verifying RS256
    against the authentication entity's JWKS.
  - [ ] Front-end: magic-link sign-in flow + token handling.
  - **Acceptance:** unauthenticated REST request → `401`;
    operator can sign in and complete a CRUD round-trip in the UI.
- [ ] **ET-6 — Durable event bus** (service T-4 elevated to entity level).
  Replace `InMemoryEventPublisher` with a durable publisher so
  index-level events survive restarts and can fan out to other
  entities / regions.
  - **Acceptance:** integration test publishes `Created` end-to-end
    through the durable bus.
- [ ] **ET-7 — Live trio integration walkthrough.**
  Front-end spec §14 records `pnpm install` / `pnpm test`
  unverified and "live integration ❌".
  - [ ] Run service + front-end together; verify every route against
    real data, including create-409 and merge.
  - **Acceptance:** front-end spec §14 rows flip to ✅ with the
    verification noted.
- [x] **ET-8 — Pin the language-tag contract.**
  The service validates `in_language` as 2-letter ISO 639-1; the
  matcher documents `in_language` as BCP 47. ~~The adapter currently
  drops the field~~ (correction 2026-06-13: the adapter **does**
  project the first `in_language` entry — `src/matching/adapter.rs`
  reads `e.in_language.first()`, projecting it when non-empty — but
  the matcher treats the field as data-only and never scores it, so
  nothing breaks). Contract now documented **and** test-pinned.
  - [x] Documentation: §5.3 table row added; divergence note in
    service spec §6.2 and service `AGENTS/matching.md`
    (2026-06-13, docs-only round).
  - [x] Bridge test pinning the projection + inertness
    (2026-06-13): `tests/duplicate_detection.rs` adds
    `in_language_first_entry_is_projected` (first entry carried,
    later entries dropped, leading-empty entry projects nothing —
    adapter reads only `.first()`, no scan-ahead) and
    `in_language_difference_does_not_affect_match` (two events
    differing only in `in_language` score identically — the matcher
    never scores the field; there is no `in_language_score` in
    `MatchBreakdown`). `cargo test --test duplicate_detection`: 18
    passed.
  - **Acceptance:** §5.3 table row added; adapter comment + bridge
    test pinning the projection. ✓
- [ ] **ET-9 — Load test at governmental scale.**
  - [ ] Seed millions of synthetic Events; measure NFR-2…NFR-6 and
    re-state them as measured multi-instance figures.
  - **Acceptance:** §7 figures annotated "measured" with date.
