## 13. Tasks

Entity-level work queue (`E-N`). Tasks that are purely crate-internal
belong in the owning crate's §13; tasks here either span subprojects
or fix the integration contract. Tick the box when an automated test
or clearly described manual check confirms the acceptance criterion.

- [ ] **E-1 — Fix duplicate-check endpoint-name drift.**
  - [x] Service `AGENTS/restful.md` lists `POST /api/places/duplicates`;
    service spec §6.4 says `POST /api/places/check-duplicates`; the
    front-end's deferred T-17 says `check-duplicates`. Establish which
    the code serves, fix the losing doc(s), and pin with a route test.
    *(2026-06-13: code serves `check-duplicates` — `src/api/rest/mod.rs`
    routes + utoipa path; `AGENTS/restful.md` was the losing doc and is
    fixed. Still open: a service route test, and a discovered front-end
    **code** bug — `place-front-end-with-svelte/src/lib/api/places.ts`
    POSTs `/api/places/duplicates` (its unit test pins the wrong path)
    and will 404 against the live service.)*
  - [x] Front-end **code** bug fixed: `place-front-end-with-svelte/src/lib/api/places.ts`
    now POSTs `/api/places/check-duplicates`, and its unit test
    (`tests/unit/places.test.ts`) asserts the correct path. Verified by
    executing the front-end unit suite (`vitest run`: 8 passed) and by
    grep-consistency across client + test + service route.
    *(2026-06-13)*
  - **Acceptance:** all three docs + code agree; a service route test
    covers the path.
- [ ] **E-2 — Purge person-entity copy artifacts from the front-end.**
  - [x] README route table says "identifiers, addresses, telecom,
    emergency contacts" and the layout lists `HumanNameInput.svelte`;
    spec §13 T-15 mentions "emergency-contact edit". Places have no
    `HumanName`, telecom lists, or emergency contacts.
    *(2026-06-13: README, spec §2/§6 FR-4–FR-5, and T-15 rewritten to
    the real Place UI — address, geo, identifiers, opening hours,
    amenities; `src/` already had no person artifacts, so no code
    change. `pnpm check` not run this round (docs-only) — left open.)*
  - **Acceptance:** front-end README / spec / `src/lib/api/types.ts` /
    components describe only Place fields; `pnpm check` passes.
- [x] **E-3 — Reconcile matcher version references.** *(2026-06-13:
  banner now `0.6.1`; service spec §6.2 now says registry dependency
  `place-matcher = "0.6.1"`.)*
  - [x] Matcher `spec/index.md` banner says "Version targeted: 0.4.0";
    `Cargo.toml` is `0.6.1`. Service spec §6.2 says "path dependency";
    service `Cargo.toml` declares `place-matcher = "0.6.1"` (registry).
  - **Acceptance:** matcher spec banner matches the crate version;
    service spec §6.2 states the actual dependency form.
- [x] **E-4 — De-Diesel the service `CLAUDE.md` / `README.md`.**
  *(2026-06-13: `CLAUDE.md` Quick Start, migrations section, and
  diagram now describe the SeaORM migration crate + loco CLI
  (`cargo run -- db migrate`); `README.md` had no Diesel mentions.)*
  - [x] Quick Start instructs `diesel_cli` + `diesel migration run` and
    the architecture diagram says "PostgreSQL (Diesel)"; spec §10 says
    SeaORM (and NFR §7 says Loco `bg_pg`).
  - **Acceptance:** user-facing intro matches spec §10 and the loco.rs
    migration workflow.
- [ ] **E-5 — SSO across the trio.**
  - [ ] Service: JWT middleware on `/api/*` verifying against the
    authentication entity's JWKS, with editor / curator / read-only /
    service roles (service spec §13 T-8).
  - [ ] Front-end: sign-in redirect + token attach in `ApiClient`.
  - **Acceptance:** unauthenticated request → `401`; authenticated
    operator round-trips create→audit with a verified `user_id` in the
    audit row.
- [ ] **E-6 — Durable event bus.** Promote `InMemoryEventPublisher` to
  the production Fluvio publisher (service spec §13 T-3) so peer
  entities can consume place events.
  - **Acceptance:** integration test publishes and consumes a
    `PlaceCreated` record end-to-end.
- [ ] **E-7 — PostGIS-backed geo-radius search** (service spec §13 T-1).
  - **Acceptance:** geo-radius ≤ 200 ms p50 at 1 M places.
- [ ] **E-8 — Verify the front-end build.** Run `pnpm install`,
  `pnpm test`, `pnpm test:e2e`, `pnpm check`; fix fallout; update
  front-end spec §14.
  - **Acceptance:** front-end spec §14 rows flip to ✅.
- [ ] **E-9 — Live trio smoke test.** Scripted walkthrough: start
  service + database, run front-end e2e against the live API
  (create → 409 → match → merge → audit).
  - **Acceptance:** one command runs the trio and the smoke suite
    passes.
- [ ] **E-10 — Localize the operator UI.** Externalise front-end
  strings; ship at least `en` plus one pilot locale from
  [`agents/share/locales.md`](../../agents/share/locales.md).
  - **Acceptance:** language switcher renders the list + create routes
    fully translated in the pilot locale.
- [ ] **E-11 — International street-vocabulary normalisation.** The
  matcher expands English-only street abbreviations (matcher spec
  §1.2); evaluate locale-aware vocabularies or document the
  worldwide-deployment consequences in matcher spec §10.
  - **Acceptance:** decision recorded in matcher spec; tests cover at
    least one non-English vocabulary or the documented exclusion.
- [x] **E-12 — GLN check-digit validation (spec-vs-code drift).**
  *(2026-06-13: service spec §6 / §14 and `CLAUDE.md` promised a "GLN
  check digit", but `validate_place` only counted 13 digits. Added
  `validation::gln_is_valid` implementing the GS1 mod-10 check digit and
  wired it into `validate_place`; updated stale test fixtures to real
  GLNs (`0614141999996`, `4006381333931`) and added check-digit unit +
  integration tests. Verified un-gated: `cargo build`, `cargo test --lib`
  (120 pass), `cargo test --test duplicate_detection` (14 pass),
  `integration_validation` / `integration_edge_cases` / `integration_models`
  green, `cargo fmt --check` clean, clippy adds no new warnings.)*
  - **Acceptance:** a GLN with a wrong GS1 check digit is rejected with a
    `422`-eligible `ValidationError`; spec §14 + `CLAUDE.md` describe the
    delivered check; tests pin valid + invalid check digits.
- [x] **E-13 — Opening-hours time validation (spec-vs-code drift).**
  *(2026-06-13: service `CLAUDE.md` listed "Opening hours validation" as a
  delivered Data-Quality feature, but `validate_place` performed none —
  `OpeningHoursSpecification.opens` / `.closes` are free `HH:MM` strings,
  so garbage like `"25:99"` or `"5pm"` was accepted. Added
  `validation::time_is_valid` (24-hour `HH:MM`: 2 ASCII digits, colon,
  2 ASCII digits; hours `00..=23`, minutes `00..=59`) and looped it over
  `place.opening_hours`, reporting indexed field paths
  (`opening_hours[i].opens` / `.closes`). Brought the source of truth into
  agreement: service spec §6.5 + §14.1 now list the opening-hours check.
  Verified un-gated: `cargo test --lib` (124 pass, +4),
  `cargo test --test integration_validation` (4 pass, +1), bridge
  `duplicate_detection` (14 pass), `integration_edge_cases` (16) /
  `integration_models` (13) green, validation doctests (5) green,
  `cargo fmt --check` clean, clippy adds no new warnings.)*
  - **Acceptance:** an opening-hours window with an out-of-range or
    malformed time is rejected with a `422`-eligible `ValidationError`
    carrying an indexed field path; spec §6.5 + §14 + `CLAUDE.md` describe
    the delivered check; tests pin valid + invalid times.
