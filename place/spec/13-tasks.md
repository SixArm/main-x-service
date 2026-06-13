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
