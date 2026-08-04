# Main X Index — Improvement Program Tasks

> Companion to [plan.md](plan.md) — read that first (themes, sequencing,
> working agreements, green gate). Execute tasks roughly in phase order;
> within a phase, respect `Depends`. Tick a task only when its gate is
> green **and** it is merged to `main`. Behavioural changes are
> three-part (crate spec §13 + code + tests) plus the crate CHANGELOG.
> Do **not** create plan/tasks files inside any crate.
>
> Sizes: S ≈ one focused session-slice · M ≈ a substantial slice or a
> delegated subagent build · L ≈ multiple slices, split before starting.

---

## Phase 1 — Truth & hygiene

- [x] **H-1 (S)** Pin the Rust toolchain. *(done 2026-07-12)*
  Add a repo-root `rust-toolchain.toml` (stable, pinned minor) so rustfmt
  drift cannot recur. Run `cargo fmt --check` across all crates after
  pinning; fix any residue in one `Style:` commit.
  *Verify:* `cargo fmt --check` clean in every crate listed by
  `find . -name Cargo.toml -not -path '*/target/*'`.
  *Result:* pinned `1.96.1` (+ rustfmt/clippy, minimal profile); all 37
  crates already `fmt --check`-clean, so no `Style:` fixup was needed.

- [x] **H-2 (M)** Make `agents/share/overview.md` honest. *(done 2026-07-15)*
  Replaced the overclaiming "What every crate provides" bullet list with a
  grounded **capability matrix**: a verified common baseline (CRUD, matching,
  merge, audit, in-memory events, REST/OpenAPI, PASETO verify + blanket ABAC,
  observability, Postgres) for the ten entity registries, then a ✅/– matrix
  of the nine capabilities that **vary** by crate, plus a note on the two
  cross-cutting services (auth, link-graph). Every cell grounded by grepping
  the tree for the live `src/` module — Tantivy + privacy: person/worker/
  place/thing/event/course (6); FHIR: those 5 (not course) + org/care-pathway/
  case (8); gRPC: person/worker/event (3); durable outbox: all but course (9);
  boundary normalization: person/worker/place/event (4); record-level ABAC +
  cross-service links: person/worker/case (3); bulk: person (1).
  *Verified:* every ✅ maps to an existing `src/` module (no overclaims).

- [x] **H-3 (M)** Rewrite `agents/share/architecture.md`. *(done 2026-07-15)*
  Replaced the pre-loco person-only layout (stale endpoint counts, a
  person-specific module tree, no loco/outbox/aggregator/auth) with a
  one-page family description: the 12-service family + libraries, one layered
  request-flow diagram, the **two internal shapes** (person-style
  `src/api/rest/` — person/worker/course + place/thing/event mid-conversion;
  loco-style `src/controllers/` matcher-DTO-as-JSONB — organization/
  care-pathway/case/portfolio), the cross-cutting subsystems (PASETO+ABAC
  auth stack, in-memory→outbox→Fluvio event bus, the link-graph aggregator),
  shared design patterns, and the create/merge/link data flows.
  *Verified:* every named file/module (person-style tree, loco-style tree,
  link-graph modules, migration location) checked to exist in the tree —
  fixed one stale reference (migrations are crate-root `migration/`, not
  `src/migration/`, except authentication).

- [x] **H-4 (M)** Roll CI `--include-ignored` to every service CI. *(done 2026-07-12)*
  Case's `.github/workflows/ci.yaml` test job is the pattern (Postgres
  service + `cargo test --all-features --all -- --include-ignored`).
  Apply to the other nine service crates' workflows (and link-graph).
  Check each service's `config/test.yaml` DB name matches its CI env.
  *Verify:* per-crate workflow lints (yamllint or careful review); the
  DB-gated suites at least compile locally (`cargo test --no-run`).
  *Result:* all 12 services now run `--include-ignored` against a Postgres
  service with a matching DB name. Category A (`ci.yaml` + PG: authentication,
  organization, portfolio) got `-- --include-ignored`; care-pathway's two
  test steps consolidated into one `--include-ignored` run; Category B
  (`test.yml`: person, worker, event) had `--test api_integration_test`
  replaced with `--all-features --all -- --include-ignored`; Category C
  (course, place, thing had no PG test job) got a new self-contained
  `test.yml`; link-graph (no workflow at all) got a full `ci.yaml`. All
  workflows YAML-validated; no `[features]` section anywhere so
  `--all-features` enables nothing new.

- [~] **H-5 (M)** Release hygiene: cut CHANGELOG releases for crates with
  large `[Unreleased]` sections (person, case, care-pathway, organization,
  portfolio, link-graph, authentication-service, authentication-verifier,
  entity-ref); tag (`<crate>-vX.Y.Z`); decide/execute crates.io publish
  for `entity-ref` and `authentication-verifier` 0.8 (both are
  dependency-light and publishable).
  *Verify:* CHANGELOGs have dated release headings; tags pushed to both
  remotes.

  **Done (2026-08-04):** Cut + tagged 4 of the 9 named crates, using the
  Cargo `[package] name` as the tag token (matches what `cargo publish`
  keys on): `person-service-v0.5.0`, `care-pathway-service-v0.1.0`,
  `organization-service-v0.1.0`, `authentication-service-v0.1.0`. Each
  is one commit (`## [Unreleased]` renamed to a dated `## [X.Y.Z] -
  2026-08-04` heading at the *current, unbumped* Cargo.toml version,
  fresh empty `## [Unreleased]` added above it) + one annotated-free
  lightweight tag on that commit. Commits pushed (`git push origin
  main`) then all 4 tags pushed in one call; landed on **both** remotes
  — verified via `git ls-remote --tags` against both
  `git@github.com:SixArm/main-x-service.git` and
  `git@codeberg.org:SixArm/main-x-service.git` directly (not just
  `origin`, since `origin` fans out to both), same 4 SHAs on each.

  **Skipped, and why** (not forgotten — each is a real blocker
  discovered while executing, not a scope-narrowing choice):
  - **`case`, `project-portfolio-management`, `link-graph`,
    `authentication-verifier`** — cutting a release under the *current*
    Cargo.toml version would create a **second, contradictory
    `## [X.Y.Z]` heading** in the same CHANGELOG: each already has a
    real, dated, previously-committed release at that exact version
    number (`case` `[0.1.0] - 2026-06-13`, portfolio `[0.1.0] -
    2026-06-18`, link-graph `[0.1.0] — 2026-06-16`,
    authentication-verifier `[0.8.0] - 2026-07-05`) — and Cargo.toml was
    never bumped past it despite substantial work landing since (event
    bus, ABAC, Tantivy search, bulk import/export, cargo-fuzz + SEC-V1/
    V2/V4 for the verifier, …). This task's own instructions forbid
    bumping the version (a separate decision), so there is no version
    number that both (a) matches Cargo.toml and (b) doesn't collide
    with an existing entry — releasing here needs a version-bump
    decision first, out of scope for H-5 as scoped. No CHANGELOG edit,
    no commit, no tag for these four.
  - **`entity-ref`** — has no `CHANGELOG.md` at all (confirmed:
    `link/entity-ref-rust-crate/` contains only `Cargo.toml`,
    `Cargo.lock`, `deny.toml`, `README.md`, `src/`). Nothing to cut a
    release from; would need a CHANGELOG created from scratch, which is
    a different task than "release hygiene."
  - **crates.io publish** for `entity-ref` and `authentication-verifier`
    — deferred **per explicit user instruction** for this pass, not
    forgotten and not blocked by the above; `cargo publish` was not run
    for either crate.

## Phase 2 — Capability completion (four newest loco services)

- [x] **S-1 (L)** Tantivy full-text search in **organization**.
  *(done 2026-07-31)* `src/search/{index,mod}.rs` — schema (`pid`
  stored; name / legal name / alternate names / Soundex codes /
  identifiers / keywords / flattened address / url full-text;
  `jurisdiction` + `active` exact) and a `SearchEngine` facade behind a
  process-wide `OnceLock` keyed on `ORGANIZATION_SEARCH_INDEX_PATH`
  (default `data/search-index`). Index writes are wired into
  `src/streaming.rs` — the single seam both the native and the FHIR
  controllers write through — after the DB write is durable and
  best-effort (a failed index write is logged at `ERROR`, never fails a
  committed request). `GET /search` keeps `?q=` and gains
  `fuzzy` / `phonetic`; `check-duplicates` now **blocks** on the index
  (≤ 200 candidates) instead of scanning 1000 rows, so a duplicate's
  reachability depends on similarity, not insertion order. Rebuild:
  `cargo loco task search_reindex` plus an automatic boot rebuild when
  the index is empty and the table is not
  (`ORGANIZATION_SEARCH_BOOT_REINDEX=0` opts out).
  *Verified:* `cargo fmt --check` + `clippy --all-targets -D warnings`
  clean; **127** DB-free tests; **22** DB-gated tests green vs
  Postgres 18. The new DB-gated tests were mutation-checked (disabling
  indexing fails 6 of them), which is how the first version of the
  boot-rebuild test was caught passing vacuously — it raced the boot
  hook's own background rebuild, so the rebuild is now split into an
  awaitable `reindex_if_empty` (tested directly) and a thin `spawn_`
  wrapper the request suite switches off.

  Three decisions worth not re-litigating:
  - **The index is a candidate generator, never a source of truth.**
    Every hit is resolved against Postgres and soft-deleted rows do not
    resolve, so a stale index degrades (a missing hit) rather than
    corrupts (it can never resurrect or leak a deleted record).
  - **A broken index is `503`, not an empty result.** Especially on
    `check-duplicates`: answering "no duplicates" from an unopenable
    index would let a caller create a duplicate believing it had been
    checked.
  - **The `ILIKE` search and its `escape_like` guard were deleted, not
    left dormant.** The crate now issues no `LIKE` query, and an
    escaper with no caller invites a future caller to assume it is
    still wired in. care-pathway / case keep theirs — they still use
    `ILIKE`.

  Not done here: the FHIR `GET /fhir/Organization` search is a
  structured multi-parameter filter over a capped scan, not a free-text
  query; moving it onto the index is a separate item.

- [x] **S-2 (L)** Tantivy in **care-pathway**. *(done 2026-08-01)*
  Organization's pattern transferred whole — index module, streaming
  seam, reindex task + boot rebuild, blocked duplicate detection — with
  the field set changed to what a pathway *is*: **condition codes**
  (indexed as `system:code`) and interventions alongside the titles, so a
  search for `I63` or `thrombolysis` finds the pathway an `ILIKE` over
  `name` never could. Search keeps the pagination added in PG-1, with the
  total now coming from Tantivy's `Count` collector.
  *Verified:* 46 DB-gated green vs Postgres 18; fmt + clippy clean.

  Worth knowing for S-3/S-4: care-pathway carries the **IEC 62304 SOUP
  gate**, so adding `tantivy` failed `cargo test` until it was annotated
  in `compliance/soup.tsv`. That is the compliance machinery working, not
  an obstacle — case carries the same gate.
- [x] **S-3 (L)** Tantivy in **case** (as S-1). Depends: S-1. *(done 2026-08-02)*
  Care-pathway's pattern transferred whole again, field set changed to
  what a case *is*: **subjects** (the opaque involved-party ids) is the
  defining attribute, made searchable alongside agency name, case
  number/agency id (exact-match), case type/status (exact-match), and
  every identifier scheme. `check-duplicates` moves from a capped
  1000-row scan to up to 200 index-blocked candidates (fuzzy title,
  exact identifier, phonetic title). `search`/`check-duplicates` return
  `503` (not silent-empty) when the index is unavailable; every hit
  still passes the existing record-level ABAC concealment before
  reaching a caller.
  *Verified:* 32 DB-gated + 1 outbox-audit green vs Postgres 18; 193 lib
  tests; fmt + clippy clean.

  Two things worth recording that weren't specific to case. First, S-2's
  own commit never added `/data/` to care-pathway's `.gitignore`, so its
  Tantivy index binaries (16 files) landed in git — fixed here alongside
  case's own (correct, from the start) `.gitignore` entry. Second, S-2
  left care-pathway's `AGENTS.md` and `spec/index.md` still describing
  `ILIKE` search as current and Tantivy as deferred; case's own docs are
  updated in this PR, and care-pathway's `AGENTS.md` is corrected too
  (its `spec/index.md` T-6 entry is left for a future pass — bundling an
  unrelated crate's spec rewrite into this PR was judged more churn than
  the inconsistency warranted).
- [x] **S-4 (L)** Tantivy in **portfolio** — note the kind gate: index
  `kind` as a field and filter search/dedup within-kind. Depends: S-1.
  *(done 2026-08-02)*

  This task's own note needed a correction, not just an implementation:
  "filter search/dedup within-kind" read as a duplicate-detection gate,
  but both `project-portfolio-management-matcher/AGENTS.md` ("do not
  reintroduce a kind gate — two plans with different kind labels may
  still be the same identity") and this service's own `AGENTS.md`
  golden rule 5 ("dedup / check-duplicates / merge are not scoped by
  kind") say the opposite, in terms strong enough to call them
  deliberate, tested invariants rather than oversights. Implemented as
  the reading that satisfies both: `kind` is indexed and available as
  an opt-in **search** filter (`GET /plans/search?kind=project`), and
  `SearchEngine::candidates` (the `check-duplicates` blocking query)
  never applies it — a `Program`-labelled stored plan still blocks
  against a `Project`-labelled query. `search::tests::candidates_ignore_kind`
  plus a DB-gated end-to-end test pin the distinction so a future edit
  can't quietly collapse it back into a gate.

  Otherwise the care-pathway/case pattern (S-2/S-3) transferred whole:
  index module, streaming seam, reindex task + boot rebuild, `check-
  duplicates` moved from a capped 1000-row scan to up to 200
  index-blocked candidates. The field set is what a plan *is trying to
  achieve* — goal titles are now searchable, alongside tags, the owner
  org, and every identifier scheme. No SOUP register change (portfolio
  carries none — lower personal-data sensitivity than case/care-pathway,
  per P-4). `.gitignore` got its `/data/` entry from the start this
  time.
  *Verified:* 38 DB-gated + 1 enforcement + 1 outbox-audit green vs
  Postgres 18; 199 lib tests; fmt + clippy clean.

  **S-1..S-4 are now complete — every entity registry indexes via
  Tantivy.**

- [x] **P-1 (M)** Privacy module in **organization**. *(done 2026-08-01)*
  `src/privacy.rs`: `mask_organization` + `export_organization`, the
  endpoints `GET /{pid}/masked` and `GET /{pid}/export`, and the ABAC
  **`mask` obligation** wired into `GET /{pid}` and the export via new
  `auth::authorize_record` + `auth::organization_resource_attrs`
  (`resource.jurisdiction`, `resource.has_fiscal_id`).
  *Verified:* fmt + clippy clean; 136 DB-free tests; 23 DB-gated green
  vs Postgres 18, including a dedicated `tests/masking.rs` binary —
  mutation-checked (dropping the obligation branch fails it).

  What organization masks is **not** what person masks, and the
  difference is the point: most of an organization record is published
  fact. Redacted are `telephone`, `email` (routinely a named
  individual's line or inbox), the address's `street_address` (for a
  sole trader that is a home address, and there is no `is_sole_trader`
  flag to key on, so the street line goes for every record while
  locality / postcode / country stay), and `TaxId` / `Vat` values.
  **Not** redacted: LEI / DUNS / ROR / ISNI / Wikidata, the names,
  `url`, `jurisdiction` — masking those would break the lookups a
  registry exists for.

  **Consent is refused, not deferred.** The shared model is a *data
  subject* granting a purpose; an organization is not one, and the
  natural persons behind it are the person service's to record. A
  second, unauthoritative home for consent is worse than none. Stated
  in the crate spec §2/§13 so the next pass does not "finish" it.
- [x] **P-2 (M)** Privacy in **care-pathway** (as P-1; clinical data —
  mind `compliance-for-healthcare.md`). Depends: P-1. *(done 2026-08-02)*

  The interesting finding was that "as P-1" couldn't mean "copy
  organization's field list" — a `CarePathway` is a **template** and
  names no patient, so it carries none of organization's
  fiscal/contact-info fields to redact. What it does carry is
  institutional (`provider_name`/`provider_id`), masked anyway for a
  cross-department reader; the truly patient-identifying fact
  (`pathway_instances.subject_ref`, a specific person's enrolment) lives
  entirely outside the masked entity and is called out as an explicit
  follow-up in spec §16 rather than silently left uncovered — the same
  honesty move P-1 made by refusing rather than deferring a consent
  model. `care_pathway_resource_attrs` adds a `sensitive_setting` flag
  (`mental_health`/`palliative`) grounded in
  `compliance-for-healthcare.md`'s special-category framing, verified
  end to end by a DB-gated test proving a policy keyed on it does *not*
  fire for an ordinary setting.

  Also fixed: care-pathway's own S-2 commit (S-3's note already flagged
  this) never updated `spec/index.md`/`AGENTS.md` off "Postgres ILIKE
  search" — corrected alongside this change since both files were
  already open for the privacy edit.
  *Verified:* 48 DB-gated (46 request-suite + 2 dedicated
  `tests/masking.rs`) + 1 enforcement + 1 outbox-audit green vs
  Postgres 18; 246 lib tests; fmt + clippy clean.
- [x] **P-3 (M)** Privacy in **case** — it already honours the `mask`
  obligation; add the masked-view + GDPR-export endpoints on top of the
  existing `mask_case`. Depends: P-1. *(done 2026-08-02)*

  The smallest of the three privacy tasks, by design — case had already
  done the harder part (`mask_case` + the ABAC `mask` obligation on
  `GET /{pid}`) when the record-level ABAC work landed earlier. Added
  `GET /{pid}/masked` and `GET /{pid}/export` (`export_case`, new,
  living beside `mask_case` in `controllers/cases.rs` rather than a
  separate `src/privacy.rs` module — matching how this crate already
  organised masking, unlike organization/care-pathway). The export
  finally wires up `disclosure::action::EXPORT`, an action constant
  that had sat declared-but-unused in `disclosure.rs`'s vocabulary
  (covered only by a distinctness test) since case's HIPAA §164.528
  read-auditing work.

  Real finding: the DB-gated obligation test could not share a binary
  with the pre-existing `tests/masking.rs` (the SEC-G2/G3 concealment
  proof). Both set the process-wide `policy()`/`require_auth()`/
  `compliance::audit_reads()` `OnceLock`s before booting the app; adding
  the new test to the existing file let whichever one's boot ran first
  silently win the policy for the whole process, so the second test's
  own env-var changes had no effect and it failed with a symptom (an
  export audit count of zero) that read as unrelated to the real cause.
  Moved to its own `tests/export_masking.rs` — the fix organization's
  and care-pathway's own `tests/masking.rs` comments already warned
  about, learned here the hard way instead of by re-reading them closely
  enough the first time.
  *Verified:* 34 DB-gated request-suite + 2 dedicated masking/export
  binaries + 2 enforcement + 1 outbox-audit green vs Postgres 18; 193
  lib tests; fmt + clippy clean.
- [x] **P-4 (M)** Privacy in **portfolio** (lower sensitivity; masking of
  owner/person refs). Depends: P-1. *(done 2026-08-02)*

  The thinnest of the four privacy modules, matching the task's own
  "lower sensitivity" framing: most of a `Plan` is operational content
  (name, code, goals, status, identifiers, tags, relationships,
  containment), not personal data. `lead_ref` — a `person:`/`worker:`
  ref to the plan lead, the one genuinely personal field — is dropped
  entirely rather than partially shown (a partial UUID has no
  "recognisable" value when the plan is already identified by name/code);
  `owner_org_id`/`owner_org_name` get the usual tail-preserving
  redaction.

  A real gap surfaced during this pass: `GET /api/plans/{pid}` had no
  ABAC check at all — not even the coarse kind organization/care-pathway/
  case already had before their own privacy work — despite
  `authorize_record` + `plan_resource_attrs` already existing and being
  wired into `PUT /{pid}` for PPM-3. Fixed as part of the mask-obligation
  wiring, not as a separate task, since the obligation has nowhere to
  attach without a read-side ABAC pass.

  Applied the lesson from case's P-3 pre-emptively: `tests/masking.rs`
  was written as its own test binary from the start, rather than
  discovering the `OnceLock`-sharing trap by hitting it.
  *Verified:* 40 DB-gated request-suite + 1 dedicated masking binary +
  1 enforcement + 1 outbox-audit green vs Postgres 18; 205 lib tests;
  fmt + clippy clean.

  **P-1..P-4 are now complete — every entity service carrying personal
  or institutional data has field masking wired to the ABAC `mask`
  obligation.**

- [x] **AU-1 (M)** Roll the case-only auth hardening to **person, worker,
  place, thing, event** (axum-style `src/api/rest/auth.rs`): key-rotation
  refresh loop (`ReloadableVerifier` + `spawn_key_refresh` — case
  `src/auth.rs` is the pattern), policy hot-reload
  (`ReloadablePolicy` + `spawn_policy_watcher`), and a per-service
  `tests/enforcement.rs` activation proof (case's is the template; each
  runs in its own test binary).
  *Verify:* per-service green gate; jwt-enforcement.md status updated.
  **Done 2026-08-01** across all five, with `jwt-enforcement.md` §Status
  rewritten.

  The rollout's finding, worth carrying into AU-2/AU-3: **every one of
  the five had snapshotted the verifier into request state** (worker
  twice over), so a rotated key set could reach the handlers but not the
  guard, or the reverse. The bug was not that rotation was missing — it
  was that adding rotation to a snapshot would have half-worked, which is
  harder to notice than not working at all.

  - [x] **person** *(done 2026-08-01)* — the axum-style reference. All
    three parts landed and verified: fmt + clippy clean, 301 lib tests,
    **40 DB-gated green** vs Postgres 18, and the new
    `tests/enforcement.rs` mutation-checked (forcing the flag off fails
    it).

    The part worth copying carefully: person kept the verifier as an
    `Arc<Verifier>` **snapshot** in `AppState`, *and* the enforcement
    middleware took its own copy. Two snapshots means a rotation could
    only ever update one of them, so the fix was to delete the field and
    have the guard and both extractors read one process-wide
    `ReloadableVerifier` per request. Any service still holding a
    verifier in its state has the same latent split.

  - [x] **worker** *(done 2026-08-01)* — the worst split of the five: the
    verifier was snapshotted in `AppState` **and** captured a second time
    by `apply_enforcement`, so a rotation could have updated one and not
    the other. Both are gone; `apply_enforcement(router, require_auth)`
    reads the holders per request. 33 DB-gated green, enforcement proof
    mutation-checked.

  - [x] **place** *(done 2026-08-01)* — `EnforcementState` now carries
    only the flag. This crate had **no HTTP test harness** (its `tests/`
    are library tests over pure functions), so the activation proof
    brings a minimal one that builds the production router; `serial_test`
    and `tower` joined its dev-dependencies for it. 3 DB-gated green.

  - [x] **thing, event** *(done 2026-08-01)* — both kept `require_auth`,
    the verifier **and** the policy in `AppState`. The verifier and policy
    moved to the holders; only the flag stays on the state, because
    turning enforcement on or off mid-flight is not something to do
    without a restart. Both gained place's in-test router for the proof.
    thing 197 lib + 3 DB-gated, event 152 lib + 7 DB-gated, all green.
- [x] **AU-2 (M)** Same for **organization, care-pathway, course,
  portfolio**. *(done 2026-08-01)* Their verifier and policy were
  **boot-only `OnceLock` snapshots**, so unlike the axum-style five there
  was no split to fix — there was simply no way for a rotation or a
  policy edit to reach a running process. Both are now reloadable
  holders read per request, with `spawn_key_refresh` +
  `spawn_policy_watcher` wired at boot.

  Course turned out to be **axum-style**, not loco-style, despite this
  task's grouping: its auth lives in `src/api/rest/auth.rs` and its state
  held the verifier, policy and flag exactly as thing's did. Same
  treatment, and the grouping in this file was simply wrong.

  Activation proofs: organization, course and portfolio gained a
  `tests/enforcement.rs` (own binary); care-pathway already had one.
  organization's `authorize_record` — added with the privacy layer —
  reads the same holder, so masking decisions follow a reloaded policy.

  *Verified:* fmt + clippy clean in all four; DB-gated green vs
  Postgres 18 — organization 24, care-pathway 44, course 15,
  portfolio 37.
- [~] **AU-3 (S)** link-graph auth completion. *(2026-08-01 — auth and
  `user_ip` done; OTLP deliberately not)*

  Done: the boot-time keys-over-HTTP fetch (there was **none** — the key
  set could only come from the environment), `spawn_key_refresh`, the
  reloadable verifier + policy holders with `spawn_policy_watcher`, and
  `user_ip` on governed audits. That last one was hard-coded `None`, so
  every governed `subject_of` read and write recorded who but never from
  where. The address is `X-Forwarded-For`'s first hop when present, else
  the `ConnectInfo` peer — behind a proxy the peer is the proxy on every
  row, and one address on every row looks like evidence while being
  none. 16 DB-gated green; the capture is pinned in
  `tests/concealment.rs`.

  **Not done — OTLP (spec T-22), and the premise was wrong.** The task
  assumed there was an OTLP pattern to adopt. There is not: person,
  worker and event carry an `src/observability/` module that builds an
  OTel `Resource` and then installs a plain JSON `tracing` subscriber
  with the exporter and the `tracing_opentelemetry` layer commented out
  behind `// TODO: Initialize OTLP exporter`; every other service has
  nothing. **No service in the family exports a span or a metric over
  OTLP.** Porting that to link-graph would have moved a stub, so it is
  left open and the shared capability baseline — which claimed
  "Observability (tracing + OpenTelemetry OTLP)" for every crate — is
  corrected to say what is actually there. Wiring a real exporter is
  its own task: which crate first, and what the collector story is in
  compose (DEP-1).

- [x] **PG-1 (L)** Pagination in the four newest loco services.
  *(done 2026-08-01 — all four services and all four front-ends; one
  sub-resource follow-up noted below.)*

  The convention is **headers, not an envelope**, written up in
  [`agents/share/restful.md`](agents/share/restful.md): `?limit=&offset=`
  in, `X-Total-Count` / `X-Limit` / `X-Offset` out, body unchanged. All
  four endpoints return a bare JSON array today and every front-end
  parses one, so an envelope would break every caller for a number most
  of them do not use. Defaults reproduce the old hard caps, `limit`
  clamps (500) rather than erroring, and an `offset` past 10 000 is a
  `400` — that one is a cheap DoS, not an unusual request.

  - [x] **organization** + its front-end. List and search paginate;
    search's total comes from Tantivy's `Count` collector rather than the
    page length. Front-end: `ApiClient.getPage()` reads the headers,
    `listPage()` wraps it, and the list route shows `shown / total`.
    *Verified:* 25 DB-gated green; `svelte-check` clean, 46 unit tests,
    build ok.

    Worth knowing before doing the other three: **`#[serde(flatten)]` on
    query parameters silently breaks typed fields.** A flattened struct
    deserializes from a string-keyed map, so `limit=2` arrives as the
    string `"2"` and fails to parse as `u64` — a `400` on a valid
    request. Declare the page fields inline instead.

  - [x] **care-pathway** *(2026-08-01)* — list + search paginate; totals
    are `COUNT(*)` over the same predicate. 45 DB-gated green.

  - [x] **case** *(2026-08-01)* — same, with one deliberate difference
    worth keeping: the total is the **collection's** match count, taken
    before the per-record concealment this service applies (§10). A
    caller-specific total would leak exactly what concealment hides —
    how many records that caller may not see — so `X-Total-Count`
    describes the query, and a caller may receive fewer rows than it
    suggests. 34 DB-gated green.

  - [x] **portfolio** *(2026-08-01)* — `GET /api/plans` and its search
    paginate, with the `?parent=` scope applied to the count as well as
    the page. 38 DB-gated green.

  - [x] **the three remaining front-ends** *(2026-08-01)* — care-pathway,
    case and portfolio each gained `ApiClient.getPage()` and a
    `listPage()`, matching organization's. `svelte-check` clean and
    suites green in all three (48 / 37 / 55 tests).

  - [ ] **Portfolio's operational sub-resource lists** still carry
    `LIST_CAP = 200`: `automations`, automation runs, the deadline
    queue, delegations, approvals. They are per-plan working lists
    rather than the entity collection, and paginating them wants the
    front-end screens that consume them (FE-4 territory), so they are
    left capped rather than half-paged.

## Phase 3 — Platform

- [x] **BUS-1 (L)** `FluvioSink` (feature `fluvio`) in **case** (the
  relay's `EventSink` seam exists; see `src/relay.rs`). Topic naming per
  event-bus.md §7 (`mxi.case.events`, partition by pid). Compose file
  with a Fluvio broker for the bus-gated test tier (`#[ignore]` +
  feature-gated test: enqueue → relay → topic → consumed).
  *Verify:* default build untouched (feature off); feature build green;
  bus-gated test compiles (runs only with broker). *(done 2026-08-03)*
  `fluvio = { version = "0.50", optional = true }` behind a `fluvio`
  feature; `FluvioSink` holds one `TopicProducerPool` per topic
  (`fluvio::Fluvio::connect_with_config` + `topic_producer`), `send`
  partitions on the record `pid` per §7. Verified the real 0.50 API
  usage against the actual compiler (not just web-search docs) under
  `--features fluvio` — `Fluvio::connect_with_config`,
  `FluvioConfig::new`, `Client::topic_producer`, and
  `TopicProducer::send(key, value)` all resolved on the first attempt.
  `spawn` selects `FluvioSink` over `LoggingSink` when
  `CASE_FLUVIO_ENDPOINT` is set; **an endpoint configured without the
  `fluvio` feature refuses to start the relay** (logged at `error`)
  rather than silently using `LoggingSink` — a fallback there would mark
  outbox rows `published_at` without ever reaching a real broker, the
  same silent-data-loss shape BLK-4's artifact-store "no fallback on an
  explicit backend choice" rule exists to prevent. The initial
  connection retries indefinitely (rather than falling back) for the
  same reason; once connected, a broker outage is absorbed by the
  existing at-least-once retry (an unpublished row just stays
  unpublished until the next successful drain). `compose.fluvio.yaml` +
  `Dockerfile.fluvio-cli` (a separate compose file from
  `compose.test.yaml` on purpose, so the Postgres-only DB-gated suite's
  tooling is untouched) provision a local SC+SPU broker, translating
  Fluvio's own documented Docker Compose layout to this repo's Podman
  conventions — **not** run by any automated stage in this repo; the
  `#![cfg(feature = "fluvio")]`-gated, `#[ignore]`d
  `tests/fluvio_relay.rs` is verified by compiling under the feature
  (confirming correct API usage), not by an actual broker round-trip,
  matching the precedent already set by person's `s3_round_trip_
  against_a_live_endpoint` (BLK-4) — neither test has ever been executed
  in this session, both are `#[ignore]`d with the run command documented
  inline. *Verified:* `cargo build`/`test --lib`/`clippy --all-targets -D
  warnings`/`fmt --check` clean under both default features (246 lib
  tests) and `--features fluvio` (also 246 — the feature adds no new
  DB-free unit tests, only the ignored live-broker one); `cargo deny
  check` shows the same pre-existing `rsa` advisory as before (confirmed
  the only `error[vulnerability]` block, not a new one from `fluvio`'s
  tree); the full DB-gated suite (11 bulk + 39 pre-existing, including
  the new `fluvio_relay.rs` binary compiling to 0 tests under default
  features) rerun against real Postgres, zero regressions.
- [x] **BUS-2 (L)** link-graph **Fluvio consumer** (spec T-6):
  per-topic consumers driving the existing `apply_event` seam, with the
  `processed_events` idempotency table (spec §10.3) and per-topic offset
  resume. Retire lazy verify-on-read for entities with a live topic
  (keep it for the rest). Depends: BUS-1. *(done 2026-08-03)* One
  consumer task per entity topic (`src/consumer.rs`,
  `entity_ref::EntityType::ALL`, 10 topics), behind this crate's own
  `fluvio` Cargo feature (off by default) and gated further by
  `LINK_GRAPH_FLUVIO_ENDPOINT` — unset ⇒ unchanged behaviour (lazy
  verify-on-read + reconciliation stay the integrity path); **set
  without the feature** ⇒ refuses to start (logged at `error`), the same
  no-silent-fallback shape BUS-1 established, rather than the read-model
  silently falling behind with no warning. New `processed_events` table
  (migration `m20260803_000001_processed_events`) backs a new
  `events::apply_event_idempotent`, which dedupes on the envelope
  `event_id` under at-least-once delivery — every real consumer call
  goes through it, not `apply_event` directly; an envelope with no
  `event_id` (v1's optional field) still applies unconditionally.
  Verified the real `fluvio` 0.50 consumer API
  (`Fluvio::connect_with_config`, `consumer_with_config`,
  `ConsumerConfigExtBuilder`, `Offset::beginning`,
  `OffsetManagementStrategy::Auto`) against the actual compiler under
  `--features fluvio` — compiled clean on the first attempt, same
  successful approach as BUS-1's producer side.
  **Deliberate design decision:** resume position is delegated to
  Fluvio's own **named-consumer offset management**
  (`offset_consumer("link-graph-<topic>")` +
  `OffsetManagementStrategy::Auto`, which the SC persists and resumes
  server-side across restarts) rather than reconstructed from
  `consumer_offsets.offset_val`. That column keeps writing exactly what
  `apply_event` already wrote before this task (the envelope's own
  per-`entity_pid` `seq`), now understood as a freshness/diagnostic
  value rather than a literal Fluvio partition byte offset — threading a
  real Fluvio-record offset through `apply_event` instead would have
  touched roughly two dozen existing test call sites for no behavioural
  gain, since resume does not depend on it. Idempotency
  (`processed_events`) is a second, independent layer needed regardless
  of which mechanism resumes a topic, since delivery is at-least-once
  either way. Full reasoning recorded in `src/consumer.rs`'s module docs
  and spec §10.3/§13 T-6, not just here.
  **"Retire lazy verify-on-read for entities with a live topic" is
  explicitly NOT implemented** — `LINK_GRAPH_LAZY_VERIFY` stays a single
  global flag (it was never per-entity), and only `case` has a real
  Fluvio producer today (BUS-1); turning it off globally now would leave
  every other entity's presence permanently unresolvable. Revisit once
  BUS-3 rolls `FluvioSink` out further — recorded as a deliberate
  scope-bounding decision, not a missed requirement.
  `compose.fluvio.yaml` + `Dockerfile.fluvio-cli` (copy-adapted from
  case-service's BUS-1 files, same drift-accepted posture as
  `compose.test.yaml` existing per crate) provision a local SC+SPU
  broker for opt-in manual runs — not run by any automated stage in this
  repo; `tests/fluvio_consumer.rs` is a `#![cfg(feature = "fluvio")]`
  -gated, `#[ignore]`d live round-trip (produce onto `mxi.person.events`
  via a raw Fluvio producer → the already-public `consumer::spawn` →
  poll for the edge to land) with its run command documented inline,
  verified by compiling under the feature, not by an actual execution —
  same posture as BLK-4's S3 test and BUS-1's own Fluvio relay test.
  *Verified:* `cargo build`/`test --lib` (48 tests)/`clippy
  --all-targets -D warnings`/`fmt --check` clean under both default
  features and `--features fluvio`; `cargo deny check` shows only the
  same pre-existing `rsa` advisory as every other crate in this family
  (confirmed the only `error[vulnerability]` block, not a new one from
  `fluvio`'s tree); the full DB-gated suite (19 pre-existing request
  tests + 3 new `tests/idempotency.rs` tests — redelivery doesn't
  duplicate, distinct event_ids both apply, a no-`event_id` envelope
  applies every time) against real Postgres, zero regressions.
- [x] **BUS-3 (M)** Roll `FluvioSink` to the remaining nine services;
  reconcile the five older crates' dormant `fluvio` Cargo deps (use or
  remove). Depends: BUS-1, BUS-2. *(done 2026-08-03)* Ran as nine
  parallel subagents (one per crate: person, worker, place, thing,
  event, course, organization, care-pathway,
  project-portfolio-management), each handed case's exact working BUS-1
  port as the reference, then independently re-verified every one
  myself before committing — build/clippy/fmt clean under both default
  features and `--features fluvio`, and the full DB-gated suite rerun
  against real Postgres for all nine, matching or exceeding each
  agent's reported counts. `<ENTITY>_FLUVIO_ENDPOINT` selects
  `FluvioSink` over `LoggingSink`; set without the `fluvio` feature
  compiled in, the relay refuses to start (logged `error`) rather than
  silently falling back — same posture as BUS-1. **The "reconcile
  dormant fluvio deps" sub-clause was moot**: verified at BUS-1 time
  that zero `Cargo.toml` in the repo mentioned `fluvio` before this
  work landed it — there was nothing dormant to reconcile.
  **Found and fixed a stale family-doc claim** while scoping course's
  slice: `agents/share/overview.md`'s capability matrix said course had
  no durable outbox ("in-memory events only"), but it demonstrably does
  — a real `course_outbox` table, a working `EventTransport::Outbox`
  switch, and `src/relay.rs` already wired into `app.rs`. Course was
  therefore a legitimate BUS-3 target; the family doc is corrected in
  the same reconciliation as this entry.
  **Verification incident, resolved:** two of the nine crates
  (organization, course) briefly showed `cargo build --features fluvio`
  failures ("does not contain this feature: fluvio" /
  "unresolved import … FluvioSink") when I ran my independent
  verification concurrently with that crate's own agent still mid-edit
  — a race, not a defect; both rebuilt clean on retry once the agent's
  edits had settled. Worker-service's DB-gated suite also showed a
  cluster of `db::audit::chain_tests::*` failures on one run
  (`left/right` hash-chain mismatches) — traced to **reused/stale test
  -database container state** from concurrent verification runs
  colliding on the same `mxi-worker-test-db` container, not a
  regression: a truly fresh, isolated container run passed all 33
  tests cleanly, corroborating worker's own agent's independent
  stash/pop A/B test (which found the *same* failures on the pre-BUS-3
  baseline, and worse — 4/11 failing there vs 1/11 with the change).
  Neither incident reflects a defect in the shipped code.
  *Verified (per crate, matching or superseding each agent's own
  report):* `cargo build`/`clippy --all-targets -D warnings`/`fmt
  --check` clean under default features and `--features fluvio`;
  `cargo test --lib` identical pass counts both ways (person 311,
  worker 303, place 205, thing 197, event 152, course 123, organization
  184, care-pathway n/a-unit-tests-are-request-level, portfolio 205);
  `cargo deny check` shows only each crate's own pre-existing `rsa`
  advisory, never a new one from `fluvio`'s dependency tree; the full
  DB-gated suite passes against real Postgres for all nine with zero
  regressions (person 21+20+1, worker 11+20+1+1, place 2+11, thing
  2+outbox, event unit+integration, course 2+12+1, organization 33,
  care-pathway 49, portfolio enforcement+masking+outbox).

- [x] **LNK-1 (M)** Envelope `data` field + `Linked`/`Unlinked` kinds in
  **person**. *(done 2026-07-14)* `EventKind` gained `Linked`/`Unlinked`
  (+ tokens); `Envelope` gained an additive `data: Option<Value>`
  (`skip_serializing_if` — the CRUD/merge wire shape stays byte-identical,
  pinned by `crud_envelope_omits_data_on_the_wire`) carrying the §4.2 edge
  detail, plus a `for_link` constructor. The links handlers now emit:
  `create_link` → `linked`, `delete_link` → `unlinked`, transport-aware —
  under `outbox` the edge mutation + its event commit in one transaction
  (the outbox guarantee), under `memory` the in-memory
  `PersonEvent::Linked`/`Unlinked` (lossy dev signal). *Verified:* the seam
  unit test `for_link_carries_edge_detail_data` (data matches the
  aggregator's `LinkedEvent`) + token/frozen-shape tests + a DB-gated
  `linked_event_is_enqueued_to_the_outbox`; person lib + clippy + fmt clean.
  **Worker mirror landed the same day** (identical change on worker's
  envelope + `WorkerEvent`; worker lib 198 pass + clippy + fmt clean), so
  both person and worker now emit `linked`/`unlinked`.
- [x] **LNK-2 (M)** **Worker** `same_identity` write-side. *(done 2026-07-14)*
  Mirrors person's (`entity_links` migration + `NULLS NOT DISTINCT` upsert
  key, SeaORM entity, `src/db/entity_links.rs` persistence, `validate_edge`
  accepting only `same_identity` **worker → person**, per-record
  `POST`/`GET`/`DELETE /api/workers/{id}/links` + the governed bulk
  `GET /api/workers/links` returning canonical `EdgeDetail`, both router
  surfaces, record-level authz + audit incl. a new `log_export`; depends on
  the shared `entity-ref` crate). Worker added to the aggregator's reconcile
  list (`app.rs` `["case","person","worker"]`) + seam test
  `bulk_response_deserializes_the_worker_same_identity_shape`. Symmetric
  double-assert is by design (aggregator canonicalises the pair). Event
  emission deferred (as on person). *Verified:* worker `cargo test --lib`
  (194 pass, 7 links) + clippy + fmt clean; aggregator lib tests (41 pass)
  + clippy clean. Follow-ups all landed since: worker `employed_by` (LNK-3),
  `linked`/`unlinked` events (LNK-1), and the matcher-partition guard test
  (`links_are_not_a_matcher_signal`, both person + worker, 2026-07-15).
- [x] **LNK-3 (M)** Affiliation edges. *(done 2026-07-14)* `works_at` /
  `member_of` on **person** (→ organization) and `employed_by` (with `role`)
  on **worker** (→ organization). Each `validate_edge` permit set extended
  from `same_identity`-only to include the affiliation kinds (person
  `{same_identity, works_at, member_of}`, worker `{same_identity,
  employed_by}`), relying on the shared `entity-ref` `EdgeKind::permits` for
  the endpoint check; same tables / endpoints / generic bulk pull unchanged.
  Accept/reject matrices unit-tested per crate (affiliation → non-org
  rejected; cross-originated kinds rejected). *Verified:* person + worker
  `cargo test --lib links` (9 each) + clippy clean. Follow-ups (shared with
  LNK-2): `linked`/`unlinked` events + matcher-partition guard test.
- [~] **LNK-4 (L)** Cross-service `same_identity` **matcher + review
  queue** (design §5.2, roadmap): a job comparing person↔worker records
  (reuse matcher components), emitting `matcher_suggested` edges
  (confidence < 1.0) into a review surface; operator confirm promotes to
  `operator`/1.0. **Spec round done 2026-07-15** — link-graph spec §16 OQ-9
  (the resolved design: cross-type `IdentityProbe` comparator reusing the
  matcher primitives; identifier+Soundex/birth-year blocking;
  aggregator-hosted job that POSTs `matcher_suggested` edges to person's
  links endpoint while the aggregator stays read-only-to-the-world;
  per-service review + idempotent promotion) + the §13 task chain T-29–T-33.
  **Coding still gated** on the OQ-9 open sub-questions (block key/threshold,
  review-surface home, aggregator-write posture, scale). Depends: LNK-1..3.

  **Done: spec round fully pinned, 2026-08-04.** Closed all four OQ-9
  sub-questions with concrete decisions grounded in existing repo
  precedent (link-graph spec §16 OQ-9 + §13 T-29–T-33 updated;
  T-29–T-33 code itself remains open, hence `[~]` not `[x]`): **(a)**
  block key = exact shared coded identifier else `Soundex(family)` +
  birth-year, auto-suggest/discard threshold = 0.7 (reusing
  `BatchDeduplicationRequest::threshold` / `IMPORT_REVIEW_THRESHOLD`'s
  existing value), no auto-merge tier — every candidate needs an
  operator confirm; **(b)** the review surface is person's own existing
  `review_queue` table/endpoints (no FK on `record_id_a`/`record_id_b`,
  the BLK-2 `provenance` column already carries `matcher_suggested`),
  not a new aggregator endpoint — a reviewing client resolves the
  worker-side summary via its own `GET /api/workers/{id}`; **(c)**
  aggregator-calls-person's-write is confirmed acceptable (read-only-
  to-the-world forbids the aggregator exposing its *own* write
  endpoint, not acting as a client of a peer's), authenticated the
  same SEC-B7 loopback/token way as the reconcile worker but under its
  own dedicated `LINK_GRAPH_SUGGEST_URL_PERSON` /
  `LINK_GRAPH_SUGGEST_TOKEN` (least-authority: a separate, narrower
  credential than `LINK_GRAPH_RECONCILE_TOKEN`); **(d)** rate/scale
  controls are `LINK_GRAPH_SUGGEST_SECS` (3600s interval, mirroring
  `LINK_GRAPH_RECONCILE_SECS`'s skip-first-tick pattern),
  `LINK_GRAPH_SUGGEST_MAX_CANDIDATES` (50, mirroring
  `BatchDeduplicationRequest::max_candidates`), and
  `LINK_GRAPH_SUGGEST_MAX_EDGES_PER_RUN` (200) — with blocking (a) as
  the explicit load-bearing claim that keeps the job sub-quadratic
  rather than O(n·m). T-29–T-33 remain queued as the next (large,
  separate) coding effort.

- [x] **BLK-1 (M)** Bulk I/O step 2a — **CSV** codec on person.
  *(codec + spec done 2026-07-15; worker/export wiring folded into and
  finished by BLK-2, 2026-08-02)* `src/bulk/csv.rs` flattens the person wire
  type per §5 (scalars → columns; primary name → dotted `name.*`; arrays →
  JSON-in-cell) and **round-trips losslessly** against JSONL
  (`decode(encode(p)) == p`); columns matched by header (reordered/extra
  tolerated); per-row `Err` on a malformed row (§7). Person's exact column
  set declared in spec §10.6; adds the `csv` crate. Unit-tested: fully-
  populated + sparse round-trip, reordered/extra columns, bad-JSON-cell
  per-row error, multi-row, header.
- [x] **BLK-2 (M)** Bulk I/O step 2b — CSV worker/export wiring +
  keyless-row → duplicate-detection → **review-queue** routing on person
  import. *(done 2026-08-02)* Two parts:
  1. **CSV end-to-end**: `BulkFormat` gains `Csv`; the import/export
     handlers accept `format: "jsonl" | "csv"`; `process_import_job` /
     `process_export_job` take a `format` param and dispatch to the
     matching codec; the `bg_pg` worker reads `job.format`; stored
     artifact filenames carry the matching extension
     (`jobs/{id}/input.{jsonl,csv}`, `jobs/{id}/export.{jsonl,csv}`).
  2. **Keyless routing**: a row with no strong identifier, no `tax_id`, and
     no explicit `id` of its own (`stable_key::is_keyless`, backed by a new
     `row_has_explicit_id` — `Person::id` defaults to a fresh UUID on
     parse, so the parsed record alone can't distinguish "no id given"
     from "an id was given") runs the same search-blocking + matcher
     duplicate detection `POST /check-duplicates` uses. A likely duplicate
     (`IMPORT_REVIEW_THRESHOLD = 0.7`, the interactive check's own bar)
     still **creates** the row — a bulk load must never silently withhold
     legitimate data — and inserts a pair into the stored `review_queue`
     via a new `provenance` column (migration
     `2026080200000001_review_queue_provenance`; `operator` backfilled for
     existing rows, `import` for these; excluded from the re-scan upsert's
     `DO UPDATE SET` so a re-scan never overwrites a pair's origin). No
     candidate clears the threshold ⇒ a plain create, same as a keyed row
     with no match.
  DB-gated tests: keyless-duplicate creates + queues for review (asserts
  the pair, its provenance, its detection method), CSV import creates a
  keyed row, CSV export round-trips; the existing JSONL import/export
  suite extended for the new signatures. *Verified:* `cargo test --lib`
  (304 pass) + `cargo test -- --ignored` against a real Postgres (all
  green, including a debugging detour: the first keyless-duplicate test
  run found zero candidates because the test's random family name exceeded
  Tantivy's default 40-byte token cutoff and was silently dropped at index
  time — a test-fixture bug, not a pipeline bug) + `clippy --all-targets -D
  warnings` clean + `cargo fmt --check` clean.
- [x] **BLK-3 (S)** Parquet **export** (feature-gated `parquet`;
  arrow/parquet deps only under the feature). Export-only per §12 lean.
  *(done 2026-08-02)* `format: "parquet"` on `POST /api/persons/export`.
  `BulkFormat::parse` recognises the token on every build (so a client's
  request is always understood the same way); `BulkFormat::is_export_only`
  refuses it for import at the handler regardless of build config; the
  encoder itself lives behind a new `parquet` Cargo feature (off by
  default, `arrow` + `parquet` 59.1.0, matched release line) — a binary
  built without it returns a clean `422` on a `format: "parquet"` export
  request rather than silently substituting JSONL. The person §10.6 CSV
  column set moved into a new shared `src/bulk/columns.rs` so CSV and
  Parquet render one declaration rather than two that could drift:
  `Scalar`/`Bool` columns become nullable Arrow `Utf8`/`Boolean` (a real
  null for an absent field, unlike CSV's ambiguous empty string); `Json`
  columns (the arrays/arrays-of-objects) become non-nullable `Utf8`
  carrying the same JSON text CSV puts in its cells. Also closed IEC
  62304 §5.3.3 SOUP annotations for the three new direct dependencies
  (`arrow`, `parquet`, and the dev-only `bytes` needed to read Parquet
  bytes back in tests, since `parquet::file::reader::ChunkReader` has no
  `std::io::Cursor` impl). *Verified:* `cargo build`/`test`/`clippy
  --all-targets -D warnings`/`fmt --check` all run **twice** — default
  features and `--features parquet` — plus a DB-gated Parquet export
  round-trip against a real Postgres (reads the encoded bytes back
  through `parquet`'s own Arrow reader). **Known gap, recorded rather
  than silent:** `scripts/ci-check.sh`'s `test`/`clippy` stages build
  this crate with its *default* features only, so the family CI does not
  exercise the `parquet` feature — a follow-up (dedicated CI matrix
  entry, mirroring how `cargo-fuzz`/`cargo-deny` already get their own
  opt-in stage) is deferred as out of scope for this Small task.
- [x] **BLK-4 (M)** S3-compatible `ArtifactStore` impl (config-driven
  switch local-fs vs S3; mirror the env-var conventions). Feature-gate
  the S3 SDK dep if heavy. *(done 2026-08-02)* Ported the care-pathway
  service's reference design (`agents/share/bulk-import-export.md` §12)
  to person: `ArtifactStore` (`src/bulk/store.rs`) became
  `#[async_trait]` and gained `S3ArtifactStore` behind this crate's own
  `s3` Cargo feature (off by default; `aws-config`/`aws-sdk-s3`/
  `aws-credential-types` 1.x, mirroring care-pathway's exact feature
  flags). `PERSON_BULK_ARTIFACT_BACKEND` selects `local` (default) or
  `s3`; an unknown value warns and falls back to local; `s3` without the
  feature is a clean error, never a silent local-storage fallback that
  would lose export data. S3 config:
  `PERSON_BULK_S3_{BUCKET(required),ENDPOINT,REGION(default
  us-east-1),FORCE_PATH_STYLE(default on)}`; credentials from the
  standard AWS chain, never a bespoke variable. `split_reference` refuses
  a reference naming a foreign bucket (IDOR guard); `presigned_get`
  clamps its TTL to `[1, 3600]` seconds. The async trait forced
  `AppState::new` (`src/api/rest/state.rs`) from a sync constructor to
  `pub async fn … -> crate::Result<Self>` — both call sites (`app.rs`'s
  `after_routes`, `tests/common/mod.rs`) were already async, so this was
  additive; `bulk_store` stays a boot-time-built, request-shared
  `Arc<dyn ArtifactStore>` field rather than reconstructed per call
  (care-pathway's own reference instead rebuilds the store per request —
  person keeps its existing cached-in-`AppState` design, which the async
  trait doesn't disturb). Test suite ported verbatim from care-pathway,
  `PERSON_`-prefixed: local round-trip, missing-artifact error,
  no-presigned-URL-for-local, `is_safe_key` unit tests, unsafe-key/outside
  -base rejection, without-the-feature error, unknown-backend fallback,
  default/local backend names, foreign-bucket rejection, path-style
  default, and an `#[ignore]`d live-`MinIO` round-trip. SOUP register
  updated for the three new direct dependencies. *Verified:* `cargo
  build`/`test`/`clippy --all-targets -D warnings`/`fmt --check` under
  default features, `--features s3`, `--features parquet`, and
  `--features s3,parquet`; `cargo deny check` shows only the
  pre-existing `rsa`/`jsonwebtoken`/loco-rs advisory (confirmed
  unrelated); the DB-gated suite (`scripts/ci-check.sh test-db`, 21 lib +
  20 integration + 1 enforcement tests) against real Postgres, which
  exercises the real boot path through the now-async `AppState::new`.
- [x] **BLK-5 (L)** Roll bulk I/O to **organization** (stable key:
  LEI → DUNS → pid) and **case** (agency-scoped case number → pid),
  declaring each §10 section in their specs. Person's `src/bulk/` is the
  reference; these services are case-style loco (simpler than person).
  Depends: BLK-1..2 (so the rolled version includes CSV + review routing).
  *(done 2026-08-03)* Ran as two parallel subagents (one per crate,
  reference paths + green gate handed to each), then independently
  re-verified before committing — build/clippy/fmt clean, and the full
  DB-gated suite rerun myself against real Postgres for both, matching
  each agent's reported counts exactly (organization: 184 lib + 33
  DB-gated incl. 8 new bulk tests; case: 246 lib + 11 new + 39
  pre-existing DB-gated, zero regressions). JSONL + CSV only for both (no
  Parquet/S3 — out of BLK-5's declared scope). **Organization**: LEI →
  DUNS → explicit `pid` → keyless; every written row goes through the
  existing `streaming::create_and_emit`/`update_and_emit` path so a
  bulk-imported row gets the same event/audit/index side effects as an
  interactive create. Bumped `limit_payload`'s SEC-M1 backstop from 2mb
  to 70mb in dev/test config so it doesn't 413 under the new 64mb
  application-level import cap. **Known, documented gap:** the per-row
  upsert is not SEC-B3 advisory-lock-protected — a locked-guard-
  transaction attempt (mirroring person's) deadlocked *every* import, not
  just concurrent ones, because `create_and_emit`/`update_and_emit` open
  their own transaction internally and aren't generic over
  `ConnectionTrait`, so the lock transaction plus that call needs two
  pooled connections against this crate's own `max_connections: 1` test
  config — closing it needs a `src/streaming.rs`-wide change, out of this
  task's scope, tracked in organization's own spec §10.7/§13. **Case**:
  stable key is the *pair* `(agency_id, case_number)` → explicit `pid` →
  keyless (case has no single scheme-scoped deterministic identifier the
  way organization has LEI/DUNS); a new `BulkCaseRow` wire envelope pairs
  `case_matcher::Case` (which carries no `pid` field) with a genuine
  `Option<Uuid>` — cleaner than person's raw-line `row_has_explicit_id`
  sniff, since there's no default-fabrication ambiguity to work around.
  Case had no `review_queue` table at all (unlike person/organization),
  so one was built fresh with `provenance` from day one. Case's bulk
  export reuses the existing inline `mask_case` redaction (case has no
  dedicated `src/privacy` module per the capability matrix, but the
  masking that exists is real and now covers the bulk path too, rather
  than the bulk path opening an unmasked side door). Both crates: export
  audit write gates job completion (SEC-B8) exactly as person's does.

## Phase 4 — Surfaces, deployment docs, tutorials, examples

- [x] **FE-1 (M)** Merge actions in the **organization, case, portfolio**
  front-ends (person/worker/place/thing/event/course have the pattern;
  API: `POST /merge`, `merges/recent`).
  *Verify:* `pnpm check` + `pnpm test` + `pnpm build` per app.

  **Done 2026-08-03.** All three: a `/merge` route (portfolio:
  `/plans/merge`, since its collection lives under `/plans/`) —
  surviving main pid + duplicate pid + optional reason, an optional
  side-by-side preview, a native `confirm()` before submitting
  (destructive — the duplicate is soft-deleted), and a recent-merges
  history table from `GET .../merges/recent`. Validation is a pure,
  unit-tested guard returning an i18n key (both ids required, must
  differ) rather than hardcoded English, since every other string on
  each page is locale-driven; each app got its full key set genuinely
  translated across all 13 locales (organization 27 keys, case 25,
  portfolio 24), each pinned by the existing per-app i18n parity test.

  **The three target services' wire shape differs from the six
  reference front-ends'** — confirmed by reading each Rust controller
  directly rather than assumed from person's pattern: `POST
  /api/{organizations,cases,plans}/merge` returns `{main_pid,
  duplicate_pid, main}` inline, with **no `merge_record` wrapper**
  (unlike person/thing's `MergeResponse`), so each page reads the
  survivor straight from that response and reads merge timestamps from
  the separate recent-merges history rather than from a merge-record
  id/`merged_at` that doesn't exist in this response shape. Each app's
  `ApiError` also has no `.code` field (only `status`), so error
  formatting uses `${err.status}: ${err.message}` rather than porting
  the reference front-ends' `.code`-based pattern verbatim.

  Portfolio's own `AGENTS.md` had a stale claim that its plan detail
  page already had a "merge" action — verified against the actual
  detail-page source that it didn't (an aspirational documentation
  line, not a duplicated feature); corrected in the same pass now that
  merge lives at its own route.

  Implemented as three parallel, independently-verified passes (one
  per front-end) since the three apps' targets don't share any state;
  each was spot-checked afterward (git diff scope confined to its own
  app directory, one merge page read in full against the actual
  API client) before committing as three separate commits, one per
  app. All three: `pnpm check` (svelte-check) 0 errors/0 warnings,
  `pnpm test` (vitest, including each app's i18n parity test) green,
  `pnpm build` succeeded with the new route present in the build
  output. `pnpm test:e2e` (Playwright) intentionally not run — it needs
  a live server; smoke-test assertions were added but not executed.
- [x] **FE-2 (M)** Links panel: person front-end (assert/list/withdraw
  `same_identity` + affiliations), case front-end (`subject_of`),
  worker front-end once LNK-2 lands.

  **Done 2026-08-03.** Greenfield UI (no sibling front-end had a links
  panel to copy from, unlike FE-1) — grounded in a direct sweep of the
  Rust `links.rs` sources across all three services before any UI was
  designed. Each front-end got a `LinksPanel.svelte` embedded on the
  record's detail route: list the record's active outbound edges,
  assert a new one (client-side validation mirroring the service's own
  `validate_edge` kind/target-type matrix — person `same_identity`→worker
  and `works_at`/`member_of`→organization; worker `same_identity`→person
  and `employed_by`→organization with `role` as job title; case's single
  kind `subject_of`→person, deliberately with no kind picker), and
  withdraw with a confirmation. Case's panel is written sensitivity-aware
  per the design doc's §10 governance note — "Subject of this case", not
  a generic "Links" widget, and a withdrawal prompt naming the person
  reference being retracted. All three: full i18n (24–32 keys × 13
  locales, genuinely translated, not copied/placeholder), new unit tests
  for the validation guards and repository methods, a stubbed e2e smoke
  assertion. `pnpm check`/`test`/`build` green in all three (person
  35/35, worker 36/36, case 60/60 tests).

  **Found and fixed a real backend bug before the UI could even work.**
  person's and worker's link endpoints (`POST`/`GET`/`DELETE
  /api/{persons,workers}/{pid}/links`) returned **bare JSON** while every
  other REST endpoint on both crates wraps in the uniform
  `{success,data,error}` envelope — their own front-end `ApiClient`s
  unwrap `.data`, so calling these endpoints as shipped would have
  silently decoded as `undefined` rather than erroring. Fixed on both
  crates (the bulk aggregator endpoint, `GET /api/{persons,workers}/links`,
  is deliberately left bare — it's consumed by link-graph's own HTTP
  client, which expects `{"edges": [...]}` unwrapped, and wrapping it
  would have broken that instead). New DB-gated integration tests on
  both crates exercise the full create/list/delete cycle through the
  real router and assert each response actually decodes as
  `ApiResponse<T>` — verified green against real Postgres, not just
  compiled. Case's own links endpoints were already correct (bare JSON
  is case's own documented, deliberate convention).

  **Found (independently confirmed by two of the three implementing
  agents) and fixed 2026-08-03, on the user's explicit direction after
  being asked whether to fix it now, investigate further, or continue
  the flattened task list.** `ApiClient.buildUrl` in all eleven
  `*-front-end-with-svelte` `src/lib/api/client.ts` files resolved a
  leading-slash repository path (e.g. `/api/persons/{id}/links`)
  against a base URL of `<origin>/api/proxy` via `new URL(path, base +
  "/")`. Per the URL spec, an **absolute-path reference** (one starting
  with `/`) replaces the base URL's entire path, discarding
  `/api/proxy` — confirmed by direct reproduction in Node before
  touching any file:
  `new URL("/api/persons/p1/links", "http://localhost:4173/api/proxy/")`
  → `http://localhost:4173/api/persons/p1/links`, **not**
  `.../api/proxy/api/persons/p1/links`. Since the only server route
  these apps define is the catch-all `/api/proxy/[...path]/+server.ts`,
  every browser API call in every affected app was silently requesting
  a path with no matching route — the BFF proxy (and the PASETO it
  injects) bypassed entirely. One corroborating trace already in the
  tree: `project-portfolio-management-front-end-with-svelte/src/lib/api/ppm.ts:1025`
  hardcodes a literal `/api/proxy/api/auditor/evidence-pack?format=csv`
  path rather than going through the shared client — someone had
  already hit this and worked around it in one call site without
  generalising the fix.

  Ten of the eleven apps (all but authentication-front-end-with-svelte,
  which has the identical `buildUrl` code but calls its own service
  directly with no BFF/proxy layer — inert there, not "not affected")
  share byte-identical `buildUrl` bodies. Fixed by stripping the
  leading slash so `path` resolves as a *relative* reference against
  the base (which keeps its trailing slash) — appending rather than
  replacing — re-verified in Node before applying to any file:
  `new URL("api/persons", "http://host/api/proxy/")` →
  `http://host/api/proxy/api/persons`. Confirmed safe against every
  existing repository test first: they all stub a bare-origin base URL
  (`http://test`, no path segment), for which the old and new behaviour
  are identical, so none needed updating. Regression tests pinning the
  fix (a base URL *with* a path segment must keep it) were added to the
  five apps FE-2 already touched directly (person, worker, case,
  organization, thing); the other six got the code + a corrected inline
  comment (the old comment described the bug's own rationale as if
  intentional — worth not reintroducing verbatim from a stale mental
  model). All eleven: `pnpm check`/`test` green; `pnpm build`
  spot-checked on two of the apps not otherwise built this session.
  Committed as eleven separate app-scoped commits.

  **Process note, caught by a stray `git status` rather than anything
  systematic:** committing the OPS-1 runbooks and this DEP-2 doc
  earlier the same day, `git add agents/share/index.md` (lowercase) on
  this case-insensitive-but-case-preserving filesystem silently failed
  to stage that file's modifications against its index entry tracked as
  `AGENTS/share/index.md` — a known trap, already in this repo's own
  memory notes, and still worth tripping over apparently. Two edits
  (the OPS-1 runbook links, the DEP-2 `configuration.md` link) sat
  unstaged through two "completed" commits until this pass's `git
  status` turned them up; fixed in a follow-up commit
  (`2678de99`) using the correctly-tracked case. Worth remembering:
  `git status` right after a mixed `git add` is not sufficient
  confirmation everything staged — check for a bare `M ` vs ` M` in the
  short-format output, or diff the actual commit against the intended
  file list.

- [x] **FE-3 (M)** Bulk import/export screen in the person front-end:
  upload JSONL, dry-run toggle, job status polling, error-report and
  export download links (BFF-mediated; no token in browser).

  **Done 2026-08-03, with one requirement descoped by explicit user
  decision.** Researched against the live Rust source before any UI
  was designed, which surfaced a real blocker: person-service has **no
  HTTP endpoint that serves artifact bytes**, on either the local or S3
  backend. `download_url`/`errors_url` in the job-status response are
  opaque store references (`file://…`, `s3://…`) — `presigned_get`
  exists on the S3 backend but is called from zero production code
  paths, confirmed by grep. Asked the user how to proceed (fix the
  backend too / build without working downloads / stop and document);
  told to build without working downloads. `/persons/bulk` therefore
  renders both artifact references as plain text with a note that
  they aren't yet downloadable, rather than a link that would either
  404 or, worse, silently do nothing. The actual gap — no
  `GET /api/persons/{export,import}/{id}/download|errors` endpoint —
  is recorded as spec `OQ-7` for whoever picks up the backend half
  later.

  Otherwise full scope: upload with a dry-run toggle (a multipart form
  field, not a query param — required extending `ApiClient` to pass a
  `FormData` body through untouched rather than JSON-serializing it,
  done as a ~10-line, backward-compatible addition with its own
  regression test), filtered export with a masking-profile selector
  (`full` correctly handled as a possible 401/403, not just assumed to
  succeed), and job-status polling for both that is supersede-safe (a
  second submit of the same kind retires the first poll loop rather
  than racing it) and treats a 404 mid-poll as "expired" rather than
  crashing — the service conflates TTL expiry and another actor's job
  behind the same 404, by design (SEC-B4), so the UI can't and doesn't
  try to distinguish them either. `include_soft_deleted` is omitted
  from the export form entirely: the endpoint accepts it (202) but the
  worker doesn't support it yet, so offering it would only ever produce
  a job that fails after acceptance. A recent-jobs table with
  client-side kind/status filters, since `GET .../bulk-jobs` takes only
  `limit` server-side. 69 i18n keys × 13 locales. `pnpm check`/`test`
  (50 tests, including new coverage for the repository methods, the
  multipart `ApiClient` path, and the pure job-status helpers)/`build`
  all green; spot-reviewed directly (git diff scope, a full read of the
  main page component) before committing, not just trusted on the
  implementing agent's report.

- [~] **FE-4 (M)** Duplicate review-queue screen (services exposing the
  review API; start with person). **Person done 2026-08-04**; the other
  services with a `review_queue` (worker, place, thing, organization) are
  the remaining fan-out. The person board existed already but was
  unspecified and untested — see that project's `spec/13-tasks.md` T-25
  for what the completion added: `?status=`/`?limit=` filters (there is
  no `offset`, and "all" is the *absence* of `status` because the
  endpoint answers `422 INVALID_STATUS` otherwise), a keyboard-reachable
  queue table + explicit `Confirm`/`Reject` buttons alongside the
  mouse-only drag-to-decide, an inline side-by-side comparison fetching
  both records with two parallel `GET /api/persons/{id}` calls plus the
  matcher's `score_breakdown`, and `provenance` surfaced on the cards.
  The load-bearing scope fact for the fan-out: **confirming does not
  merge** — the decision endpoint is a pure status change and no service
  links a confirmed item to a merge, so the UI must supply that path
  itself (person deep-links `/persons/merge?main=…&duplicate=…` in either
  survivor order, since a review item names an unordered pair).

> Note: the **test** database side of this is already done — every
> service crate carries a `compose.test.yaml` driven by
> `scripts/test-db.sh` (see DEP-0 below). DEP-1 is the *demo/dev* stack:
> services plus their databases, wired to each other.

- [x] **DEP-1 (M)** `examples/compose/`: podman-compose for (a) one
  service + postgres, (b) the full family (10 services + auth +
  link-graph + postgres), (c) the enforced variant (auth on, policies
  mounted, reconciliation configured). Compose is also what tutorials and
  the bus-gated tests build on.

  **Done 2026-08-03.** `examples/compose/{single-service,full-family,
  enforced}.yml` (+ `init/`, `policies/`, `README.md`) written and
  verified **for real**, not just `podman compose config`: every
  container actually brought up against a live Postgres and exercised
  over HTTP.
  - **single-service.yml** (case-service + its own Postgres,
    migrate-then-start): `up -d`, migration log showed all 13
    migrations applying, `GET /_health` → `200`, `POST /api/cases` →
    `422` (a real validation rejection against the migrated schema, not
    a missing-table `500`) — proves the pattern end-to-end, not just
    that it boots.
  - **full-family.yml** (all 12 services, one shared Postgres, twelve
    databases via `init/00-extensions.sh` + `init/10-databases.sh`):
    all 12 containers came up, all 12 health endpoints returned `200`
    (`/api/health` for the six person-style crates, `/_health` for the
    five loco-idiomatic registries + link-graph), and two real
    functional calls confirmed the stack does actual work, not just
    health-check theatre: `POST /api/persons` → `201` with a real
    persisted record, and authentication-service's
    `/.well-known/paseto-keys` → `200` with a real Ed25519 key.
  - **enforced.yml** (override on top of full-family.yml — turns on
    `<ENTITY>_REQUIRE_AUTH`, mounts `policies/default.json`, wires
    `<ENTITY>_PASETO_KEYS_URL` at authentication-service, configures
    link-graph's `LINK_GRAPH_RECONCILE_URL_{PERSON,CASE}`): merged
    stack came up clean, health stayed public (`200` on both crate
    shapes) while an unauthenticated `GET`/`POST /api/cases` now got
    `401` (ABAC genuinely active, not merely configured), and
    link-graph logged the exact documented refusal —
    `"refusing an unauthenticated remote reconcile source: set
    LINK_GRAPH_RECONCILE_TOKEN…"` — for both `person` and `case`,
    confirming SEC-B7's fail-closed behaviour fires as designed rather
    than silently no-op'ing. `LINK_GRAPH_RECONCILE_TOKEN` is left empty
    by design: completing it needs a real PASETO minted through
    authentication-service's live magic-link flow, which a static
    compose file cannot script — documented as the one manual step in
    both `enforced.yml`'s header comment and the README, not silently
    left unexplained.

  Two real bugs found and fixed along the way, beyond the four crates'
  Dockerfile/compose fixes already recorded below:
  1. **`init/10-databases.sh`**: an unquoted `CREATE DATABASE case;`
     is a Postgres syntax error — `case` is a reserved SQL keyword.
     Fixed by quoting every database name (`CREATE DATABASE "case";`);
     harmless for the other eleven, load-bearing for this one. Found
     by actually running the init script against a real container, not
     by inspection — the failure only surfaces at `initdb` time.
  2. **The `docker-compose` build-hang** (noted below for the four
     legacy crates) recurred identically for images this task built
     fresh via `podman compose build` — confirming it is a
     compose-provider issue with this repository's build context size,
     not specific to any one Dockerfile. Same workaround: `podman
     build -t <exact-compose-image-name> …` directly, then `podman
     compose up -d` with no `--build` (documented in both compose
     files' header comments and the README, not left as a trap for the
     next person to hit).

  **Prerequisite work done 2026-08-03 (leading up to the above):** scoping the "full family"
  variant found that only 4 of the 12 crates it needs (person, worker,
  event, course) had a production `Dockerfile` at all — the other 8
  (place, thing, organization, care-pathway, case,
  project-portfolio-management, authentication, link-graph) had none.
  Built and **fully verified end-to-end** (real build + real boot
  against Postgres + a `200` health response) all 8 missing Dockerfiles.
  Along the way:
  - Discovered person's own **existing** Dockerfile (the template the
    new 8 were modelled on before this was found) is itself broken
    today — its `docker-compose.yml` sets `build: context: .` (this
    crate's own directory), but its Cargo.toml depends on sibling
    crates by path (`person-matcher`, `integrity-mac`,
    `authentication-verifier`) that live outside that context. Every
    crate in the family has this shape (no root `Cargo.toml`, so no
    shared vendor/lock step — agents/share/rust-loco-stack.md), so the
    fix is structural: **every Containerfile in the family must build
    from the repository root**, `COPY`-ing in each sibling path
    dependency explicitly. The 8 new Dockerfiles use this pattern and
    are proven to work; person/worker/event/course's existing
    Dockerfiles are **not yet fixed** to match (their own follow-up).
  - Added `.containerignore` (+ `.dockerignore` symlink) at the
    repository root — without it, a repo-root build context tries to
    copy every crate's multi-gigabyte `target/` directory and reliably
    fails (`cannot allocate memory` copying extended attributes on one
    crate's `target/` alone, ~2GB).
  - Found and fixed a **family-wide production-boot bug**: 5 of the 8
    crates' `config/production.yaml` (organization, care-pathway,
    case, project-portfolio-management, authentication) had an
    unquoted `mailer.smtp.auth.{user,password}: {{ get_env(name="…",
    default="") }}` — an unset env var renders as YAML `null`, not
    `""`, and loco's `SmtpAuth` fields are `String` not
    `Option<String>`, so config parsing failed at boot with "invalid
    type: unit value, expected a string". **The same pattern exists in
    4 more crates outside today's scope** (contact-relationship-
    management, patient-flow, content-management-system,
    workforce-planning-management) — confirmed present, not fixed
    today.
  - Found the **reason nobody had caught the SMTP bug**: 6 of these 8
    crates' `.gitignore` excluded `config/production.yaml` entirely (a
    loco scaffold default, `**/config/production.yaml`, never
    customized away) — so the file existed only on whichever machine
    happened to create it, never reached git, and a fix to it (or a
    bug in it) could never propagate. The file holds no secret
    directly (every credential is `{{ get_env(...) }}`-templated), so
    there was no real reason to keep it untracked. Removed the
    ignore rule and committed the (now-fixed) files for organization,
    care-pathway, case, project-portfolio-management, authentication,
    link-graph — matching the already-tracked convention
    person/worker/place/thing/event/course used all along.
  - Also hit (and worked around, not a code defect): the local podman
    VM's default 6GB memory allocation OOM-killed concurrent release
    builds (LTO + `codegen-units=1` is memory-hungry at the final
    link/codegen step); bumped to 12GB and rebuilt sequentially where
    needed. Podman's default OCI image format silently drops the
    Dockerfile `HEALTHCHECK` instruction (`--format docker` is needed
    to bake it in) — noted in each Dockerfile; the compose-level
    `healthcheck:` DEP-1 will add works under either format.
  **Further prerequisite work done 2026-08-03 (still not DEP-1
  itself):** fixed person/worker/event/course's existing
  Dockerfile + `docker-compose.yml` to the same repo-root-context
  pattern, then found — only by actually **running** each built image,
  not by trusting a green `podman build` — that all four had **three
  further real boot bugs**, on top of the context mismatch:
  1. **No `config/` copy.** None of the four Dockerfiles copied
     `config/` into the runtime image at all; the loco binary crashed
     immediately with `Message("no configuration file found in folder:
     config")`. (These four crates' `config/production.yaml` were
     already git-tracked, unlike the six-of-eight gitignore gap found
     above — this bug is purely a missing `COPY`.)
  2. **`CMD` with no `start` subcommand.** All four `CMD`s were the
     bare binary (`["/app/person-service"]`, etc.); every one of these
     crates' `src/bin/main.rs` dispatches through `loco_rs::cli::main`,
     which needs an explicit `start` argument — a bare invocation just
     prints the CLI's own `--help` and exits `0`, a "successful"
     container that serves nothing and would have looked healthy to
     any check that only inspects the exit code.
  3. **No `LOCO_ENV`, and a dead `SERVER_PORT`.** None set `LOCO_ENV`,
     so a `production`-tagged image would have booted in loco's default
     `development` config. Separately, all four already set
     `SERVER_PORT=8080`, but loco's own `config/production.yaml` reads
     the env var `PORT` (`server.port: {{ get_env(name="PORT",
     default="8080") }}` — `8084` for course specifically), not
     `SERVER_PORT` (a same-named but unrelated field these crates' own
     `src/config/mod.rs` documents, for a different, non-loco code
     path) — so `SERVER_PORT` was silently inert and every one of these
     four would have bound to whatever loco's Tera default happened to
     be. Fixed by setting `LOCO_ENV=production` and `PORT=8080`
     explicitly (course's own default was `8084`, now overridden to
     match its siblings' `8080` convention and its own `EXPOSE`/
     `HEALTHCHECK`).

  Course's `docker-compose.yml` had a **fourth, distinct** context bug:
  it already built from one level up (`course/`, not its own
  directory), but never copied `integrity-mac` or
  `authentication-verifier` — both of which live *outside* `course/`
  entirely — so it was exactly as broken as the other three's
  `context: .`, just by a different sibling-dependency gap, and its
  matcher dependency turned out to be a plain crates.io registry
  version (`course-matcher = "0.6.1"`, no `path`), so the existing
  Dockerfile's `COPY course-matcher-rust-crate` was unnecessary and
  dropped. Also fixed in all four: the stale `/api/v1/health`
  `HEALTHCHECK`/compose-healthcheck path (the decommissioned
  API-versioning-in-the-URL scheme — see
  [`agents/share/api-versioning.md`](agents/share/api-versioning.md))
  → `/api/health`, confirmed as the real mounted path for all four by
  grepping their `src/api/rest/mod.rs` route tables rather than
  trusting the (mutually inconsistent) per-crate docs. All four
  verified end-to-end (build + boot against real Postgres + `200` on
  `/api/health`) after every fix, individually and all together.

  **Compose-stack verification done 2026-08-03:** each of the four
  crates' own `docker-compose.yml` (not just a bare `podman run`) now
  brings up cleanly with `podman compose up -d` and answers `200` on
  `/api/health`, catching two further bugs that only a real compose
  run (not a direct `podman build`/`run`) surfaces:
  - **All four `postgres:18-alpine` services mounted the wrong
    directory.** They used the pre-18 convention, a named volume at
    `/var/lib/postgresql/data`; postgres 18's image stores data at
    `/var/lib/postgresql/<major>/docker` and refuses to start with that
    old path mounted (`Error: in 18+, these Docker images are
    configured to store database data in a format which is compatible
    with "pg_ctlcluster"…` — the exact issue
    [`agents/share/postgresql.md`](agents/share/postgresql.md) already
    documents for `compose.test.yaml`, just not yet applied to these
    four dev compose files). Fixed by mounting the volume at
    `/var/lib/postgresql` (the parent directory) instead, matching
    every crate's own `compose.test.yaml`.
  - **`JWT_SECRET` was never set** in any of the four `environment:`
    blocks, and loco's `config/production.yaml` `auth.jwt.secret` has
    no Tera default — added `JWT_SECRET: ${JWT_SECRET:-dev-only-unused-jwt-secret}`
    with a comment noting it is unused by this crate's actual auth
    flow (offline PASETO verification, not JWT sessions —
    [`agents/share/jwt.md`](agents/share/jwt.md)).

  Also cleaned up (no behaviour change): dropped the obsolete
  `version: "3.8"` key docker-compose warned about on every invocation
  from person/worker/event's compose files (course's already lacked
  it). And a build-tooling quirk worth knowing, not a bug in these
  files: `podman compose up -d --build` (which shells out to the
  Homebrew `docker-compose` binary as its compose provider) reliably
  **hung indefinitely** building these repo-root-context images on this
  machine, with no visible build subprocess and 0% CPU — while the
  identical build via plain `podman build -f <crate>/Dockerfile -t
  <name> .` from the repo root completed in the same one-to-three
  minutes it always does. Pre-building the image under the exact name
  `podman compose` expects (`<compose-project>-<service>`, e.g.
  `worker-service-with-loco-worker-server`) and then running `podman
  compose up -d` (no `--build`) works reliably — compose just reuses
  the already-built image. Not investigated further (no reproduction
  outside this compose provider); worth a second look if it recurs
  when building `examples/compose/`.

  All of the above was prerequisite work; DEP-1 itself (the three
  `examples/compose/` files) is recorded above as done.
- [x] **DEP-2 (M)** `agents/share/configuration.md`: the complete env-var
  reference — every `<ENTITY>_*`, `LINK_GRAPH_*`, `AUTH_*` variable, its
  default, effect, and which doc governs it. Generated by sweeping
  `std::env::var` call sites; keep a per-service table.

  **Done 2026-08-03.** Swept every `std::env::var` call site plus every
  `config/*.yaml` Tera `get_env()` call across all twelve crates
  (grep-grounded, not doc-derived — several claims were spot-verified
  directly against source after the sweep, e.g. `EVENT_EVENT_TRANSPORT`'s
  doubled prefix, portfolio's MAC-only `PORTFOLIO_` exception, and
  authentication's `LOCO_ENV=production` fail-closed dev-seed refusal).
  Structure: §1 explains the **two independent config paths** in the six
  person-family crates (loco's own Tera yaml vs. their separate
  `Config::from_env()`) — a real trap, since both read `DATABASE_URL`/
  port-shaped vars under slightly different names for different
  surfaces; §2–§3 the two path's own variable tables; §4–§5 the
  family-wide `<P>_REQUIRE_AUTH`/PASETO/ABAC and event-bus patterns;
  §6 the prefix table with its two documented exceptions (event's
  doubled prefix, portfolio's MAC-prefix mismatch) called out so a
  future edit doesn't "fix" them incorrectly; §7 link-graph's
  reconcile/probe vars, including the uppercased `EntityType::as_str()`
  suffix set (`COURSEINSTANCE` with no underscore, `CARE_PATHWAY` with
  one); §8 authentication-service's unprefixed `TOKEN_*`/`AUTH_*`
  vocabulary; §9 the integrity-MAC var names, which are **constructed**
  by the shared crate's `KeyConfig` rather than literal, so invisible
  to a plain grep; §10 the per-capability additions (search-reindex,
  bulk/S3, read-audit, portfolio-only); §11–§12 security-relevant
  defaults and boot-fail conditions gathered in one place. Wired into
  `agents/share/index.md`'s table (not into every crate's root
  `AGENTS.md` `@`-include list, matching `jwt-enforcement.md`/
  `event-bus.md`'s existing precedent of index-only reference docs).
  Out of scope, noted honestly rather than silently: the four consumer
  apps (contact-relationship-management, content-management-system,
  patient-flow, workforce-planning-management) follow the same shape
  under their own prefixes but were not swept.
- [x] **OPS-1 (M)** Runbooks under `agents/share/runbooks/`: auth
  activation checklist (expand jwt-enforcement.md), key rotation,
  reconciliation-divergence response, event-bus outage/replay, bulk-job
  failure recovery. Each: symptoms → checks → actions → verification.

  **Partial (2026-07-27):**
  [`runbooks/integrity-activation.md`](agents/share/runbooks/integrity-activation.md)
  covers the integrity and audit controls — activation order and why it
  is an order, how to verify each step actually took effect, checkpoint
  storage, symptoms→checks→actions, and MAC-key rotation. Written because
  every control in that stack is **default-off**, so a deployment doing
  nothing gets none of it.

  **Done 2026-08-03** — the four remaining runbooks, each researched by
  an independent code sweep (not written from the design docs' prose
  alone) before being written, and each surfacing at least one real gap
  or bug the runbook has to work around rather than paper over:

  - [`runbooks/paseto-key-rotation.md`](agents/share/runbooks/paseto-key-rotation.md)
    — found and fixed a **false claim** in
    `authentication-service-with-loco/config/keys/README.md`: it said
    peers refetch the key set "at their next boot **or on the first
    `UnknownKid`**" — there is no such refetch-on-`UnknownKid` trigger
    anywhere in `authentication-verifier` or any peer's `auth.rs`. A
    peer only refreshes at boot or on its own `<ENTITY>_PASETO_KEYS_
    REFRESH_SECS` timer (default 3600s; `0` disables it; unset
    `<ENTITY>_PASETO_KEYS_URL` means never). The runbook's rotation
    sequence is reordered around this: publish the new key as
    *additional* first, wait out the slowest peer's refresh interval,
    **then** promote it to primary — the crate README's original
    2-step "promote + restart" sequence would 401 every peer that
    hadn't yet polled. Also documents a real observability gap: no
    endpoint or metric on any peer exposes which `kid`s it currently
    holds.
  - [`runbooks/reconciliation-divergence.md`](agents/share/runbooks/reconciliation-divergence.md)
    — documents two sharp edges in the one divergence gauge link-graph
    exports: it is **global across all configured entities** (person
    and case both write the same unlabelled metric, so one entity's
    convergence can mask another's ongoing divergence), and it is
    **only updated on a successful pass** (a failed pass leaves the
    gauge exactly where it was, so a genuine `0` and a "hasn't run
    since boot" `0` are indistinguishable — the log line is the only
    real signal). Confirms there is no endpoint, task, or admin route
    to force a pass, list last-run time, or see a pass/fail counter.
  - [`runbooks/event-bus-outage-replay.md`](agents/share/runbooks/event-bus-outage-replay.md)
    — leads with the honest headline: **there is no replay** anywhere
    in the family today (no CLI task, no admin endpoint, no documented
    SQL procedure), despite several design docs using the word for this
    rollout step. Documents three distinct ways an outbox row can be
    marked "published" with nothing ever reaching a real broker
    (`LoggingSink` misconfiguration, the relay not awaiting a broker
    ack, and retention purging published-but-undelivered-in-truth rows
    after `<ENTITY>_EVENT_RETENTION_DAYS`), and flags that the
    consumer-side lag metric is likely reading near-zero for at least
    one producer regardless of real lag, because that producer's
    envelope doesn't carry `occurred_at` on the wire.
  - [`runbooks/bulk-job-recovery.md`](agents/share/runbooks/bulk-job-recovery.md)
    — states plainly that a `bulk_jobs` row can sit in `running`
    forever if its worker process dies (no heartbeat, no staleness
    check, loco's own queue reaper is off by default family-wide), that
    `bulk_jobs` has no error-message column (the real failure reason
    lives only in the log line), and draws the line the design doc
    blurs between **stable-key upsert** (what makes re-submitting a
    fixed file safe) and the **job-level `Idempotency-Key` header**
    (what makes retrying a possibly-already-received HTTP request
    safe) — two different mechanisms solving two different problems,
    easy to conflate. Also restates organization/case's known
    advisory-lock gap (SEC-B3) as an operational instruction: serialize
    concurrent bulk imports against those two crates yourself.

  All four follow the existing runbook's symptoms → checks → actions
  shape and end with an explicit "what this runbook cannot help you
  do" section — several real gaps (no per-kid key visibility, no
  per-entity divergence metric, no replay tooling, no bulk-job
  status/kind filter despite the design doc describing one) surfaced
  by the research are documented as follow-up work rather than
  silently designed around.

- [x] **TUT-1 (S)** `tutorials/01-getting-started.md` — run one service +
  front-end (uses DEP-1a). Every command copy-pasteable and verified.

  **Done 2026-08-04.** `tutorials/01-getting-started.md`: builds +
  starts `examples/compose/single-service.yml` (case-service + its own
  Postgres) as two separate commands per the documented `up -d --build`
  hang, verifies `/_health`, creates a case via the exact body from
  `examples/api/case.http`, reads it back by pid, and reads the
  paginated list with its `X-Total-Count`/`X-Limit`/`X-Offset` headers
  — then runs `case-front-end-with-svelte`'s dev server against the
  same backend and confirms both the SPA shell and its own BFF proxy
  (`/api/proxy/api/cases`) serve the just-created case. Ends with a
  tear-down section (`down -v`) and names TUT-2..TUT-6 as not-yet-written.

  **Every command block actually run, twice, end to end**: cleared a
  `podman machine` disk-full condition first (unrelated leftover
  containers/images from other work had filled the VM's 100 GB disk to
  98% — `podman container prune -f` + `podman image prune -a -f` freed
  it back to 53 GB free; noted nowhere in the tutorial since it isn't
  part of the documented flow, just a one-time host fix), then ran
  `build` → `up -d` → health/create/get/list curls → `pnpm install` →
  `pnpm check` (0 errors/0 warnings) → `pnpm dev` → root-page and
  BFF-proxy curls → `down -v`, confirmed zero `mxi-example-*`/
  `compose-case-*` containers and zero stray `vite dev` processes
  remained, then repeated the whole sequence a second time from a fresh
  `up -d` (new pid, same shape) to make sure nothing in the first pass
  was a fluke. All JSON shown in the tutorial is real captured output,
  not invented.

  **Corrected against the task brief**: the health path is `/_health`
  (confirmed from the compose file's own healthcheck), not guessed.

  **Real defect found and worked around, not fixed**: the front-end's
  own `.env.example` names stale variables
  (`PUBLIC_API_BASE_URL`/`VITE_AUTH_FRONTEND_URL`) that
  `src/lib/server/config.ts` does not read — the code (and the
  front-end's own `README.md` table) actually reads `CASE_API_URL`/
  `AUTH_API_URL`. Using `.env.example` as shipped silently leaves the
  BFF proxy pointed at its `http://localhost:5150` fallback, which is
  not where this tutorial's compose stack publishes case-service
  (`8089`). The tutorial calls this out explicitly and uses the correct
  variable names; fixing `.env.example` itself is left alone as outside
  this task's scope (only `tutorials/` + `tasks.md` staged).

  **Not covered on purpose** (out of scope for TUT-1, called out in its
  own "what's next"): the `seed_examples` loco task (EX-4) needs a host
  route to the service's Postgres to run `cargo loco task
  seed_examples`, but `single-service.yml` deliberately publishes no
  Postgres port — so TUT-1 seeds its one record via a single `curl`
  POST instead, and leaves `seed_examples` for TUT-2/TUT-4 once they
  exist; authentication/ABAC (TUT-3); and the other nine services
  (`full-family.yml`, DEP-1b).
- [x] **TUT-2 (M)** `tutorials/02-identity-lifecycle.md` — create →
  409-duplicate → check-duplicates → match → merge → audit trail, curl +
  UI.

  **Done 2026-08-04.** Uses **person-service**, not case (the family's
  richest matching implementation, and the direct beneficiary of EX-1's
  fixture + EX-4's seed task + the person front-end's `/review` screen).
  Runs person-service + its Postgres directly (`scripts/test-db.sh up` +
  `cargo run -- db migrate` / `-- start`), not in Podman — TUT-1 already
  covered the container path, and a 3-minute rebuild per tutorial is
  wasted reader time. Seeds via `cargo run -- task seed_examples` (real
  50-person output captured), then walks `409` create →
  `check-duplicates` (no side effect, confirmed by a follow-up empty
  search) → `match` (Ren vs Kenji Nakamura scoring the fixture's
  documented 0.9426 "probable") → `POST /deduplicate` (all 5 documented
  pairs, all 5 documented scores) → `review-queue` decisions (reject the
  ambiguous Nakamura pair, confirm the clear Okonkwo pair) → `merge` via
  curl → the duplicate's `404` (not `active:false`) → the audit trail —
  then the same review → confirm → merge sequence again through
  `person-front-end-with-svelte`'s `/review` board, on a **different**
  pair (Halloran), verified via the front-end's own BFF proxy
  (`/api/proxy/...`) making the exact calls the UI's merge page makes.
  Every curl response shown is real captured output from this run.

  **Two real, live-verified findings, not assumptions:**

  1. **Seeding also skips the search index**, not just audit/events
     (the seed task's own doc comment only mentions the latter two).
     `create`/`check-duplicates`/`match` all block on Tantivy before
     matching, so a seeded pair is invisible to them until a no-op `PUT`
     re-indexes it (`update_person` does index; the seed task's
     model-layer insert does not). Confirmed live: `check-duplicates`
     against seeded person #1's own exact data returned
     `has_duplicates:false` before the `PUT`. The batch scan
     (`POST /deduplicate`) is unaffected — it walks `list_active()`
     directly.
  2. **Merge was completely broken** — a defect this task found and
     fixed rather than routed around, since there was no way to
     demonstrate TUT-2's core walkthrough otherwise (every pair hit it,
     unconditionally). See `PERSON-CONTACT-CASE` above for the full
     writeup; the one-line summary is that `merge`'s `"old"`-aliased
     name and `Replaces` link were written in a case its own CHECK
     constraints reject. Fixed in a separate commit from the tutorial.

  Also found and worked around, smaller: person-service's own
  `README.md` documents `cargo loco start` for local dev, but no
  `cargo-loco` shim is installed in this environment
  (`cargo loco --version` → "no such command: `loco`"); the README's own
  parenthetical alternative, `cargo run -- start` (and `-- task ...`,
  `-- db migrate`), is what the tutorial uses throughout. And the front-end's
  `.env.example` is stale in the same way TUT-1 found for case
  (`PUBLIC_API_BASE_URL` vs. the real `PERSON_API_URL`/`AUTH_API_URL`).
- [x] **TUT-3 (M)** `tutorials/03-authentication-abac.md` — magic link
  (console), session cookie, `POST /token`, protected call, 401/403
  matrix, write + hot-reload a policy, `mask` obligation demo.

  **Done 2026-08-04.** Pairs **authentication-service** with
  **case-service** (per `authorization-attributes.md` §9/§11/§12: case is
  the reference implementation for record-level ABAC, the `mask`
  obligation, and policy hot-reload). Runs both directly via
  `cargo run --` against two throwaway test Postgres instances
  (`TEST_DB_PORT=5434` for case-service's, alongside auth's default
  5432) — `cargo loco db migrate` still doesn't work in this environment
  (no `cargo-loco` shim, confirmed again), matching TUT-2's finding.
  Walks: signup → magic link retrieved straight from the dev console log
  (simpler here than EX-3's compose-only workaround, since a bare
  `cargo run -- start` already boots `environment: development` with no
  override needed — EX-3's throwaway-container trick was compensating
  for the shipped compose baking in `LOCO_ENV=production`) → verify →
  session cookie pair → `POST /api/auth/token` (session + CSRF → PASETO
  v4.public, decoded live to show the real claim shape) → restart
  case-service with `CASE_REQUIRE_AUTH=true` (confirmed read-once-at-boot,
  restart required) and the full 401/403 matrix (no token / blank-`attrs`
  / `access=write` / `access=admin`) minting each `attrs` combination via
  the `user_attributes` CLI task (the only surface that can bootstrap the
  first admin — the HTTP admin API is itself `access=admin`-gated) →
  write a policy file (a scratch copy, never the repo's own example) →
  hot-reload it twice, live-timed both times (~4-6 s and ~7-10 s
  observed, comfortably inside the nominal 15 s `POLICY_WATCH_SECS`
  ceiling — never the full worst case in this run, honestly reported as
  such rather than claimed as a guarantee) → side-by-side full vs.
  `mask`-obligation read on the same case for two callers differing only
  in `dept`.

  **Two real, live-verified findings, not assumptions**, both documented
  in place rather than routed around silently:

  1. `examples/policies/closed-case-write-deny.json` (EX-2's cookbook),
     loaded **exactly as shipped**, denies *every* non-admin write
     unconditionally — not just writes to a closed case. Live-verified:
     an `access=write` caller's `POST` 403s ("default deny") even
     against a case that doesn't exist yet, so `resource.status` can't
     be the reason. Cause: a **configured** policy file replaces the
     built-in default policy outright rather than layering on it, and
     this cookbook file's two rules (`access=admin` override,
     `resource.status=closed` deny) never grant a plain write at all —
     it's written to be **composed** with a base grant, not deployed
     standalone, which its own `README.md` entry doesn't say. This
     tutorial's copy adds the missing `access=write ⇒ allow write` rule
     as a third entry (deny-before-allow order, so the closed-case
     restriction still fires).
  2. `examples/api/case.http`'s own `PUT` example body
     (`"status": "in_progress"`, not marked curl-verified unlike its
     neighbours) is wrong: `case_matcher::CaseStatus` carries **no**
     `#[serde(rename_all)]`, so the wire form is the bare Rust variant
     name — `"Open"` / `"Closed"`, not `"open"`/`"closed"`. A lowercase
     `status` value 422s with a generic `{"error":"Bad Request"}` (the
     Axum extractor rejecting the body before the handler's validator
     ever runs) — confirmed live, both the failure and the fix. This is
     unrelated to the *separate*, intentionally-lowercase
     `resource.status` tokens `case_resource_attrs` derives for ABAC
     matching. Not fixed (out of scope — only `tutorials/` and
     `tasks.md` staged by this task); documented so the next reader
     doesn't lose time to it.

  Smaller, also live-confirmed: an unauthenticated/under-authorized call
  gets a **plain-text** body (`missing authorization header` /
  `default deny`), not the family's usual JSON error envelope; a token
  with no attributes assigned serializes with **no `attrs` key at all**
  in its PASETO payload (not `"attrs":{}`), matching
  `authorization-attributes.md` §3's "absent claim ⇒ empty map" exactly;
  and assigning attributes via the CLI task revokes every session for
  that user (SEC-A8), so each `attrs` change in the matrix walkthrough
  needed a fresh magic-link sign-in before the new token would carry it
  — expected per spec, but easy to trip over if you reuse a stale
  session cookie.

  The port collision (both crates default to 5150) is resolved via
  loco's own `config/development.local.yaml` overlay mechanism — already
  gitignored by case-service's own `.gitignore`, so it needed no
  cleanup from git, only removal from the working tree at teardown.
- [x] **TUT-4 (M)** `tutorials/04-cross-service-linking.md` —
  `subject_of` + `same_identity` writes → aggregator `neighbors` /
  `single-view` / `freshness`; break-and-reconcile demo (divergence
  metric → repair). Depends: DEP-1b.

  **Done 2026-08-04.** Full `examples/compose/full-family.yml` (twelve
  services, DEP-1b), live end to end: person + worker (one real human,
  `same_identity`) + case (`subject_of`, the highest-governance v1
  kind) created via curl, both edges written from their originating
  service (`POST /{id}/links`, confirming **worker's own
  `same_identity` write-side has landed** — resolving the open question
  in the task brief; `worker/worker-service-with-loco/src/api/rest/links.rs`
  is a full peer of person's, both DB-gated round-trip and outbox-emit
  tested), then the link-graph aggregator's `neighbors` / `single-view`
  / `health/freshness`, and finally a live break-and-reconcile pass
  (direct `psql` corruption of link-graph's own `edges` row — delete
  the real edge, inject a fabricated one — then periodic reconciliation
  repairing both in one pass).

  **A real, severe infrastructure finding, not a shortcut**: the
  documented `podman compose -f full-family.yml build` command (all
  twelve Dockerfiles building concurrently) took down the **entire
  podman VM**, not just the compose wrapper — `podman ps -a` and even
  `podman machine ssh ... uptime` stopped responding for 90+ seconds
  under the memory pressure of twelve simultaneous release Rust
  compiles on a `no-swap` 12 GB machine. `podman machine stop` (clean)
  + `podman machine start` recovered it; the reliable workaround is
  bypassing compose's build orchestration and building each image
  **sequentially** with plain `podman build` (11 images, case reused
  from an earlier run, ~27 minutes total, peak memory ~3.4 GB — the
  ceiling is concurrency, not any one image's size). Documented in the
  tutorial itself as the load-bearing warning for anyone hitting the
  same silent hang. (Side effect, disclosed rather than hidden: the VM
  restart also stopped one pre-existing, unrelated container —
  `fhir-mssql-db` — that predated this task; not restarted, since
  restarting other work outside this task's scope wasn't requested.)

  **A second real, empirically-surprising finding**: expecting
  `link_graph_reconciliation_divergence` to show `2` during the broken
  window, it read `0` on every single poll (≈1 s granularity) across
  two full corrupt→repair cycles — not a bug, but a live reproduction
  of `agents/share/runbooks/reconciliation-divergence.md`'s own
  documented "sharp edge": the gauge is **one unlabelled value shared
  by every configured entity's reconcile worker** (`person` and `case`
  here), and `person`'s own always-`0` pass (nothing on that side was
  corrupted) kept winning the last-write race against `case`'s
  diverging-then-repaired pass. The read-model itself (`/api/edges`,
  and the per-status `link_graph_edges` gauge) is what actually proved
  the repair — real observed latency 6.6 s and 9.3 s across the two
  runs, both inside the configured 10 s `LINK_GRAPH_RECONCILE_SECS`.
  Documented in place as a "don't trust the divergence gauge alone in a
  multi-entity deployment" finding, not routed around.

  Also confirmed live and documented: `full-family.yml` leaves every
  `<ENTITY>_EVENT_TRANSPORT` at `memory` and link-graph's own image has
  no `fluvio` feature compiled in, so **reconciliation is the only path
  an edge reaches the read-model in this compose stack** — `freshness`
  stays `{"topics":[],"as_of":null}` for the entire tutorial, even with
  real `verified` edges present, since `as_of` tracks bus-consumption
  freshness specifically (a different signal from reconciliation).
  `LINK_GRAPH_RECONCILE_TOKEN` only needs to be a non-empty placeholder
  string here (SEC-B7's loopback-or-token gate), not a real PASETO,
  because `PERSON_REQUIRE_AUTH`/`CASE_REQUIRE_AUTH` are both off (the
  default) and both services' bulk-links `authorize_bulk` short-circuits
  to `Ok(())` when their own flag is off — documented alongside why
  `examples/compose/enforced.yml` deliberately leaves that same variable
  empty. Lazy verify-on-read (`LINK_GRAPH_LAZY_VERIFY`) settled edge
  status to `verified` synchronously on first read, and — unplanned —
  caught the fabricated corruption edge as `dangling` (not
  `unverified`) on the very read that surfaced it, since its fake
  `to_ref` 404s against person-service.
- [x] **TUT-5 (S)** `tutorials/05-bulk-import-export.md` — fixtures
  import (dry-run, error report), idempotent re-import, masked vs full
  export (and the 403 on ungated full). Depends: EX-1.

  **Done 2026-08-04.** Depended on `COMPOSE-WORKER` landing first
  (previous commit) — without it every job submitted here would have
  accepted a `202` and sat in `queued` forever, exactly as EX-1 found.
  Rather than reusing the fixed compose stack directly, ran
  person-service **locally** via `cargo run -- start --server-and-worker`
  (the identical `StartMode::ServerAndWorker` the compose fix invokes,
  just without a container — and the pattern TUT-2/TUT-3 already
  established as what actually works in this environment, no
  `cargo-loco` shim installed).

  **All five asked-for behaviours live-verified** against
  `examples/data/persons.jsonl` (50 rows, 5 duplicate pairs; EX-1):
  - **Dry-run** committed nothing (`total: 0` after) — and surfaced a
    genuinely surprising, live-verified finding not asked for but
    documented in full: `rows_to_review` came back `0`, not `5`, on
    the dry run. Read `src/bulk/pipeline.rs`'s dry-run branch to
    understand why — dry-run never writes a row, so when it reaches
    the *second* half of a duplicate pair the *first* half (which
    exists only earlier in the same file) was never persisted for it
    to find; duplicate detection queries the database, not the
    in-flight batch. A dry run therefore cannot see duplicates that
    exist only **within** the file being imported, only ones already
    in the database beforehand. Not a bug, but an easy-to-miss limit
    on trusting a dry-run's `rows_to_review` as a preview.
  - **Real import**: `rows_created: 50, rows_to_review: 5` — the five
    pairs surfaced correctly this time (rows now commit progressively,
    so the second half of each pair finds the first), matching
    `match-search-merge.md`'s "still created, also queued" contract.
    ~15–18 s wall time (duplicate detection + Tantivy indexing per row).
  - **Error report**: `persons.jsonl` imports clean (EX-1 already
    proved this), so a small 4-row file with a blank required field, a
    future birth date, and malformed JSON was crafted fresh
    (`/tmp/persons-errors.jsonl`, not checked in — the tutorial embeds
    it inline) — `status: "completed_with_errors"`, `rows_created: 1,
    rows_errored: 3`, and the real `errors.csv` content captured
    verbatim, including a live detail worth knowing: a row failing one
    validator reports only that one error line, not every rule it
    happens to also violate.
  - **Idempotent re-import**: submitting the same 50-row file again
    gave `rows_upserted: 7, rows_created: 43, rows_to_review: 43` —
    exactly `examples/data/README.md`'s documented 7-keyed/43-keyless
    split. The keyed rows upserted in place (no growth); the 43
    keyless rows have no upsert handle, ran ordinary duplicate
    detection, found their own first-run copy (now genuinely present,
    the mirror image of the dry-run gap above), and were created again
    while being queued for review — review-queue count verified
    growing from 5 to 48 (`5 + 43`) live. "Idempotent" here means
    upsert-by-key is idempotent, not that the whole file replay is a
    no-op — worth being precise about in the write-up.
  - **Masked vs. full export, and the `403`**: with `PERSON_REQUIRE_AUTH`
    off, both profiles worked (masked SSN `***-**-4728` / passport
    `*****0108` vs. full's real `000-31-4728`). With auth turned on
    (a real authentication-service brought up on port 5151 alongside,
    same TUT-3 pattern including the `user_attributes` CLI task and
    the SEC-A8 session-revoke-on-attribute-change re-sign-in dance):
    full **401**s with no token (same as masked — the blanket guard
    gates on *any* valid token before the export-specific check runs);
    both **403** with a token carrying no `attrs` (export is not one
    of `DESTRUCTIVE_POST_SUFFIXES`, so the blanket guard alone denies
    a plain `write` under the default-deny-mutation policy); with
    `access=write`, masked **202**s but full still **403**s — the
    precise, live-verified punchline: `export_requires_elevation`
    demands `Action::Destructive` specifically, and `write` does not
    imply `destructive` (`authorization-attributes.md` §2's rule
    applied to a masking profile instead of an HTTP verb); with
    `access=admin`, full finally **202**s and completes
    (`rows_total: 94` — 50 + 1 + 43 from the steps above). Full 4-row
    matrix captured in the tutorial.

  Full teardown (`test-db.sh down` for both throwaway Postgres
  instances, both `cargo run` processes killed, scratch files removed);
  `podman ps -a` afterward shows only the pre-existing, unrelated
  `fhir-mssql-db` container from a different task, untouched. Also
  updated `examples/data/README.md`'s "cannot finish an import yet"
  warning (written before `COMPOSE-WORKER` landed) to point at the fix.
- [x] **TUT-6 (S)** `tutorials/06-event-bus.md` — outbox rows, relay,
  `/events/recent`; extend with Fluvio when BUS-1..3 land.

  **Done 2026-08-04.** Uses **case-service** — the durable-event-bus
  reference implementation and the one service whose Fluvio producer
  side is wired to a real deployment target (`overview.md`'s
  capability-matrix footnote 4). Ran locally via `cargo run -- start`
  with `CASE_EVENT_TRANSPORT=outbox CASE_EVENT_RELAY=true` against a
  throwaway `scripts/test-db.sh` Postgres; live-verified a create, an
  update, and a merge all the way through: the `event_outbox` row (full
  envelope payload, `psql` query against `mxi-case-test-db`), the relay
  draining it to the no-broker `LoggingSink` (real log line captured),
  and `GET /api/cases/events/recent` reflecting all five events
  newest-first with the correct `seq` ordering (a merge's own `Deleted`
  sorting ahead of the duplicate's `Created` because `seq` is assigned
  at envelope-build time in the handler, not at relay-publish time).

  **A genuinely new, live-verified finding, not an assumption**: the
  outbox relay is **not** a loco background worker — `src/app.rs`'s
  `after_routes` hook spawns it directly via `tokio::spawn`,
  unconditionally on every boot, and `crate::relay::spawn` itself is
  what no-ops when `CASE_EVENT_TRANSPORT`/`CASE_EVENT_RELAY` are off.
  Confirmed live: the server booted showing `modes: server` (not
  `server, worker`, unlike TUT-5's bulk-job-worker finding) and still
  logged `starting event-outbox relay` and later `relay: published
  outbox event` — so plain `cargo run -- start` is correct here, and
  `--server-and-worker` is neither needed nor wrong for this particular
  background task, only for loco's own `BackgroundQueue` workers (bulk
  import/export). Worth being precise about since TUT-5 just established
  the opposite pattern for a *different* background mechanism in a
  *different* service.

  **A correction to earlier tutorials' "no `cargo-loco` shim" claim,
  narrowed rather than overturned**: `cargo loco --version` and `cargo
  loco db migrate` both actually ran in this crate — not because a
  global `cargo-loco` plugin is now installed, but because
  `case/case-service-with-loco/.cargo/config.toml` (and
  authentication-service's) carries a repo-local `[alias] loco = "run
  --"`, present in the newer loco-scaffolded crates and absent from
  person/worker/place's. TUT-2/TUT-3/TUT-5's finding stands for the
  crates they used; this tutorial still writes every command as `cargo
  run -- …` throughout for consistency across crates that do and don't
  carry the alias.

  **The PUT status-casing defect TUT-3 found in `examples/api/case.http`
  reconfirmed and demonstrated correctly**: `"status":"in_progress"`
  (the shipped example) still 422s; `case_matcher::CaseStatus` has no
  `#[serde(rename_all)]`, so `"status":"Open"` is what a live `PUT`
  needs. Still not fixed here (only `tutorials/` and `tasks.md` staged
  by this task, matching the file's own scope note); this tutorial's
  own `PUT` example uses the correct casing rather than propagating the
  known-bad one.

  **Fluvio (§7) is documented, not run** — config vars
  (`CASE_FLUVIO_ENDPOINT` etc.), the crate's own opt-in
  `compose.fluvio.yaml` (a Stream Controller + SPU pair, separate from
  `compose.test.yaml`), and the two no-silent-fallback guardrails
  (`fluvio` feature required, indefinite reconnect retry) are covered
  from the source, with an explicit statement that no broker was stood
  up in this session — matching `compose.fluvio.yaml`'s own header
  comment and `event-bus.md` §8's honest scope note that this is true of
  every one of the ten `FluvioSink`-carrying services today, not a gap
  specific to this tutorial.

  Full teardown (`test-db.sh down` for the one throwaway Postgres,
  `cargo run` process killed, `/tmp/case-service.log` removed);
  `podman ps -a` afterward shows only the pre-existing, unrelated
  `fhir-mssql-db` container from a different task, untouched. This was
  the last of the six planned tutorials (TUT-1..TUT-6); only `LNK-4`
  (spec-first) remains in the flattened task order.

- [x] **EX-1 (S)** `examples/data/` — synthetic JSONL fixtures: ~50
  persons (with duplicate pairs for the dedup tutorial), ~20
  organizations, ~10 cases with subject links. No real PII; documented
  provenance header in each file.

  **Done 2026-08-04.** `examples/data/{persons,organizations,cases}.jsonl`
  (50 / 20 / 10 rows) + `case-subject-links.md` + `README.md`.
  - **Provenance** lives in one `README.md` rather than a per-file
    header, because a JSONL header line is a contradiction in terms —
    every line must parse as the entity, so a comment line would break
    the format the files exist to demonstrate. It states plainly that
    every value is invented and written by an AI coding assistant for
    these tutorials, not sampled from any dataset, and tabulates the
    reserved ranges used (`555-01xx`, Ofcom `+44 20 7946 0xxx`, RFC 2606
    `.example`, SSN area `000`, unissued LOU/GS1/DUNS prefixes).
  - **Check digits are real where the service checks them.** The
    organization service validates LEI (ISO 7064 MOD 97-10) and GLN (GS1
    mod-10) per SEC-M5, so those were computed rather than invented; a
    made-up LEI is a `422`.
  - **Subject links are a separate file, not a `Case` field.**
    `subject_of` is a cross-service edge written via
    `POST /api/cases/{pid}/links` after both records exist, and the pids
    are not knowable at import time — so `case-subject-links.md` gives
    the case-line → person-line mapping plus the real request shape,
    and deliberately invents no UUIDs. `cases.jsonl`'s `subjects[]`
    holds agency-local labels, not `person:` URNs.

  **Verified against the services' own code, twice over.** Temporary
  harnesses (not checked in — these are data, and a permanent test in
  three service crates would be a three-part spec change in each):
  - *Offline, all 80 rows*: each file's real `bulk::jsonl::parse_line`,
    its `validation` module (the per-row validators the import pipeline
    calls), and its `bulk::stable_key` resolver; plus the real person
    `ProbabilisticScorer` over all 1 225 person pairs. Result: 0 parse
    or validation failures; 7/15/8 stable-keyed and 43/5/2 keyless rows;
    no stable-key collision; no duplicate identifier `(system, value)`
    (which the person schema's `UNIQUE` constraint would turn into a DB
    error, not an upsert).
  - *Live, against `scripts/test-db.sh` Postgres*: 17 representative
    persons via `POST /api/persons` → `201` (minimal rows, all three
    fully-populated rows, each pair's first half), then each pair's
    second half → `409 DUPLICATE_DETECTED`; all 20 organizations and all
    10 cases created and read back field-by-field; the `subject_of` link
    created + listed, and `subject_of → organization:` refused `422`.
    The five pairs score 0.9934 / 0.9908 / 0.9705 / **0.9426** / 0.9995
    — four *certain* and one deliberately *probable* (Ren vs Kenji
    Nakamura), so the dedup tutorial has a pair that genuinely warrants
    an operator decision. No unintended pair scores above 0.70. The
    live pass matters: it proved the Tantivy blocking query (family-name
    fuzzy, max edit distance 2) actually retrieves each pair, which no
    offline scoring check can show.

  **Defect found, and worked around rather than fixed here:**
  person-service cannot persist `telecom` at all, nor `use_type` on a
  name or identifier. `src/db/repositories.rs:501,555,596` write them as
  `format!("{u:?}")` → `"Phone"` / `"Official"`, while the CHECK
  constraints in
  `migrations/2024122800000003_create_patient_related_tables/up.sql:7,23,59,61`
  require lowercase. Any such person is a `500 DATABASE_ERROR` — the
  validators pass it, so only a real database catches it. The sibling
  2026 tables are unaffected (`enum_to_tag` → serde, correct case, no
  CHECK), so emergency-contact telecom and address `use_type` do work
  and are used. The fixtures therefore omit the broken fields; fixing
  the service is a three-part change in person-service and is **not**
  done — see PERSON-CONTACT-CASE below.

  **Second defect found (blocks TUT-5):** the DEP-1 compose stack cannot
  complete a bulk import. All three documented `curl -F file=@…/import`
  commands were run against a live `full-family.yml` stack and all three
  were accepted with a job id, but every job stays `queued`. The
  containers run `CMD [".../<svc>-service", "start"]` — loco's
  server-only mode — while `BulkJobWorker` is registered in
  `connect_workers`, which only runs under `start --server-and-worker`.
  Tracked as COMPOSE-WORKER below. The fixtures themselves are
  unaffected: their content was proven through the synchronous create
  endpoints and the real per-row import validators.

- [x] **COMPOSE-WORKER (S) 🟠** `examples/compose/*.yml` start every
  service server-only, so no loco background worker runs and **no bulk
  import or export job ever leaves `queued`**. Both `full-family.yml`
  and `single-service.yml` inherit the Dockerfile's `CMD [..., "start"]`.
  Fix: `command: ["start", "--server-and-worker"]` (or a sidecar worker
  container per service, which keeps the web tier's restart behaviour
  independent), then re-verify by importing `examples/data/persons.jsonl`
  and watching the job reach `completed`. TUT-5 cannot be written
  truthfully until this lands. Found by EX-1's live verification,
  2026-08-04.

  **Done 2026-08-04.** Added `command: [...]` to every `*-service:`
  block in `examples/compose/full-family.yml` (all 12 — the ten entity
  registries + authentication + link-graph) and the one service block
  in `examples/compose/single-service.yml` (case-service). Landed as
  the full explicit argv, not the brief's literal
  `["start", "--server-and-worker"]`: none of these Dockerfiles set an
  `ENTRYPOINT` (confirmed by grep — only `CMD`), so compose's
  `command:` replaces the entire argv, and a bare `["start", ...]`
  would try to `exec("start")` as a binary path and fail. Each block
  now reads `["/app/<bin>", "start", "--server-and-worker"]`, matching
  the exact binary name each crate's own Dockerfile `CMD` already
  names (`authentication-service-cli` for authentication, the crate
  name for the other eleven). `--server-and-worker` confirmed against
  the installed `loco-rs` 0.13/0.14 sources
  (`~/.cargo/registry/.../loco-rs-*/src/cli.rs`) rather than trusting
  the brief's spelling. `*-migrate` one-shot containers were left
  untouched, as instructed.

  **Verified live**, not just read — chosen vehicle: `single-service.yml`
  (case-service) rather than a `full-family.yml` rebuild, since one
  service already proves the fix and case-service's image was already
  built and cached locally from an earlier session (`docker.io/library/
  mxi-family-case-service`, ~3h old), so no rebuild — and no repeat of
  TUT-4's parallel-build VM crash — was needed at all; compose reused
  the existing image under the new `command:`. `up -d` → the
  container's own boot banner now reads `modes: server, worker` (was
  `modes: server`) and logs `worker is online` /
  `Starting background job processing`, confirming
  `StartMode::ServerAndWorker` actually took effect. Submitted a real
  `examples/data/cases.jsonl` import (`POST /api/cases/import` →
  `{"job_id":"57a1f8d0-..."}`); polled `GET /api/cases/import/{id}`
  and it was already `"status":"completed"` on the very first poll
  (`rows_total:10, rows_created:10, rows_upserted:0, rows_to_review:0,
  rows_errored:0`) — fast enough that no intermediate `running` state
  was ever observed. Cross-checked against the data itself, not just
  the job row: `GET /api/cases?limit=1` returned
  `X-Total-Count: 10`, confirming the rows are real, not a job-status
  fiction. `down -v` torn down cleanly; `podman ps -a` afterward shows
  no container from this work (one unrelated, pre-existing
  `fhir-mssql-db` container from a different task was left untouched,
  as it predates and is out of scope for this one).

  **Not re-verified**: `full-family.yml`'s twelve-service form (the
  single-service fix is byte-for-byte the same change, twelve times
  over, mechanically applied — full-family's own live exercise happens
  incidentally the next time a full-family tutorial runs, e.g. a future
  TUT-6). No podman-machine issue was hit this run; the machine was
  left running and healthy throughout (no restart needed, unlike TUT-4).

- [x] **PERSON-CONTACT-CASE (S) 🟠** person-service silently cannot store
  contact points. `repositories.rs` persists `ContactPointSystem`,
  `NameUse` and `IdentifierUse` via `format!("{:?}")` (PascalCase) into
  columns whose CHECK constraints only admit lowercase, so **every**
  person with a `telecom` entry fails with `500 DATABASE_ERROR` rather
  than a `422`. Contacts are advertised as a baseline capability
  (`agents/share/overview.md`), and no test covers the path — the
  existing integration tests all post persons without `telecom`. Fix:
  use the existing `enum_to_tag` helper (serde, already correct) on both
  the write and read sides, or relax the constraints; either way add a
  round-trip test that posts a person **with** telecom and reads it
  back, and restore the contact fields to `examples/data/persons.jsonl`.
  Found by EX-1's live fixture verification, 2026-08-04.

  **Done 2026-08-04**, forced by TUT-2's live verification: merge
  unconditionally sets an `"old"`-aliased name and a `Replaces` link, so
  this defect blocked *every* merge, not just fixtures carrying
  `telecom` — there was no way to write TUT-2's merge step without
  fixing it. `src/db/repositories.rs` write side switched from
  `format!("{:?}")` to the pre-existing `enum_to_tag` helper (already
  correct for `person_addresses`/emergency-contact tables) for
  `NameUse`/`IdentifierUse`/`ContactPointSystem`/`ContactPointUse`/
  `LinkType`/`IdentifierType`; the read side switched from hand-rolled
  `PascalCase` match arms to `tag_to_enum`. `NameUse`, `LinkType`,
  `ContactPointSystem`, `ContactPointUse`, `IdentifierUse` gained
  `PartialEq, Eq` for the new tests' assertions. Two new DB-gated tests
  in `tests/api_integration_test.rs`:
  `test_merge_two_persons_round_trips_alias_name_and_replaces_link`
  (merges two real persons, re-fetches the survivor, asserts the alias
  name's `use_type` and the link's `link_type` both round-trip) and
  `test_create_person_with_telecom_and_identifier_use_type_round_trips`
  (posts a person with `telecom` + an identifier `use_type` set, reads
  it back). Both green, alongside the full existing suite (23 request
  tests, up from 21) and `cargo fmt --check` / `cargo clippy
  --all-targets -- -D warnings` clean. **Not done**: restoring the
  contact fields to `examples/data/persons.jsonl` — that fixture has
  its own extensive, already-completed live-verification pass (EX-1),
  and re-verifying it after an edit is out of scope for a fix found
  while writing a tutorial. Also found, writing the new telecom test:
  several *existing* tests (and `examples/api/person.http`) write
  `"use": "official"` on a name, which silently no-ops — the wire field
  is `use_type`, not `use`, and always was; left alone as pre-existing
  and not asserted on by those tests. **Residual, narrower, not fixed:**
  `LinkType::ReplacedBy`'s `#[serde(rename_all = "lowercase")]` produces
  `"replacedby"`, not the CHECK's `'replaced_by'` — nothing in this
  crate constructs that variant today. Landed as its own commit
  (`person/person-service-with-loco` code + tests + `CHANGELOG.md`),
  separate from the TUT-2 tutorial commit.
- [x] **EX-2 (S)** `examples/policies/` — ABAC cookbook: dept-scoped
  read-deny, closed-case write-deny (`resource.status`), after-hours deny
  (`env.after_hours`), ownership (`$sub`), masked-read obligation,
  machine-peer grant (`svc`). Each policy JSON + a three-line README
  entry; all validated by a small test in the verifier crate that parses
  every example file. Done: six policies under `examples/policies/` +
  `examples/policies/README.md`; `authentication-verifier`'s
  `abac::tests::every_example_policy_parses` reads and `Policy::from_json`
  parses every file in the directory (`cargo test` green, 58 tests incl.
  4 doctests, clippy clean).
- [x] **EX-3 (S)** `examples/api/` — per-service request collections
  (`.http` files or curl scripts) for the main endpoints incl. auth
  handshake. Spot-verified against a running compose. Done: twelve
  `.http` files (one per service) + `00-auth-handshake.http` +
  `README.md` under `examples/api/`, written from each crate's actual
  `src/controllers/`/`src/api/rest/` route + handler code, not from the
  family narrative docs. Live-verified against
  `examples/compose/full-family.yml` (build, then `up -d` as two
  separate commands): health check + create + get curl-verified for
  all twelve services; search/list, match, check-duplicates,
  review-queue, cross-service links, FHIR read, and several
  engineering/operational sub-resources (care-pathway instance
  enroll+status, portfolio task move + sprint + burndown) also
  curl-verified live. The full auth handshake (signup → magic-link
  verify → session cookie → `POST /token` with CSRF → `/api/auth/me` →
  `/.well-known/paseto-keys` → account export/audit → admin-attributes
  403) was driven end-to-end against a throwaway `LOCO_ENV=development`
  authentication-service container on the same compose network, since
  the shipped compose bakes `LOCO_ENV=production` (SEC-A1 fail-closed)
  and `users.magic_link_token` is stored pre-hashed (SEC-A9) — neither
  the dev-console-log trick nor a direct DB read yields a usable token
  against the stock stack, which `00-auth-handshake.http` and
  `authentication.http` now document. Real discrepancies caught by
  curl and corrected before commit: worker's assessment endpoints
  return the bare resource, not the crate's usual envelope; `place`/
  `thing` require the full struct (no `#[serde(default)]`) rather than
  a minimal body; review-queue decisions take `{"status":...}`, not
  `{"decision":...}`; `case-service` has no review-queue HTTP surface
  at all (a made-up path 500s, mis-parsing "review-queue" as a UUID);
  care-pathway's instance status body is `{"to":...}` and its FHIR
  `$export` is `GET /fhir/$export` (not `POST .../PlanDefinition/$export`);
  portfolio's burndown requires `?sprint=`; and `/api/whoami` (and its
  loco-style `/api/<plural>/whoami` equivalents) unconditionally
  require a bearer token regardless of `<ENTITY>_REQUIRE_AUTH`. Also
  corrected a stale premise from the task brief itself: all four
  loco-idiomatic services (organization, care-pathway, case, portfolio)
  return bare loco JSON identically — case is not a `{success,...}`-vs-
  bare exception, they all are.
- [x] **EX-4 (S)** A demo **seed** path: loco task (person + organization
  + case) or documented bulk-import of EX-1 fixtures — pick one,
  reference it from TUT-1/2/4.

  **Done 2026-08-04.** Picked the **loco task**, not bulk-import — per
  EX-1's live finding that the shipped compose stack runs every
  container server-only (`COMPOSE-WORKER`), so `BulkJobWorker` never
  registers and a bulk-import job never leaves `queued`. A `seed_examples`
  task per crate (`person`, `organization`, `case`
  `src/tasks/seed_examples.rs`, registered in each `app.rs`) reads the
  matching `examples/data/*.jsonl` fixture and inserts through the
  **model layer** directly (`SeaOrmPersonRepository::create` /
  `organizations::Model::create` / `cases::Model::create`), bypassing
  the HTTP create handler's real-time duplicate detection — necessary
  because `persons.jsonl` deliberately carries five duplicate pairs that
  `POST /api/persons` would `409` on the second half of (EX-1 confirmed
  this live). No audit row or event is written by the seed itself (no
  audit log / event publisher attached to the model-layer path); that is
  a deliberate, documented choice, not a gap — the tutorials that
  exercise duplicate detection, audit, and events do so against records
  already present, not against the act of seeding. Each task counts its
  table first and refuses to insert into a non-empty one, so a re-run is
  a no-op rather than a duplicate load.

  Organization and case reuse each crate's own `bulk::jsonl::parse_line`
  (the same parser BLK-5 import already uses) rather than reinventing
  JSONL parsing; person parses directly into `models::Person` (the same
  type its own `bulk::jsonl::parse_line` uses). All three wire types
  matched the fixture files with no changes needed — EX-1's earlier
  offline verification of the fixtures against these same parsers meant
  no drift surfaced here.

  **Verified live against real Postgres**, all three crates, via
  `scripts/test-db.sh up <crate>` + `scripts/ci-check.sh test-db <crate>`
  (then `down`): a DB-gated test per crate truncates its table, runs the
  task, asserts the exact row count (50 / 20 / 10), asserts a second run
  changes nothing (idempotency guard held), and — person only — asserts
  both halves of the documented "Okonkwo/Okonkow" duplicate pair
  (`examples/data/README.md`) landed in `person_names`. All three
  crates' full DB-gated suites (not just the new tests) ran green
  alongside this — 21+30+34 request-level tests, several `#[ignore]`d
  outbox/enforcement/masking binaries — confirming the new task didn't
  disturb anything already there. `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` clean on all three crates
  (`-D warnings` is a hard CI gate, so this was actually run, not
  assumed). DB-free unit tests per crate parse one real line of the
  actual fixture file into the wire type, catching wire-type/fixture
  drift cheaply.

  **Docs**: a new "Loading into a running service" section in
  `examples/data/README.md` documents the task as the reliable path
  (cross-referencing `COMPOSE-WORKER`), each crate's `AGENTS.md`
  (organization, case) / `index.md` (person) gained a short mention +
  invocation, and each crate's `CHANGELOG.md` gained an `[Unreleased]`
  entry. No crate `spec/index.md` §13 edit: `integrity_key` and
  `integrity_resign` — the closest precedent, an operator/demo CLI
  utility rather than a service-behaviour change — landed in both
  `person` and `case` with **zero** mentions in any per-crate markdown
  (verified by grep before deciding this), unlike `search_reindex`
  (a real service capability, fully documented in spec + AGENTS +
  CHANGELOG). `seed_examples` is closer to `integrity_key` in kind, so
  it got the lighter footprint: AGENTS/README + CHANGELOG, no spec
  rewrite.

  **Not done, deliberately out of scope**: `TUT-1`/`TUT-2`/`TUT-4` do
  not exist yet (later in the flattened task order); they should
  reference `cargo loco task seed_examples` in each of the three crates
  once written. Case's `seed_examples` does not create the ten
  `subject_of` links in `case-subject-links.md` — the case/person pids
  are not known until after both seed tasks run, so that step stays a
  documented follow-up curl call, not automated here.

## Phase 5 — Security hardening (audit-driven, 2026-07-12)

> From the repo-wide security audit (plan.md Theme F). Severity: 🔴 critical
> · 🟠 high · 🟡 medium · ⚪ low. Each code task is three-part (crate spec
> §13 + code + **security test**) + CHANGELOG; the audit's "recommended
> tests" ARE the test half (this is where the "improve tests — fuzzing,
> races, unverified inputs" mandate lands). Criticals (SEC-A1, SEC-B1,
> SEC-G1, SEC-B5) lead; SEC-I1/I3 are cheap Phase-1 hygiene. File:line
> anchors are from `main` at audit time — re-verify before editing.

### F-authn — token & session integrity (authentication-service)

- [x] **SEC-A1 (S) 🔴** DEV_SEED prod guard. *(done 2026-07-12)* `load_seed()`
  (`src/auth/mod.rs`) silently fell back to the committed `DEV_SEED` →
  forgeable tokens. Now `dev_seed_fallback(is_production)` refuses the
  fallback when `LOCO_ENV`/`RUST_ENV` = `production`, so `load_keys()`
  errors and `keys()` boot-panics with guidance; dev/test still boot on
  `DEV_SEED`. Unit test `dev_seed_fallback_refused_in_production`
  (env-free, pure helper — edition-2024 + `forbid(unsafe)` rules out
  `set_var` in tests). Green: lib tests + clippy + fmt.
- [x] **SEC-A2 (S) 🟠** Gate `GET /api/auth/audit/recent`. *(done 2026-07-13)*
  Was unauthenticated + returned `auth_events` emails/outcomes (an
  enumeration oracle via timing). Now requires a PASETO bearer with
  `access=admin` (`401`/`403`); `recent_audit` takes `AuthUser` +
  `claims_have_admin`. Unit test `recent_audit_requires_admin`; DB-gated
  request test pins the `401`; spec §12 decision superseded.
- [x] **SEC-A3 (S) 🟠** Magic-link token logging. *(done 2026-07-13)*
  `deliver_magic_link` logged the full verify URL (embedding the live token)
  at `info` in every env. Now gated to `Environment::Development` only via
  pure `log_magic_link_url`; other envs log the issuance without the token.
  Unit test `magic_link_url_logged_only_in_development`.
- [x] **SEC-A4 (S) 🟡** Atomic single-use magic-link consume. *(done 2026-07-13)*
  Replaced SELECT-then-clear with `Model::consume_magic_token` — one
  `UPDATE … WHERE magic_link_token=$1 AND not-expired RETURNING *` (via
  `query_one` + `FromQueryResult`), so concurrent redemptions can't both win.
  DB-gated `concurrent_magic_link_redemptions_only_one_wins` (exactly one 200,
  one 401). Green: lib (71) + `test --no-run` + clippy + fmt.
- [x] **SEC-A5 (S) 🟡** Constant-work signup. *(done 2026-07-14)*
  `create_passwordless` returns `EntityAlreadyExists` before its Argon2 hash,
  so only the new-account path paid the deliberately-slow hash — a timing
  oracle for enumeration despite the always-`200` response. The
  existing-email branch now runs one equivalent Argon2 hash
  (`constant_work_hash`, discarded), so both paths perform one hash and
  signup latency is indistinguishable between new and existing. Unit test
  pins that a real `$argon2` hash is performed (fresh per call).
- [x] **SEC-A6 (S) 🟡** Rate-limit email canonicalization + case-consistent
  `find_by_email`. *(done 2026-07-14)* `rate_limit::normalize_key` folds the
  throttle bucket aggressively — trim + lowercase, strip `+tag`, Gmail/
  `googlemail` dot-folding — so `victim+1@gmail.com` / `v.ictim@gmail.com` /
  `Victim@…` collapse to one bucket (throttle-only; never loosens the quota).
  `users::find_by_email` + `create_passwordless` are now case-insensitive
  (`LOWER(email)` compare + `normalize_email` store), so a case variant is
  the same account, not a duplicate. **Deliberately case-only for identity**
  — `+tag`/dot folding is confined to the throttle bucket, not account
  identity (the security-clean subset; provider-specific folding must not
  merge distinct accounts). Pure key-collapse tests + a DB-gated
  case-variant signup test (one lowercased account).
- [x] **SEC-A7 (S) 🟡** GDPR erasure completeness. *(done 2026-07-13)*
  Erasure now scrubs the subject's email from `auth_events`
  (`AuthEvent::scrub_subject_email`, pid OR normalised-email match) and
  `sessions.user_agent` (`scrub_user_agent_for_user`), and writes
  `account_erased` without the email. Extended `account_erasure_…` request
  test asserts no `auth_events` row retains the email + user_agent scrubbed.
- [x] **SEC-A8 (M) 🟡** Privilege-revocation latency. *(done 2026-07-13)*
  The admin attribute API (`PUT …/attributes`) and the `user_attributes` CLI
  task now `sessions::Model::revoke_all_for_user` after a change, so a
  session that snapshotted the old attrs can't keep minting stale-attribute
  tokens until its absolute TTL — the next login copies fresh attrs. Extended
  admin request test asserts the target's sessions are revoked after the PUT.
- [x] **SEC-A9 (M) 🟡** Hash bearer-equivalent secrets at rest.
  *(done 2026-07-14)* Magic-link token, session `jid` (cookie / PASETO `sid`),
  and CSRF token (`sessions.data.csrf`) now store only a one-way SHA-256 hash
  (new `secret_hash` module; fast unsalted hash — high-entropy tokens, so
  deterministic lookup-by-hash, not Argon2). `create_magic_link` returns the
  plaintext in-memory (email/log) but persists the hash; `consume_magic_token`
  / `find_by_jid` / CSRF compare hash the presented value. Migration `_000009`
  enables `pgcrypto` + hashes existing rows in place
  (`encode(digest(x,'sha256'),'hex')`, guarded `length <> 64`) so live
  links/sessions survive. *Test:* `secret_hash` vectors + DB-gated
  `session_secrets_are_hashed_at_rest` / `magic_link` assert the DB holds no
  usable plaintext credential while presented-plaintext lookups still resolve.
- [x] **SEC-A10 (S) 🟡** CSRF origin backstop. *(done 2026-07-14)* Pure
  `csrf_token_gate(is_production, origin_ok, session_csrf, provided_csrf)` in
  `controllers/auth.rs`: a token-carrying session must echo `X-CSRF-Token`; a
  legacy no-`csrf` session must prove same-origin (`AUTH_ALLOWED_ORIGINS`) and
  is refused in production without it, so it can no longer bypass both checks.
  Unset allow-list in production warns once (`warn_missing_allowed_origins`).
  *Test:* `csrf_gate_matrix` — no-csrf session cannot bypass both CSRF and
  origin checks in production.

### F-authz — verifier & ABAC (authentication-verifier)

- [x] **SEC-V1 (S) 🟡** `from_paseto_keys_url` hardening. *(done 2026-07-13)*
  Now refuses non-`https://` URLs, forbids redirects (no https→http bounce),
  sets a 10 s timeout, and reads the body under a 64 KiB cap (MITM key
  injection / boot-hang / OOM). Test `non_https_keys_url_is_refused`.
- [x] **SEC-V2 (M) 🟡** Vacuous-negation escalation. *(done 2026-07-13)*
  A `!`-negated `resource.`/`env.` condition matched vacuously when the
  namespace was absent. An absent namespaced attr now biases by effect to
  the safe outcome: `allow` rules do NOT match (no silent grant), `deny`
  rules still match (fail-closed); subject-attr negation unchanged. Test
  `negated_allow_does_not_match_vacuously_when_namespace_absent` (+ existing
  deny-rule test preserved).
- [~] **SEC-V3 (S) ⚪** Key-set load resilience — **deferred**. Skipping a
  malformed Ed25519 entry (vs the current deliberate fail-fast on a
  malformed key set) contradicts a stated design decision + spec and is Low
  severity; left as an open call (fail-fast surfaces misconfiguration;
  skip-and-continue favours availability).
- [x] **SEC-V4 (M) 🟠 (tests)** Forgery + robustness suite. *(done 2026-07-13)*
  Added the previously-missing **cross-key forgery** test (attacker sig +
  honest `kid` ⇒ `Paseto` Err), `token_missing_exp_is_rejected`, and
  `malformed_tokens_never_panic` (arbitrary/truncated/oversized input ⇒ only
  ever `Err`, never panics). Example-based (no new deps); a full `proptest`/
  `cargo-fuzz` policy-property + parser-fuzz layer folds into SEC-I2.

### F-guard — read-path masking & guard consistency (entity services)

- [x] **SEC-G1 (M) 🔴/🟠** Governed bulk-links leak. *(done 2026-07-12)*
  `GET /api/cases/links` dumped every `subject_of` edge (and the person
  twin `GET /api/persons/links` every `same_identity` edge) with only the
  coarse gate + no audit. Both handlers now authorise the cross-record dump
  as a privileged governed read (`authorize_record(Action::Destructive,…)`
  — default policy admits only `svc`/`admin`) and audit each surfacing.
  Case: DB-gated `bulk_links_requires_elevated_authority` (401/403/200 in
  `tests/enforcement.rs`). Person: unit test pins the `Destructive`
  classification (shares case's e2e-proven gate). Green: both crates' lib
  tests + clippy + fmt (+ case `test --no-run` DB-gated).
- [x] **SEC-G2 (M) 🟠** Case FHIR read/search authz + mask. *(done 2026-07-13)*
  Both now take a caller and apply record-level ABAC + the `mask` obligation
  like native `get_one`: FHIR `read` → `403` on deny + masked Task on the
  `mask` obligation; FHIR `search` omits denied cases + masks the rest.
  DB-gated `tests/masking.rs` (denied caller gets `403` on `/fhir/Task/{id}`).
- [x] **SEC-G3 (M) 🟠** Masking-on-every-read. *(case done 2026-07-13; person
  done 2026-07-14)* Case `list`/`search`/`check_duplicates` **omit** cases the
  caller may not read (concealment) via the shared `auth::read_visibility`;
  DB-gated `tests/masking.rs` proves the list conceals a denied case. Person
  `search_persons` (`api/rest/handlers.rs`) now runs `auth::read_visibility`
  on every hit too: a denied record is omitted (concealed), a `mask`
  obligation masks even without the client `mask_sensitive` param (closing the
  bypass), and the param still masks on request; no-op when
  `PERSON_REQUIRE_AUTH` is off. Pure `search_result_disposition` unit test
  pins the omit/mask/full matrix. (Person `check_duplicates`/`match` return
  match candidates, not a record dump; a concealment pass there is an optional
  follow-up if a deployment needs it.)
- [x] **SEC-G4 (S) 🟡** `escape_like` in the repo-based searches. *(done 2026-07-13)*
  person / worker / event `db/repositories.rs::search` built `format!("%{}%",
  query.to_lowercase())` with no escaping (raw `%`/`_` = wildcard
  injection / scan-everything DoS; already a bound param so not SQLi). Each
  now escapes `\`/`%`/`_` via a per-crate `escape_like` helper before the
  contains-pattern; unit test `escape_like_neutralises_wildcards` in all
  three (ports the loco `escape_like` test). Green: lib test + clippy
  `-D warnings` + fmt per crate.
- [x] **SEC-G5 (M) 🟡** Guard-all for event/thing/course. *(done 2026-07-13)*
  Their `enforce` was **allow-unless-in-prefix** (any non-`/api`/non-`/fhir`
  path unguarded); now **deny-unless-public** via a `is_public_path`
  allow-list (matching the case reference), and the dead prefix consts/helpers
  removed. Guard-bypass test per crate: enforcement on + no token ⇒ `401` for
  `/`, `/admin`, `/secret`, `/foo/bar`. (The other 8 services are already
  guard-all. A percent-encoded/normalisation matrix vs the router is a
  deeper follow-up.)
- [x] **SEC-G6 (S) 🟡** Destructive-action classification robust to a
  trailing slash. *(all 10 services done 2026-07-13)* `derive_action` now
  `trim_end_matches('/')`-normalises the path before the destructive-suffix
  check, so `POST …/merge/` stays `Destructive` (was downgraded to `Write`,
  which an `access=write` non-admin caller could exploit). Rolled to all ten
  services (case, event, thing, course, care-pathway, portfolio, place,
  organization, worker, person) with a trailing-slash test per crate.
- [x] **SEC-G7 (S) ⚪** Bound person `search_persons` `offset`. *(done
  2026-07-14)* `GET /api/persons/search` rejects `offset > MAX_SEARCH_OFFSET`
  (10 000) with `400 OFFSET_TOO_LARGE` before asking the index for
  `offset + limit` hits (unbounded offset ⇒ index materialises arbitrarily
  many hits; the add could also overflow — now `saturating_add`). Pure
  `search_offset_within_bound` unit test + DB-gated `400` integration test.
- [x] **SEC-G8 (S) 🟡** Default-off exposure pin. *(done 2026-07-14)* Added a
  named unit test
  (`default_off_exposes_sensitive_reads_activation_is_a_release_gate`) to the
  two services the audit flagged for the bulk-links + audit exposure — **case**
  (PII / audit / governed `subject_of`) and **person** (PII / GDPR export /
  audit / `same_identity`) — pinning that with `<ENTITY>_REQUIRE_AUTH` off
  those reads are open without a token, so activation is a **tracked release
  gate** (framed in `agents/share/security.md` §4 from SEC-I4). The generic
  flag-off `enforce` pin already exists family-wide; this adds the explicit,
  sensitive-path-named form on the flagged services. Feeds OPS-1 runbook.

### F-data — bulk / linking / concurrency integrity

- [x] **SEC-B1 (M) 🔴** link-graph reconcile cross-entity scoping. *(done 2026-07-12)*
  It diffed the **global** read-model (`all_edge_ids`) against **one**
  entity's edges, so each entity pass deleted the others' edges and the
  graph never converged. `AuthoritativeSource` now declares `entity()`;
  `reconcile` diffs only `edges::Model::edge_ids_from_entity(source.entity())`
  (exact `<entity>:` `from_ref` prefix — correct for `subject_of` from=case
  and canonical `same_identity` from=person). DB-gated
  `reconcile_is_scoped_to_the_source_entity` (case pass leaves the person
  edge intact) + pure `from_ref_scoping_*` unit tests. Green: lib tests +
  `test --no-run` (DB-gated) + clippy + fmt.
- [~] **SEC-B2 (M) 🟠** person bulk import caps. *(caps + fuzz done
  2026-07-13; true streaming deferred)* Import upload read chunk-by-chunk
  and rejected `413` past `MAX_IMPORT_BYTES` (64 MiB) **before**
  materialisation (`read_field_capped`/`exceeds_cap`); pipeline rejects a
  load over `MAX_IMPORT_ROWS` (1M) via `split_lines_capped`; export `limit`
  clamped to `MAX_EXPORT_ROWS` (1M) via `clamp_export_limit` (worker mapping
  + pipeline listing path). proptest fuzzes `parse_line`/`split_lines`/
  `split_lines_capped` (random bytes / truncated UTF-8 / 2 MiB line never
  panic); boundary `exceeds_cap` unit-tested incl. saturating-add overflow.
  **Deferred:** true end-to-end streaming (never buffering the whole file,
  so the caps can rise) — the caps make the current buffered path safe.
- [x] **SEC-B3 (M) 🟠** person bulk upsert idempotency race. *(done
  2026-07-13)* The per-row find→create/update runs under a
  transaction-scoped advisory lock on the stable key
  (`pg_advisory_xact_lock(hashtext(key))`, `import_upsert_locked`), so two
  concurrent importers of one key produce exactly one record (the second
  upserts the first's). **Chose advisory lock over `UNIQUE(system,value)`**:
  the registry permits duplicate identifiers by design (dedup is a
  workflow), so a hard uniqueness constraint would reject legitimate data.
  DB-gated test: two concurrent imports of one SSN key ⇒ one distinct owner,
  one create + one upsert; plus a pure lock-key collision test.
- [~] **SEC-B4 (M) 🟠** person bulk artifact hardening. *(store confinement +
  IDOR + TTL done 2026-07-13; object-store sweep deferred)* (1) store `get`
  **confined** to the canonicalised base + `is_safe_key` on `put`/`get`
  (rejects `..`/absolute/`file://`-escape → closes arbitrary-file read);
  (2) job-status GET returns `404` unless the caller **owns** the job
  (`is_job_owner`, `actor == sub`) or is elevated (`access=admin`/`svc=true`)
  → closes IDOR/BOLA on status + download URL; (3) `create` stamps
  `expires_at = created_at + BULK_ARTIFACT_TTL_SECS` (7 days), status handler
  `404`s an expired job (`artifact_expired`). Pure cores unit-tested incl.
  the outside-the-base `file://` refusal. **Deferred:** physical artifact
  deletion (object-store TTL sweep) — the expiry gate stops the reference
  being handed out.
- [x] **SEC-B5 (M) 🔴/🟠** Merge TOCTOU + self-merge. *(done 2026-07-13)*
  person `POST /merge` had **no self-merge guard** (merged a record into
  itself → tombstoned + data loss) — now `422` before any fetch
  (`test_merge_into_self_is_rejected`; case already had the guard).
  Both merges read main+duplicate **unlocked** before the write tx; the
  person repository `merge` and the case `outbox` `merge_and_emit` now lock
  both participant rows `FOR UPDATE` (id-ordered, deadlock-free) and
  re-check the duplicate is still active before writing, so concurrent
  merges of the same duplicate can't both apply (loser fails closed).
  Green: person lib (186) + integration compile + clippy + fmt; case lib
  (96) + no-run + clippy + fmt. *(Residual: a deterministic concurrent-race
  integration test, and the case `memory` (dev, non-transactional)
  path — both follow-ups.)*
- [x] **SEC-B6 (M) 🟠** Relay exactly-once. *(all 10 services done 2026-07-13)*
  `drain_once` now runs in a transaction and `unpublished` claims rows with
  `FOR UPDATE SKIP LOCKED`, so >1 relay instance can't double-ship (a second
  instance skips the locked rows; lock releases on commit). Rolled to **all
  ten** loco/axum services (case, person, worker, place, thing, event,
  course, organization, care-pathway, portfolio) — each green (lib + clippy
  + fmt). Consumer-side `event_id` dedupe (`processed_events`) is the
  aggregator's job — folds into BUS-2; a deterministic two-concurrent-drain
  test needs a dual-connection harness (follow-up).
- [x] **SEC-B7 (S) 🟡** link-graph reconcile peer trust. *(done 2026-07-13)*
  `HttpAuthoritativeSource::from_env_for` refuses an **unauthenticated
  remote** source — a non-loopback URL requires `LINK_GRAPH_RECONCILE_TOKEN`
  (`source_auth_ok`/`is_loopback_url`, fail-closed on an unparseable URL);
  only a loopback URL may be token-less. Before `apply_linked`, `reconcile`
  validates each edge via `edge_valid_for_source`: it must originate from the
  source's own entity AND its endpoint types must be permitted for its kind
  (`EdgeKind::permits`), so a compromised/buggy source can't inject a
  cross-typed or foreign-origin edge (ill-typed edges skipped, stay as
  divergence). Pure helpers unit-tested (remote-needs-token, loopback-ok,
  ill-typed + foreign-origin rejected).
- [~] **SEC-B8 (S) 🟡** Bulk audit gaps. *(job-level audit + fail-closed
  export + actor threading done 2026-07-13; per-row actor deferred)* A
  successful import now writes a job-level `IMPORT` audit row (`log_import`)
  with the actor + reconciled counts; the export audit is written **before**
  `finish_export` and its error **propagates**, so a failed audit marks the
  job `failed` and never surfaces `download_url` (fail-closed delivery). The
  actor is threaded into both rows (fallback `system` only when the job had
  no caller). Pure `import_audit_summary`/`export_audit_summary` unit-tested.
  **Deferred:** threading the real actor into each **per-row** create/update
  audit — needs a `PersonRepository::create/update` signature change (they
  build a default `system` `AuditContext` today).
- [x] **SEC-B9 (S) 🟡** Wire the idempotency key. *(done 2026-07-13)* Both
  submit handlers read an `Idempotency-Key` header;
  `create_or_get_idempotent` returns the original job (no re-store /
  re-enqueue) when the key already names one, backstopped by the existing
  `UNIQUE(entity,kind,idempotency_key)` on the check-then-insert race (no
  migration needed — the constraint already existed, just never fired). Blank
  key ⇒ absent; key-less ⇒ always creates. DB-gated same-key/keyless tests +
  pure key-trim test.
- [x] **SEC-B10 (S) 🟡** person merge audit in-tx. *(done 2026-07-13)* The
  merge `UPDATE` (survivor) + `DELETE` (duplicate) audit rows are written on
  the merge transaction (new connection-generic `log_update_on`/
  `log_delete_on`) **before** commit, so a crash after commit cannot lose the
  merge audit and an audit failure rolls the whole merge back (was
  best-effort post-commit). DB-gated test asserts both rows present after a
  merge.
- [x] **SEC-B11 (S) ⚪** link-graph `freshness` authz + non-redirecting probe.
  *(done 2026-07-13)* The probe now uses a shared **non-redirecting** reqwest
  client (`redirect::Policy::none()`); a `3xx` ⇒ `Unknown`
  (`outcome_from_status`), closing SSRF-via-redirect — the only host
  contacted is the operator-configured `LINK_GRAPH_PROBE_URL_<ENTITY>`
  template, which *is* the host allow-list (no separate list needed once
  redirects are off). Freshness was already behind the blanket guard (not in
  `is_public_path`); added a regression test pinning it stays guarded (`401`
  when enforcement on) so it can't be mistaken for a public health probe.
  Pure status-mapping + freshness-guard tests.

### F-input — unverified input, false matches & fuzzing (validators + matchers)

- [x] **SEC-M1 (M) 🟠** Input-size caps. *(case + care-pathway + portfolio
  validators done 2026-07-13; `limit_payload` backstop + a broken-production-config
  sweep done 2026-08-02)* Per-field length + array-cardinality caps in
  `validate`/`problems` → `422` **before** persist, closing the O(n·m)
  Jaro-Winkler/Levenshtein/Jaccard DoS. Shared caps `MAX_TEXT_LEN=1024` chars
  / `MAX_ARRAY_LEN=256` entries / `MAX_ITEM_LEN=512` chars, incl. struct-array
  inner strings; false/oversized unit tests + within-caps pin; each crate
  green): case, care-pathway, portfolio, **organization** *(new
  `src/validation.rs`, done 2026-07-13)*, **course** *(caps woven into
  `validate_course`/`validate_instance`, done 2026-07-14)*, and the **5 older
  axum services** (person/worker/place/thing/event `validation/mod.rs` — each
  `<entity>_size_caps` woven into `validate_<entity>`, done 2026-07-14).
  **The `limit_payload` backstop (2026-08-02):** loco's own default
  (`loco_rs::controller::middleware::limit_payload`) is a hardcoded 2MB and
  is *always* active whether or not a service's config declares it — so a
  config with no `limit_payload` key silently runs on 2MB, and a config that
  declares a *smaller* framework cap than an application-level upload
  feature silently breaks that feature before its own check ever runs. Two
  real breaks found this way: person-service's 64MB bulk-import
  (`src/bulk/mod.rs::MAX_IMPORT_BYTES`) was capped at 5MB in production/2MB
  in dev+test; content-management-system's 25MB asset upload
  (`src/controllers/assets.rs::DEFAULT_MAX_UPLOAD_BYTES`) was capped at 2MB
  everywhere. Fixed by raising person to `70mb` and CMS to `30mb` (headroom
  above their app-level caps) in all three environments, and adding an
  explicit `2mb` backstop (formalizing, not changing, today's implicit
  default) to the other 15 crates with a `config/` directory; worker/place/
  thing/event's previous production-only `5mb` was tightened to the same
  explicit `2mb` used elsewhere so all three environments agree. Verified by
  booting person-service and content-management-system against their test
  DBs and posting a 3MB body past the old implicit 2MB default (person:
  reached the app's own 422 validation; CMS: reached the app's own 404
  route logic) — neither hit a 413.
  **Broken production.yaml discovery:** while adding the backstop, found 9
  crates' local `config/production.yaml` contained **only** a `cache:`
  block — no `server:`, `database:`, or `logger:` at all. loco's top-level
  `Config` struct declares all three as required fields with no defaults
  (`loco_rs::config::Config`), so a file in that shape could not have
  deserialized in production: the service would fail to boot outright,
  independent of any body-limit concern. Of the 9, this was a **real,
  committed defect** for 3 — content-management-system,
  contact-relationship-management, workforce-planning-management — whose
  `config/production.yaml` is tracked in git and is what a deployment
  actually clones; those are genuinely fixed by this change. The other 6 —
  care-pathway, case, authentication, organization,
  project-portfolio-management, patient-flow — `.gitignore` **every**
  crate's `config/production.yaml` (`**/config/production.yaml`), so no
  production config is committed for them at all; the empty-looking file
  found locally was a stray, untracked scratch artifact, not something any
  clone or deployment inherits — those 6 crates never shipped a broken file
  because they never ship a file. Filled out anyway, for parity and in case
  the local tree is itself used to deploy, but the fix does not appear in
  `git status` and is **not part of this commit** for those 6; a real
  deployment of those crates must still author its own `production.yaml`
  from scratch (there is also no tracked `.example` template for any of
  them to copy). Written to mirror each crate's own
  development.yaml/test.yaml (same env-var names: `DATABASE_URL`,
  `DB_CONNECT_TIMEOUT`, `DB_IDLE_TIMEOUT`, `DB_MIN_CONNECTIONS`,
  `DB_MAX_CONNECTIONS`, `PORT`,
  `HOST`, `JWT_SECRET`), plus the family's production middleware shape
  (person-service's compression/etag/limit_payload/cors, reference) and an
  SMTP-via-env mailer block. All 51 config files (17 crates × 3
  environments) now parse as valid YAML with `logger`/`server`/`database`
  present and an explicit `limit_payload`, verified with a Tera-placeholder
  + PyYAML sweep.
- [x] **SEC-M2 (M) 🟠** False-deterministic-match empty guards. *(done 2026-07-13)*
  A post-normalization empty/trivial-value guard added to every string-keyed
  deterministic short-circuit across **all 9 matchers**, each with a
  false-match unit test (two different records sharing only a
  blank/punctuation/trivial value MUST NOT match) + preserved positive test:
  person/worker passport `passport_books_share_pair` + demographic fallback
  (non-empty normalised names); place `name_and_postcode_match`; thing
  `same_canonical_url` + `shares_same_as` (skip empty); event
  `name_and_start_date_match`; course + care-pathway R-1 provider-scoped
  code (require non-empty normalised code); case R-0 identifier
  (`is_trivial_identifier`: empty / `"0"` / all-zeros UUID) + R-2 `same_as`
  `"/"`; portfolio R-2 `same_as` `"/"`. Each crate re-verified independently
  green (test + clippy `-D warnings` + fmt).
- [x] **SEC-M3 (S) 🟠** Reject sentinel national IDs (all-zeros). *(done 2026-07-13)*
  `parse_ie_ihi`/`parse_es_tsi`/`parse_dk_cpr` in both the person- and
  worker-matcher `identifiers.rs` now reject an all-zeros placeholder (via
  a shared `is_sentinel_zeros` helper), matching the `nl_bsn` posture, so a
  `"0000000"` sentinel shared by two records cannot short-circuit to 1.0.
  Unit test `format_only_parsers_reject_all_zeros_sentinels` in both crates.
- [x] **SEC-M4 (S) 🟡** portfolio `days_from_civil` overflow. *(done 2026-07-12)*
  `iso_date_to_days` parsed `year` as unbounded `i64`, so a crafted date
  overflowed `era*146_097` (panic debug / wrap release) via the timeframe
  component. Year now bounded to ISO `0..=9999`; out-of-range ⇒ `None`.
  Test `iso_date_year_is_bounded_and_never_overflows` (incl. `i64::MAX`
  year). Green: lib tests + clippy + fmt.
- [x] **SEC-M5 (S) 🟡** organization identifier validation. *(done
  2026-07-14)* `validation::problems` (`identifier_problem`) validates the
  deterministic schemes before store: **LEI** (ISO 17442, 20 alnum + ISO
  7064 MOD 97-10), **GLN** (13 digits + GS1 mod-10 check digit), **DUNS**
  (9 digits — no public check digit), **VAT** (2-letter country prefix +
  2–13 alnum; per-country check digits deferred). A bad value ⇒ field-scoped
  `422`; non-deterministic schemes unconstrained. Pure check-digit helpers
  unit-tested with hand-verifiable values (GS1 `5901234123457`; ISO 7064
  synthetic `…098`).
- [x] **SEC-M6 (M) 🟠 (tests/infra)** Matcher property harness. *(proptest done
  2026-07-13; cargo-fuzz = SEC-I2, still pending)* Added `proptest = "1.11"`
  (dev-dep) + property tests to the five newer matchers (course,
  organization, care-pathway, case, portfolio — the older five already had
  it). Invariants pinned per crate: **never panics** (engine + pure helpers
  on arbitrary UTF-8), **score ∈ [0,1]** & finite, **symmetric**,
  **identical ⇒ is_match / ≥ threshold**, Soundex shape `[A-Z][0-9]{3}`/None;
  portfolio also pins the **kind gate** (cross-kind ⇒ 0.0) and an
  `iso_date_to_days` no-overflow property (reinforces SEC-M4). The symmetry
  property surfaced a **real bug** in course `provider_score` (asymmetric on
  a one-sided empty `provider_id`) — fixed to require both sides non-empty
  (three-part). Each crate independently re-verified green. cargo-fuzz
  targets remain as SEC-I2.

### F-assurance — supply-chain & test infrastructure

- [x] **SEC-I1 (M) 🟠** Dependency-scanning CI + `deny.toml`. *(done 2026-07-13)*
  Added a per-crate `deny.toml` (advisories + a permissive-license allow-list
  + `private.ignore` for local crates + bans/sources = warn) and a
  `Security Audit` `security.yml` (single `cargo deny check` job, on
  push/PR/weekly) to **all 25 Rust crate roots** (services + matchers +
  libs) — previously only 3 services had any dep-scanning. Consolidated on
  `cargo deny` (same RUSTSEC DB as `cargo audit` but honours the ignore
  policy). **All 25 pass `cargo deny check` locally** (verified). The scan
  surfaced **real transitive advisories** in the shared loco-rs tree
  (quick-xml namespace-decl DoS via opendal, protobuf recursion,
  unmaintained async-std/instant/paste, …); these are not fixable at the
  service level, so they're `ignore`-listed **with justification** and to be
  revisited on the next loco-rs bump — see the note below. Matcher/library
  crates (small trees) pass clean with no ignores needed.
- [x] **SEC-I2 (M) 🟡** `cargo-fuzz` scaffolding. *(all 9 matchers +
  auth-verifier + person-bulk done 2026-07-14; CI wiring done 2026-08-02)*
  Each matcher has a standalone `fuzz/` cargo-fuzz crate (not a workspace
  member, so it never touches the stable build) with libFuzzer targets
  mirroring the SEC-M6 invariants: `match_<entity>` (JSON deserialize →
  engine; finite score ∈ [0,1], both orders) plus the pure-helper targets
  that crate exposes; `fuzz/README.md` documents run + roll-out. **Targets
  — all 9 matchers:** person (reference), **worker / place / thing / event**
  (3 targets: match + `normalizer` + `scorer`), and **course / organization /
  care-pathway / case / portfolio** (2 targets: match + `normalize` — these
  expose their similarity primitives only through the engine, no public
  `Scorer`). Each verified `cargo +nightly fuzz build` (cargo-fuzz 0.13.2,
  nightly) + short campaigns run clean (millions of execs each, no
  panics/crashes; e.g. place `match_places` 2.7M, worker `match_workers`
  3.35M, case `match_cases` 2.37M). **Also:** the **auth-verifier** `fuzz/`
  crate — `verify` (the PASETO `v4.public` token parser: header / footer
  `kid` / signature over an arbitrary token) and `policy`
  (`Policy::from_json` + `evaluate_with_context` — the ABAC parser + rule
  evaluator), both pinning golden rule #5 (no panics); verified clean at
  `verify` 11.1M / `policy` 6.6M execs. Plus the **person bulk** `parse_line`
  target (`bulk::jsonl` split + per-line JSON parse over attacker-supplied
  upload bytes; verified clean, 173k execs). 28 targets total across 12
  fuzz crates.
  **CI wiring (2026-08-02):** a new `fuzz` stage in `scripts/ci-check.sh`
  — a no-op for any crate without a `fuzz_targets/` directory (same
  pattern as `deny`/`evidence`), and for a fuzz sub-crate, `cd`s to the
  *parent* crate (`cargo fuzz` cannot run from inside `fuzz/`) and runs
  `cargo +nightly fuzz run <target> -- -max_total_time=${FUZZ_SECONDS:-30}`
  per target discovered from `fuzz_targets/*.rs`. Wired into a new,
  **separate** `.github/workflows/fuzz.yml` (discover job filters
  `scripts/ci-crates.sh` output to paths ending `/fuzz`, then a per-crate
  matrix job installs nightly + cargo-fuzz and runs the stage) on the same
  push/PR/weekly cadence as the per-crate Security Audit workflow, plus a
  `fuzz-smoke` step in `.woodpecker.yml` (Woodpecker has no inline cron, so
  it runs on every triggering event there). Kept **out of** the main
  `ci.yml`/pipeline deliberately: a nightly + ASAN-instrumented build is
  much slower than the stable fmt/clippy/test pipeline it would otherwise
  slow down on every push. This is a **smoke run**, not exhaustive fuzzing
  — no corpus persists between runs (`target/`, `corpus/`, `artifacts/` stay
  gitignored per crate); it exists to catch an immediate regression, not to
  replace a real long-running local campaign. Verified locally: a full
  sweep (`FUZZ_SECONDS=8 scripts/ci-check.sh fuzz`, no crate arg) ran all 28
  targets across all 12 fuzz crates clean, zero failures. `person-matcher`'s
  `fuzz/README.md` documents the CI shape as the reference (other crates'
  READMEs point at it).
- [x] **SEC-I3 (S) ⚪** Add `#![forbid(unsafe_code)]` to every crate root
  missing it. *(done 2026-07-14)* The three named roots (care-pathway-matcher
  `src/main.rs`, case-folder `src/lib.rs` + `src/bin/main.rs`) **plus** the 12
  SeaORM `migration/src/lib.rs` roots — the only remaining gaps a full grep
  surfaced. Now **every** `src/lib.rs` / `src/main.rs` / `src/bin/main.rs` in
  the workspace forbids `unsafe`. Builds clean; grep shows full coverage.
- [x] **SEC-I4 (M) 🟡** `agents/share/security.md`. *(done 2026-07-14)*
  Written: provenance (2026-07-12 audit), the audit summary by theme
  (F-authn/authz/guard/data/input/assurance with per-item status), the 10
  cross-cutting invariants (fail-closed secrets / never-panic / bound-input /
  no-spurious-identity / masking-on-every-read / fail-closed-authz /
  trusted-source-verify / concurrency-integrity / no-secret-in-logs /
  least-authority-artifacts), the `<ENTITY>_REQUIRE_AUTH` activation gate,
  secret-handling, the threat model, and a status snapshot. Wired into
  `agents/share/index.md`, the root `AGENTS.md` `@`-includes, and both
  compliance docs. Feeds OPS-1 runbooks.

---

## Suggested execution order (flattened)

**Security criticals FIRST** (before any exposed/enforced deployment):
SEC-A1 → SEC-B1 → SEC-G1 → SEC-B5 → then the F-authz forgery/fuzz proof
SEC-V4. Cheap security hygiene lands with Phase 1: SEC-I1, SEC-I3.

H-1 → H-4 → **SEC-A1 → SEC-B1 → SEC-G1 → SEC-B5** → SEC-I1 → SEC-I3 →
H-2 → H-3 → H-5 →
S-1 → P-1 → AU-1 → AU-2 → AU-3 → S-2..S-4 (parallelizable) →
P-2..P-4 (parallelizable) → PG-1 →
(F-input with search/privacy: SEC-M1 → SEC-M2 → SEC-M3 → SEC-M4 → SEC-M5 →
SEC-M6 → SEC-I2) →
(F-guard with B-auth: SEC-G2..G8) → (F-authn remainder: SEC-A2..A10) →
(F-authz remainder: SEC-V1..V3) →
LNK-1 → LNK-2 → LNK-3 → BLK-1 → BLK-2 → BLK-3 → BLK-4 → BLK-5 →
(F-data with A-bulk/A-link: SEC-B2 → SEC-B3 → SEC-B4 → SEC-B6 → SEC-B7 →
SEC-B8 → SEC-B9 → SEC-B10 → SEC-B11) →
BUS-1 → BUS-2 → BUS-3 →
DEP-1 → DEP-2 → OPS-1 (+ SEC-I4, SEC-G8) → FE-1..FE-4 →
EX-1..EX-4 → TUT-1..TUT-6 → LNK-4 (spec-first, last).

Parallelization note: S-2/S-3/S-4, P-2/P-3/P-4, AU-1's five services,
BLK-5's two services, and the per-matcher SEC-M2/M6 + SEC-I1 rollouts are
good one-subagent-per-crate fan-outs — give each agent the reference-crate
paths and the green gate verbatim, then re-verify independently before
committing (see plan.md §4).

## Found 2026-07-18 (while fixing the family-wide EntityNotFound→500 bug)

- [x] **QA-CASE-MASK (M)** *(fixed 2026-07-18)* — the test was **born
  failing** (reproduced at its birth commit c4e34443): its
  subject-only deny rule (`dept=blocked`) matched at the coarse
  blanket guard, 403ing the surface before the record-level
  concealment it meant to pin could run. Contract clarified in the
  test doc: a subject-only deny **belongs to the coarse guard**
  (defense-in-depth); SEC-G3 concealment is the property of callers
  who pass the guard and are denied on *specific records* via
  `resource.*` conditions (which, per SEC-V2, never match on the
  coarse no-record path). The test now uses a resource-scoped deny
  (`dept=blocked` + `resource.case_type=investigation`) on an
  Investigation-typed case — list concealment, native-GET 403, and
  FHIR 403 all actually exercise the record-level pass. Green vs
  Postgres 18. Bonus: the same order-dependence class was found and
  fixed in case's shared requests binary
  (`blanket_enforcement_gates_api_but_not_public_paths` duplicated
  `tests/enforcement.rs` and only passed when it ran first —
  removed; the dedicated binary owns that pin).
- [x] **QA-CP-FLAKE (S)** *(fixed 2026-07-18)* — moved to its own
  `tests/enforcement.rs` binary (the case / patient-flow pattern):
  the flag is now set before the process's only boot, so the pin is
  order-independent. Full care-pathway DB-gated suite green (1 + 22
  + 1 across the three binaries).

## Done 2026-08-01 — every service's DB suite now runs

- [x] **QA-SWEEP (M)** *(done 2026-08-01)* — ran all eight remaining
  unenrolled DB-gated suites through their new containers (DEP-0). Five
  were **already green and had simply never been run**:
  contact-relationship-management (8 tests), link-graph (16),
  patient-flow (9), place (2), thing (2). Three were red; all three are
  fixed, and **all 17 service crates are now enrolled** in
  `ci/db-suites.txt`.

  **course — 6 of 14 failing, two stacked causes.**
  - `POST /api/courses` stored an explicit all-zeros `id` verbatim.
    `Course::id` mints via `#[serde(default)]`, which only applies to an
    *absent* field — so the first create claimed the nil UUID and every
    later one died on the primary key with a `500`. The handler now mints
    on nil, matching the event service. *(Product fix.)*
  - The fixtures fought the duplicate detector: names were
    `Integration <suffix> <micros>`, and consecutive microsecond stamps
    share nearly every leading digit, scoring ~0.92 on Jaro-Winkler.
    Swapping in a UUID was not enough — the constant `Integration `
    prefix held the score at ~0.88 via the prefix bonus. Names now lead
    with the random token. The detector was right; the fixtures were
    wrong. Now 14/14.

  **event — 1 failing, a product defect.** `POST /api/events` *required*
  `created_at` / `updated_at`, which the repository sets on insert and
  refreshes on update — it demanded values it then discarded, answering
  `422 missing field created_at`. Both are now `#[serde(default)]`. The
  test also now reads the body before asserting the status, which is what
  turned "422 != 201" into a one-run diagnosis. Now 6/6.

  **portfolio — 2 failing, both test bugs; the service was right.**
  - The automation test read `moved[0]["assignee_ref"]` from
    `GET /tasks`, which answers `{ "tasks": [...], "counts": {...} }` —
    indexing an object gave `Null`, reading as "the automation never
    fired". It had: an `applied` run was logged and the row carried the
    assignee.
  - The burndown test hard-coded a July sprint window. Burndown counts
    `done_at` stamps within the window, the test completes a task *now*,
    and once now drifted past `ends_on` the completion stopped counting.
    The window is now relative to today. Now 36/36.

  Worth stating plainly: of the four defects this sweep found across the
  family (counting authentication's), **two were in shipped product code**
  and two were tests that had rotted against changes nobody could have
  noticed — because the suites had never run. The cost of a DB-gated
  suite that never runs is not zero; it is the illusion of coverage.

- [x] **QA-SERVER-FIELDS (S)** *(fixed 2026-08-04)* — `POST /api/places`
  (and, on the same evidence, `thing`) **required the fields the server
  owns**: `id`, `is_deleted`, `created_at`, `updated_at`, `keywords` and
  every other collection field the model declared without a serde
  default. Omitting one made the JSON extractor answer `422 missing
  field …` before a handler ran, for a value the repository then
  discarded. Confirmed real first: a hand-built POST body (not built via
  `Place::new()`/`Thing::new()`) 422'd on `missing field id` against a
  live Postgres, before any fix landed.

  Same defect, same fix, as the event-service `created_at`/`updated_at`
  fix (2026-08-01): every server-managed field is now
  `#[serde(default)]` in both `Place` and `Thing`. `name` — the one
  field the server does *not* own — is also `#[serde(default)]` now, so
  an omitted name reaches `validate_place`/`validate_thing` (`422
  validation_error`) instead of being turned away by the extractor's
  generic "missing field" error.

  Making the fields merely optional would have been a *new* bug on its
  own: an omitted `id` defaults to the nil UUID, and both repositories
  previously persisted whatever `id`/`created_at`/`updated_at` the
  domain value carried **verbatim** rather than overwriting them — so a
  second hand-written create would have collided on the same nil
  primary key, and an omitted timestamp would have stored the Unix
  epoch. Fixed alongside: `create_place`/`create_thing` mint a fresh id
  whenever the wire value is nil (mirroring the event service's
  existing pattern), and both repositories now stamp
  `created_at`/`updated_at` to "now" on insert (preserving `created_at`
  and refreshing `updated_at` on update) instead of trusting the
  passed-in domain value.

  New DB-gated regression suite in both crates
  (`tests/api_integration_test.rs`, 3 tests each): a minimal hand-built
  create body succeeds and reads back a fresh id, ~now timestamps, and
  empty collections; two consecutive hand-written creates mint distinct
  ids rather than colliding; an omitted `name` still fails, but via
  `validation_error`, not the JSON extractor. Verified live:
  `scripts/ci-check.sh test-db` green for both crates (place 9/9,
  thing 9/9, fresh Postgres 18 each time), plus `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` clean on both. The stale
  "recorded as QA-SERVER-FIELDS rather than fixed here" comment in each
  crate's `tests/enforcement.rs` is updated to point at the new suite.

## Found 2026-08-01 (first run of the authentication DB suite)

- [x] **QA-AUTH-DB (M)** *(fixed 2026-08-01)* — `authentication-service`'s
  DB-gated suite had **never been run**; the containerised database
  (DEP-0) made it a one-liner, and it came up 16 pass / 22 fail. Three of
  the four causes were test rot; one was a **production defect**.

  - **Every `LOWER(email)` lookup failed against Postgres.**
    `Expr::cust_with_values("LOWER(email) = ?", …)` emits the `?`
    verbatim — a MySQL placeholder where Postgres wants `$n` — so the
    driver sent `… WHERE LOWER(email) = ? LIMIT $1` and Postgres rejected
    it with `syntax error at or near "LIMIT"`. That is `find_by_email` and
    the duplicate-account guard: **signup and magic-link sign-in returned
    500**. Replaced with sea-query's typed `LOWER()`.
  - `src/fixtures/users.yaml` never gained the ABAC `attributes` column;
    loco seeds by deserializing into the entity, so every model test
    aborted on `missing field 'attributes'`.
  - Two request tests redeemed the magic-link token they read back out of
    the database — but SEC-A9 stores only its hash. The helper now issues
    a link through `create_magic_link` and uses the plaintext it returns.
  - One test asserted the decommissioned `/.well-known/jwks.json`. It now
    asserts the PASETO key set, plus that nothing serves a key set at the
    old path — checking the *body*, because loco's fallback middleware
    answers unmatched routes with `200` and a status check would pass
    either way.

  38/38 green vs Postgres 18; crate enrolled in `ci/db-suites.txt`.

- [x] **QA-CUST-SQL (S)** *(decided 2026-08-04)* — the same
  `cust_with_values` footgun was suspected latent in **person**,
  **worker**, and **event** (`src/db/repositories.rs`:
  `LOWER(family|name) LIKE $1`). All three already spell the
  Postgres-style `$1` placeholder (not MySQL's `?`), so none carried
  the auth-service bug — confirmed live against Postgres 18 for all
  three, not assumed. The three crates got **different** answers,
  decided independently per the actual evidence of who calls `search()`:

  - **person — exercise it (it already is).** `grep -rn` found a real
    caller: `src/bulk/pipeline.rs::run_export` calls `repo.search(q)`
    whenever a bulk-export request carries a `query` filter — this
    method is *not* dead code here, unlike the other two. It is already
    covered by an existing DB-gated test,
    `bulk::pipeline::db_tests::export_round_trips_through_jsonl` (plus
    the masked/CSV/Parquet export variants, which also set
    `query: Some(...)`), all of which pass against a live Postgres 18
    (`scripts/ci-check.sh test-db`, 21/21 lib unit tests green). No code
    change — the method was already exercised and already correct; only
    the verification (and this record of it) is new.
  - **worker — delete it.** `grep -rn` across the whole crate (handlers,
    tests, benches — worker has no bulk module) found **zero callers**:
    `/api/workers/search` goes through Tantivy, and no test ever called
    the repository method either. Removed the trait method, its impl,
    and its now-orphaned `escape_like` SEC-G4 helper + `Expr` import
    (nothing else used them). `scripts/ci-check.sh test-db` 34/34 green,
    `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`
    clean.
  - **event — delete it.** Same shape, same finding, same fix: zero
    callers (`grep -rn`, no bulk module), removed alongside its
    `escape_like` helper and `Expr` import. `scripts/ci-check.sh
    test-db` 7/7 green, `cargo fmt --check` and `cargo clippy
    --all-targets -- -D warnings` clean.

  Each crate's decision landed in its own commit + `CHANGELOG.md`
  entry, per the family's three-part-change discipline.

## Done 2026-08-01 — a containerised test database per service

- [x] **DEP-0 (M)** *(done 2026-08-01)* — **`compose.test.yaml` in all 17
  service crates**: one `postgres:18-alpine` container each (Podman, not
  Docker), providing exactly what that crate's DB-gated suite needs and
  matching what CI provides (`.github/workflows/ci.yml` `test-db`):
  superuser `loco`/`loco`, port 5432, the database its `config/test.yaml`
  names. Driven by the new **`scripts/test-db.sh`**
  (`up`/`down`/`psql`/`logs`/`url`/`status`/`down-all`), which waits on
  the container healthcheck instead of sleeping. Extensions come from one
  shared init script (`ci/postgres-init/`, mounted read-only into every
  container) that enables them in **`template1`**, so the `ci_*` databases
  `ci-check.sh` creates per crate inherit them. PGDATA is on **tmpfs**:
  every `up` is a fresh `initdb`, and a test database that accumulates
  state is the difference between a real failure and a stale one.

  `scripts/ci-check.sh test-db` gained **`DB_SUITES_FORCE=1`**, which runs
  an unenrolled crate anyway — the missing half of the `ci/db-suites.txt`
  rule that a crate is enrolled *once observed green*. Together these are
  what unblocks enrolling the nine services still outside that allowlist.

  *Verified:* all 17 compose files parse, and all 17 containers were
  started, reported healthy, and served the 5 expected extensions. Four
  suites were then run end to end through a container —
  **organization 22/22, person 38/38, link-graph 16/16, place 2/2** — and
  side-by-side operation (`TEST_DB_PORT=5434`), `status`, and `down-all`
  were exercised.

  Three findings worth keeping:
  - **A healthy container is not a reachable one.** On macOS podman
    publishes on IPv6 `*`, so a Postgres already holding IPv4
    `127.0.0.1:<port>` answers `localhost` first: the container is
    healthy, the connection succeeds, and the error is "database does not
    exist" — which reads like a broken container. Hit while testing the
    second-port path on this machine. `test-db.sh up` now probes the
    published port from outside and says so.
  - **The old `docker-compose.test.yml` files (person / worker / event)
    were wrong in three ways** and are removed: credentials
    (`test_user`/`test_password`) matched neither CI nor `config/test.yaml`;
    the tmpfs mount was at `/var/lib/postgresql/data`, which the 18 image
    does not use (PGDATA is `/var/lib/postgresql/18/docker`), so it
    silently did nothing; and their `test-runner` service built a
    `Dockerfile.test` pinned to Rust 1.93 against a repo pinned to 1.96.1,
    running a `cargo test --test api_integration_test` command that no
    longer describes the suite. The three `Dockerfile.test` files went
    with them.
  - **Six crates' `config/test.yaml` disagreed with everything else.**
    person / worker / event / place / thing defaulted to
    `postgres://localhost/<db>` (no credentials — implicitly the
    developer's OS user), and case-folder to `postgres:postgres`. All six
    now default to `loco:loco@localhost:5432`, so config, container, and
    CI finally name the same connection.

  **Found here, fixed next** (see QA-AUTH-DB above):
  `authentication-service`'s DB-gated suite was **red — 16 pass, 22
  fail** — the first time it had ever been run. One of the causes was a
  production defect in the signup / sign-in path.

## Found 2026-07-31 (during the doc harmonization pass)

- [x] **FE-LILY-RENAME (M)** *(done 2026-07-31)* — Lily renamed its
  two helper packages upstream: `-locale-select` → `-locale-picker`
  and `-theme-select` → `-theme-picker`, components likewise
  `LocaleSelect`/`ThemeSelect` → `LocalePicker`/`ThemePicker`. All 15
  older front-ends declared the old `file:` paths, which no longer
  exist, so a fresh `pnpm install` failed in every one of them. They
  built only because pnpm's store still held a copy.

  **All 16 front-ends are now on the pickers** (the 15 plus the CMS
  client, which was already there). Verified: `pnpm install` resolves
  the new packages everywhere with no stale `-select` directory left;
  `svelte-check` clean in all 16; **467 unit tests** pass; all 16
  build; **15 of 16 Playwright suites pass** — case-folder's is
  skipped for the pre-existing reason that its pre-flight needs the
  live Rust backend.

  It was not a pure rename, as expected. Three things had to change
  beyond the identifiers:
  - **Chrome CSS.** The old component rendered a `<select>` inside a
    `.theme-select` / `.locale-select` root. The picker renders a
    button plus a `ul` listbox, so every `:global(select)`,
    `:global(.theme-select)` and `:global(.locale-select)` rule was
    styling nothing. Repointed at `.theme-picker-button` /
    `.locale-picker-button` in 11 layouts.
  - **Playwright.** Two suites drove `select.locale-select` with
    `.selectOption()`, which cannot operate a listbox. Both now use a
    `chooseLocale` helper.
  - **case-folder's tracked `package-lock.json`** carried the old
    paths alongside its `pnpm-lock.yaml`.

  Two findings worth keeping, both discovered by running the thing
  rather than reading it:
  - The options carry **no `lang` attribute** and the **theme picker
    renders `li[role="option"]` too**, so a bare
    `li[role="option"][lang="de"]` selector matches nothing while
    `li[role="option"]` matches 58 elements. The list must be scoped
    (`ul.locale-picker-list`) and the option matched by its label.
  - **The picker stays open after a pointer selection**, so a test
    that clicks the button again to make a second choice *closes* it.
    The helper opens only when collapsed, which is correct either way.
    Reported upstream; Lily shipped a fix on 2026-07-31 for the
    related **effect re-entrancy** (`onChange` writing reactive state
    re-entered the apply effect, hitting
    `effect_update_depth_exceeded` and freezing the component with a
    stale `aria-expanded`). That fix is now in every front-end — see
    FE-LILY-REFRESH — and the freeze is gone; the list still being
    expanded after a click persists, and the helper is agnostic to
    it.

- [x] **FE-LILY-REFRESH (S)** *(done 2026-07-31)* — refreshed all 16
  front-ends onto Lily's fixed pickers after the upstream repair of
  the apply-effect re-entrancy (`onChange` writing reactive state
  re-entered the effect, hitting `effect_update_depth_exceeded` and
  freezing the listbox with a stale `aria-expanded`; the fix guards on
  an `appliedValue` so re-applying is idempotent).

  Two mechanical notes for the next time upstream changes:

  - **`dist/` is what consumers get.** The package `exports` point at
    `./dist/index.js`, and the fix landed in the package-root source
    only, so the front-ends would have kept the old behaviour. Run
    `npm run build` in `lily-design-system-svelte-helpers` first;
    upstream's own suite (211 tests) passing does not mean `dist` was
    rebuilt.
  - **`pnpm install --force` is not always enough** for a `file:`
    dependency: pnpm reuses the content-addressed store entry, and
    three apps kept the stale copy until `node_modules` was removed
    and reinstalled. Verify by diffing the installed
    `dist/LocalePicker.svelte` against upstream's rather than trusting
    the install to have done it.

  Verified: all 16 installed copies byte-identical to upstream's
  `dist` for both pickers; `svelte-check` clean; 467 unit tests pass;
  15/16 Playwright suites pass (case-folder needs its live backend);
  no `effect_update_depth_exceeded` in the browser.

## Phase 6 — Documentation harmonization (spec/AGENTS/README audit, 2026-08-04)

> Repo-wide: every subproject's `spec/` (the SDD single source of
> truth), `AGENTS.md`/`CLAUDE.md`/`AGENTS/*` (working agreements), and
> `README.md`/`index.md` (navigation + quick start) checked against the
> **current** code, not against what the doc itself claims — the same
> discipline that caught root `index.md` being stale since before
> `case`/`project-portfolio-management` shipped (DOC-1, below). Every
> `AGENTS.md`/`CLAUDE.md`/`AGENTS/*.md` file stays **under 40 KB**
> (none currently exceed it — confirmed 2026-08-04 — so this is a
> guardrail on the edits here, not a current violation to fix). The
> family's own stated anti-pattern is the thing to watch for and
> remove on sight: a hand-maintained table/list that duplicates
> content already accurate elsewhere ("duplicating it here is how the
> two stop agreeing" — `agents/share/overview.md`'s own words, already
> proven true once this pass by root `index.md`).
>
> Batched by subproject family so independent crates can be audited in
> parallel; do **not** audit `link-graph-service-with-loco` while
> LNK-4's T-29..T-33 chain is still landing (active file conflicts) —
> queue it last.

- [x] **DOC-1 (S)** Root-level docs: `index.md` (`README.md` symlinks
  to it), root `AGENTS.md`, root `spec/` (the monorepo-umbrella
  `architecture`/`postgresql` topic dirs + the loose
  `data.md`/`data-modeling.md` files). *Done 2026-08-04* — `index.md`
  rewritten: removed a subprojects table and capability list stale
  since 2026-06-18 (missing `case`, `project-portfolio-management`,
  `link-graph`, all 5 consumer apps, `examples/`, `tutorials/`),
  replaced with pointers to `agents/share/overview.md`/`index.md` (the
  tables that actually stay current) plus a new Examples/tutorials
  section; fixed the "Backend-only Rust services" status line (false
  since the front-ends landed). Root `AGENTS.md` and root `spec/` were
  read during this pass and found current/self-aware (the latter
  already documents its own incomplete promotion status honestly) —
  not rewritten.

- [x] **DOC-2 (L)** Entity + auth service crates' `spec/`, `AGENTS.md`,
  `README.md`/`index.md` against current code: person, worker, place,
  thing, event, course, organization, care-pathway, case,
  project-portfolio-management, authentication-service (11 crates).
  Per crate: confirm the `spec/` §13 task queue reflects what's
  actually merged (not still `[ ]` for shipped work, not silently
  missing recently-landed features like this session's `seed_examples`
  task or the person merge fix); confirm `AGENTS.md` describes the
  crate's *real* capabilities (cross-check against the honest matrix
  in `agents/share/overview.md` rather than restating a possibly-stale
  local claim); confirm `README.md`/`index.md` quick-start commands
  actually run. Fix what's wrong; where a crate's own doc duplicates
  something `agents/share/*.md` already states accurately, prefer a
  link over a second copy (same fix pattern as DOC-1).

  **All 11 crates done 2026-08-04** (person, worker, place, thing,
  organization, course, care-pathway, project-portfolio-management,
  event, case, authentication-service — sub-notes below). Recurring
  finding across the batch: a real, shipped, tested feature (most often
  the keyed-integrity-verification endpoints, sometimes cross-service
  links or FHIR) landed in code with zero `spec`/`AGENTS.md` presence;
  every crate's stale `§13`/`§14`/`§15` narrative text (claims of
  "deferred"/"open"/"planned" for work that had since landed, or vice
  versa) was cross-checked against the code rather than assumed
  current; and, where still on the old pattern, `CLAUDE.md` was thinned
  to the documented one-line `@AGENTS.md` include.

  - [x] **person/person-service-with-loco** *(done 2026-08-04)*.
    `spec/13-tasks.md`: reconciled real drift — T-2 (Fluvio publisher),
    T-3 (FHIR bundle), T-4 (FHIR CapabilityStatement) were still `[ ]`
    though delivered by BUS-3 and T-11 respectively; T-10's own Step
    2/4/5 checkboxes were stale `[ ]` though the work landed under the
    repo `tasks.md` BLK-2/3/4 labels without a pass back through this
    file; T-1c's last open box (a DB-gated activation test) was closed
    by AU-1's `tests/enforcement.rs` with no corresponding task update.
    Added missing task entries for `seed_examples` (EX-4), the
    PERSON-CONTACT-CASE merge/`use_type` write-rejection fix, and AU-1
    (PASETO key rotation + ABAC policy hot-reload) — none had a §13
    entry despite landing this session. Also fixed `spec/14` (open-gaps
    table still listed T-2/T-3/T-4 as open, "15 endpoints" stale to
    35+, FHIR row didn't say Patient-primary) and `spec/16` (OQ-1
    resolved — FHIR Organization lives in organization-service, not
    here). `AGENTS/restful.md`: added three entirely-missing endpoint
    groups (cross-service links, bulk import/export, and
    audit/compliance/erasure — 15 real mounted routes with no table
    entry) and fixed the FHIR table, which still described the
    pre-T-11 non-standard `resourceType: "Person"` prototype instead of
    the shipped `Patient`-primary/`Person`-alias shape.
    `AGENTS/testing.md`: bench/bridge-test counts were off by one file
    / one test. `index.md`: corrected an OpenTelemetry-export
    overclaim and a "Prometheus metrics — future enhancement" claim for
    an endpoint that already ships (both against the honest matrix in
    `agents/share/overview.md`), an "Authentication: planned" claim
    when PASETO verification + the blanket guard were long since done,
    stale test-coverage/version/Project-Structure/Development-Phases
    figures, and ported several curl examples, feature bullets (tax ID,
    documents, emergency contacts, a whole missing "Data Quality &
    Validation" section) and the Merge data-flow step that existed only
    in `CLAUDE.md` and would otherwise have been lost.
    **Cross-cutting structural finding, reported for the other five
    older crates (worker/place/thing/event/course) to cross-check:**
    root `AGENTS.md`'s own "per-subproject docs" table says `CLAUDE.md`
    should be "a one-line `@AGENTS.md` include" — true for the 22 newer
    crates (verified: organization's and care-pathway's `CLAUDE.md` are
    both literally `@AGENTS.md`), but person's `CLAUDE.md` instead held
    a full independent README (Quick Start, Project Structure,
    Configuration, curl examples, …) that near-duplicated `index.md`
    almost section-for-section, *plus* a scattered set of `@`-imports
    (`AGENTS/matching.md`, `AGENTS/models.md`, `agents/share/*.md`,
    …) that `AGENTS.md` itself did not carry — so a Claude Code session
    here was pulling in "read-only reference" content only because it
    happened to be interleaved into the README-shaped file, not because
    the crate's actual working-agreements doc named it. Resolved (not
    just flagged) by moving those `@`-imports into a new "Session
    context (auto-loaded)" section in `AGENTS.md` and reducing
    `CLAUDE.md` to the documented one-liner, after confirming every
    piece of `CLAUDE.md`-only prose content had a home in `index.md`
    (added the few pieces that didn't — see above). No content was
    dropped; the crate now matches the documented convention and the
    newer crates' actual practice.

  - *`worker/worker-service-with-loco` done 2026-08-04.* Found and
    fixed: (1) **duplicate task IDs** in `spec/13-tasks.md` — "T-10"
    named both the cross-service-links task and the workforce-
    assessments task, and "T-11" both bulk import/export and
    `Config::from_env`; renumbered the later pair to T-14/T-15 and
    updated `spec/14-implementation-status.md`'s cross-references
    (CHANGELOG.md left as the historical record, unedited). (2) **T-3
    (FHIR capability statement + bundle) marked wholesale `[ ]`** even
    though `GET /fhir/metadata` and an ad hoc searchset `Bundle` had
    already shipped 2026-07-07 via T-12 — split into done sub-items
    (CapabilityStatement, ad hoc Bundle) vs. remaining (typed
    `Bundle`/`BundleEntry`, `POST`/transaction bundles, Touchstone
    validation); mirrored into §14/§15. (3) **`AGENTS/models.md`
    missing the `Worker.worker_type` field** entirely — a real,
    persisted, Tantivy-indexed, erasure-scrubbed field
    (`src/models/worker.rs`), not scaffolding; added to the field
    table + a new `WorkerType` enum entry. (4) **`AGENTS/restful.md`
    missing the cross-service links endpoints** (`POST`/`GET`/`DELETE
    /api/workers/{pid}/links` + the bulk `GET /api/workers/links`
    pull) even though `spec/09-api-surface.md` §9.1 documents them in
    full and the doc's own header claims to be the "complete endpoint
    reference" — added a Links section mirroring §9.1. (5) **The
    `CLAUDE.md`/`AGENTS.md` split**: root `AGENTS.md` documents
    `CLAUDE.md` as "a one-line `@AGENTS.md` include," but worker's was
    16.5 KB of content (Features/Quick Start/Project Structure/…)
    that was a near-total subset of `index.md` (the real `README.md`
    target, which has every section CLAUDE.md had, plus more) — i.e.
    the same duplicate-copy problem DOC-1 found, not genuinely unique
    content. Cross-checked against the other 10 DOC-2 crates first:
    the newer loco-idiomatic ones (organization, care-pathway, case,
    authentication, project-portfolio-management) **already** carry
    the thin one-line `CLAUDE.md`, and course-service (an older crate)
    has already converted too — only person, worker, place, thing,
    event are still on the old bloated pattern. This confirms the
    family's actual converged answer rather than requiring a fresh
    derivation: **fold nothing** (nothing in worker's CLAUDE.md was
    absent from `index.md`), thin `CLAUDE.md` to `@AGENTS.md`, and
    correct worker's own `AGENTS.md` "doc hierarchy" table, which had
    been asserting the opposite (grouping `CLAUDE.md` with `README.md`
    as "user-facing intro"). The same resolution applies to
    person/place/thing/event when their DOC-2 passes run — no
    independent re-derivation needed. Everything else checked out:
    `AGENTS/matching.md`'s weight/threshold/rule tables verified
    byte-accurate against `src/matching/scoring.rs`; `README.md`
    already symlinks to `index.md` (not a duplicate); the workforce
    assessment capability (worker's most distinctive vs. its siblings)
    is accurately covered end-to-end in spec §5.5/§6.9/§9.2/§10.5 and
    `AGENTS/models.md`/`AGENTS/restful.md`. One pre-existing item
    surfaced but deliberately **not** touched: `src/models/ods.rs` /
    `geography.rs` / `codesystem.rs` plus ~15 matching `src/db/models.rs`
    tables (NHS ODS organization expansion) have real domain models,
    migrations, and SeaORM entities dating to the 2026-06-18 init
    commit, but zero repository/API wiring — `spec/15-roadmap.md`
    already and correctly lists this as unstarted roadmap work, so
    the schema-only scaffolding is not spec drift, just unfinished
    work already honestly described as such.

  - *`place/place-service-with-loco` done 2026-08-04.* Confirmed
    today's QA-SERVER-FIELDS fix (65f83fbb) was already a clean
    three-part landing — `spec/09-api-surface.md` and `CHANGELOG.md`
    both already documented it accurately; nothing to fix there.
    Found and fixed several other real gaps: (1) **`spec/02-scope.md`
    and `spec/09-api-surface.md` both flatly claimed "this crate does
    not expose a FHIR R5 surface / Places are not a FHIR-resource
    concern"** — false; T-11 shipped a full `Location` mapping,
    mounted and routed, on 2026-07-07, and `spec/13-tasks.md` says so
    two sections later in the same file. Fixed both, and added the
    FHIR tier + corrected endpoint count (14 → 16) to §9's surface
    table. (2) **A shipped, tested, reachable feature with zero
    `spec/` presence anywhere**: keyed integrity verification
    (`src/compliance/` — SHA-256 + SHA3-256 digests + an HMAC-SHA256
    MAC via the shared `integrity-mac` crate, `GET /api/records/verify`
    + `GET /api/audit/verify`, landed 2026-07-27/28) had no `spec/13`
    task, no `spec/14` row, and wasn't in `AGENTS/restful.md`'s
    endpoint table or `spec/09` at all — added all three. While
    tracking this down, found `agents/share/overview.md`'s
    `integrity-mac` row was itself stale, naming only "person, worker,
    care-pathway, case" when the capability is now family-wide (all
    ten entity registries + authentication + link-graph, verified by
    directory listing + `git log`) — fixed that one shared line too.
    (3) **Stale test counts everywhere**: `AGENTS/testing.md` said 125
    unit tests, `spec/14-implementation-status.md` said 151,
    `index.md` said "191+" — the live count (`cargo test --lib`) is
    205 (207 incl. 2 DB-gated `#[ignore]`); none of the three matched
    each other, let alone reality. Rewrote `AGENTS/testing.md`'s
    per-module table against a live `--list` run (new modules since
    the 125-test snapshot: `api::rest::auth`, `fhir`, `compliance::*`,
    `config`, `db::outbox`, `relay` — none previously documented) and
    corrected the other two. Also documented three DB/broker-gated
    integration-test files (`api_integration_test.rs`,
    `enforcement.rs`, `fluvio_relay.rs`) that existed but weren't
    mentioned in `AGENTS/testing.md` at all. (4) **`AGENTS/models.md`**
    didn't note which `Place` fields are `#[serde(default)]` /
    server-managed post-QA-SERVER-FIELDS — added a column. (5) **The
    `CLAUDE.md`/`AGENTS.md` split**: same finding and same fix as
    worker's note above (place was one of the five still-bloated
    crates it named) — thinned place's 21 KB `CLAUDE.md` to
    `@AGENTS.md` (nothing in it was absent from `index.md`/`AGENTS/`)
    and corrected place's own "doc hierarchy" table, which had the
    same inverted `CLAUDE.md`-groups-with-`README.md` claim. Matching
    weights/confidence thresholds (`AGENTS/matching.md`) verified
    byte-accurate against `src/matching/scoring.rs`; `README.md`
    already symlinks to `index.md`.

  - *`thing/thing-service-with-loco` done 2026-08-04.* Confirmed
    today's QA-SERVER-FIELDS fix (8fecaac7) was already documented
    accurately in `spec/09-api-surface.md` and `CHANGELOG.md`; added a
    corresponding note to `AGENTS/models.md` (which fields are now
    `#[serde(default)]`) since it had none. Found and fixed several
    other real gaps, the same shapes place and worker's passes turned
    up: (1) **`spec/02-scope.md` claimed "REST API (Axum) + gRPC
    stub"** and both `CLAUDE.md`/`AGENTS.md` claimed "REST + gRPC API"
    — false; there is no `.proto`, no gRPC module, and no server, just
    an unused `tonic` dependency and a `GRPC_PORT` setting nothing
    reads (confirmed against `agents/share/overview.md`'s matrix,
    which already correctly marks thing's gRPC stub `–`). Fixed the
    claim everywhere it appeared (`spec/02`, `spec/08`, `spec/09`,
    `spec/15`, `AGENTS.md`). (2) **A shipped, tested, reachable
    feature with zero `spec/` presence**: row-level integrity digests
    (`src/compliance/mac.rs`+`record_integrity.rs`+`audit_integrity.rs`,
    `GET /api/records/verify` + `GET /api/audit/verify`, landed
    2026-07-28) had no `spec/13` task, no `spec/14` row, no
    `CHANGELOG.md` entry, and wasn't in `AGENTS/restful.md`'s endpoint
    table (which also still listed a phantom `/api/audit/user` that
    doesn't exist in the router) — added all four. (3) **A large,
    previously-undiscovered dead-config surface**: `SERVER_HOST`,
    `SERVER_PORT`, `GRPC_PORT`, `DATABASE_MAX_CONNECTIONS`,
    `DATABASE_MIN_CONNECTIONS`, `OTLP_SERVICE_NAME`, `OTLP_ENDPOINT`,
    `STREAMING_BROKER_URL`, and `STREAMING_TOPIC` all parse into the
    crate's legacy pre-loco `src/config/mod.rs::Config` struct but are
    read **nowhere else** in `src/` — the real bind address/port/pool
    size come from loco's own `config/*.yaml`, and there is no OTLP
    exporter at all (no `src/observability/` module, unlike
    person/worker/event). `CLAUDE.md`'s and `index.md`'s configuration
    tables documented all nine as if live; annotated every one rather
    than deleting the rows outright, since a future pass may choose to
    wire or remove them (flagging, not silently patching, since this
    edges toward a code question). (4) **Stale test count**:
    `spec/14-implementation-status.md` said 141 unit tests; the live
    count (`cargo test --lib`) is 197 — rewrote the Delivered/gap
    tables to add T-7/T-9/T-10/BUS-3/review-queue/QA-SERVER-FIELDS,
    none of which appeared there before, plus a `T-8` (bulk
    import/export) gap row that was missing. Also fixed **T-1**
    ("Production Fluvio publisher"), which was still `[ ]` even though
    BUS-3's `FluvioSink` is exactly that capability, landed under a
    different trait name (`EventSink`, not `EventProducer`) —
    marked `[x]` with a note on what's actually still open (no
    deployment points `THING_FLUVIO_ENDPOINT` at a broker yet). (5)
    **`AGENTS/testing.md`** was missing 4 of the 10 real `tests/*.rs`
    files (`api_integration_test.rs`, `duplicate_detection.rs`,
    `enforcement.rs`, `fluvio_relay.rs`) — added rows for all four. (6)
    **The `CLAUDE.md`/`AGENTS.md` split**: same finding and fix as
    worker's and place's notes above — thinned thing's 8.2 KB
    `CLAUDE.md` to `@AGENTS.md` after first porting its
    Features/Configuration/Security content that wasn't already in
    `index.md` (unlike worker/place, thing's `CLAUDE.md` was not a
    strict subset of `index.md`, so this pass expanded `index.md`
    rather than discarding that content), and corrected thing's own
    "doc hierarchy" table, which had the same inverted
    `CLAUDE.md`-groups-with-`README.md` claim. `AGENTS/matching.md`'s
    weight/confidence tables and its cross-references into the
    *entity-level* `thing/spec/13-tasks.md` (T-8) and
    `thing/spec/16-open-questions.md` (OQ-2) — distinct from this
    crate's own `spec/13`/`spec/16` — verified correct on inspection;
    `README.md` already symlinks to `index.md`.

  - *`organization/organization-service-with-loco` done 2026-08-04.*
    Confirmed `CLAUDE.md` is still the documented thin `@AGENTS.md`
    one-liner (no drift since worker's DOC-2 pass), no `AGENTS/`
    directory (correct for a newer loco-idiomatic crate — §5's domain
    model section is intentionally short because the API DTO **is**
    `organization_matcher::Organization`, nothing forked to describe),
    and `README.md`/`index.md` are two real (non-symlinked) files with
    distinct roles here, not a duplicate pair. Found and fixed several
    real gaps, the largest being that organization is this crate's most
    load-bearing claim to fame — the family's **FHIR R5 reference
    implementation** (`agents/share/fhir.md` §10) — and that fact had
    **zero presence** outside `spec/13`'s own task entry: absent from
    `spec/§2` (Scope), `§6`/`§9` (API surface — no `/fhir/*` row
    anywhere), `§14` (Implementation status), `AGENTS.md`'s endpoint
    table and layout tree, `README.md`, `index.md`, and `CHANGELOG.md`
    (no dated entry at all for the commit that shipped it,
    `57b6c710`, 2026-07-08) — added it to all seven. (2) **A second
    shipped-but-undocumented feature from the same commit**:
    header-based API versioning (`src/version.rs`, `Accepts-version`,
    copied from the event-service reference) had the same zero-presence
    problem in `spec/` and `CHANGELOG.md` (AGENTS.md/README already
    mentioned it) — added a `spec/§13` task, a `§9` paragraph, and the
    missing `CHANGELOG.md` entry (folded into the same FHIR entry,
    since one commit shipped both). (3) **AU-2 (key rotation + ABAC
    policy hot-reload without a restart, 2026-08-01)** — real, wired,
    tested (`tests/enforcement.rs`, `ReloadableVerifier`/
    `ReloadablePolicy`, `spawn_key_refresh`, `spawn_policy_watcher`,
    the `ORGANIZATION_PASETO_KEYS_REFRESH_SECS` env var) — had no
    `spec/13` task entry, wasn't in the `§7` env-var table, and left
    **two now-false claims standing**: `§7`'s own PASETO_KEYS_URL row
    said "no refresh loop" and `§16` carried an open question asking
    whether one was needed, both contradicted by AU-2 landing three
    days later. Added the task, the env var row, and resolved (not
    deleted) the open question with a strikethrough + resolution note,
    matching person's DOC-2 precedent. (4) **A stale duplicate task**:
    `§13` had a `[ ] Bulk import / export — adopt the family contract`
    item describing, verbatim, work already marked `[x]` under **BLK-5**
    earlier in the same section — i.e. the exact "still `[ ]` for
    shipped work" bug this audit exists to catch, self-contradicting
    within one file. Collapsed into a single `[x]` pointer entry with a
    note on why, rather than deleting the history outright. (5) **`§13`
    `seed_examples` (EX-4, 2026-08-04)** was in `AGENTS.md` and
    `CHANGELOG.md` but had no `spec/13`/`§14` entry — added both,
    matching person's/worker's/place's/thing's identical finding for
    their own EX-4/T-N gaps. (6) **`§11` Testing strategy** named only
    2 of the 7 real test binaries — `tests/enforcement.rs`,
    `tests/masking.rs`, `tests/outbox_audit.rs`, `tests/seed_examples_db.rs`,
    and `tests/fluvio_relay.rs` were all missing — added a subsection
    for the five Postgres-gated, own-process binaries (each isolated
    from the others by a process-wide `OnceLock`, same pattern as
    case's `tests/masking.rs`). (7) **`§14` Implementation status**
    was a stale MVP-era summary missing FHIR, privacy/masking/export,
    SEC-M1/SEC-M5, batch dedup + review queue, pagination, AU-2, and
    API versioning — rewritten against the actual `§13` "Done" list
    (187 lib tests, `cargo test --lib -- --list`, live-counted) plus a
    new "Still open" paragraph so the section stops silently
    contradicting `§13`. (8) **`§13`'s "Richer validation" item**
    marked flatly `[ ]` even though SEC-M5 (validated, `[x]`, earlier
    in the same section) already delivers the "identifier formats"
    third of it — changed to `[~]` naming exactly what's done
    (check-digits) vs. still open (URL/country-code format, confirmed
    absent by reading `src/validation.rs`). `§12` Compliance's "honour
    GDPR when the privacy layer lands" was also stale (the privacy
    layer landed 2026-08-01) — updated to state what's live. Everything
    else checked out: `§13`'s BLK-5/Tantivy/merge/masking entries were
    already accurate against `src/`; the env-var table (aside from the
    two fixes above) matched `config/*.yaml` and `src/`; `README.md`'s
    quick-start `DATABASE_URL`/port matched `config/development.yaml`
    exactly.

  - *`course/course-service-with-loco` done 2026-08-04.* Confirmed
    `CLAUDE.md` is already the documented thin `@AGENTS.md` one-liner
    (this is the crate worker's DOC-2 note flagged as already-converted;
    confirmed here rather than re-derived) and `README.md` symlinks to
    `index.md`. Course is the family's only entity with a
    `CourseInstance` sub-resource and a deliberately **non-standard**
    FHIR surface (`/fhir/Basic`, per `agents/share/fhir.md` §3, since no
    FHIR R5 resource models a course) — both were already documented
    accurately in `spec/13`/`AGENTS/restful.md` — but several other real
    gaps of the same "shipped feature with zero spec presence" shape
    surfaced: (1) **`spec/02-scope.md` directly contradicted `spec/13`
    T-20**: it still listed "FHIR resource mapping (no FHIR resource
    fits Course cleanly)" under *out of scope*, even though T-20 shipped
    the non-standard `Basic` wrapper on 2026-07-07 — self-contradicting
    within the same file, the same bug class DOC-2 exists to catch.
    Also claimed "gRPC API (stub only)" — false, there is no
    `tonic`/`prost` dependency at all, not even an unused stub (unlike
    thing, which at least had the dependency); and claimed "Observability
    (tracing + OpenTelemetry OTLP)" as delivered — false, `OTLP_*` parse
    into `Config` but reach no exporter, there is no `src/observability/`
    module. Rewrote §2.1/§2.2 and added §2.1b/§2.1c pointers. (2) **Two
    entirely undocumented, shipped, tested features** — the same
    "zero spec presence" pattern place/thing/organization each found:
    row-level integrity digests + audit-log MAC (`src/compliance/`,
    `GET /api/records/verify`, `GET /api/audit/verify`, landed
    2026-07-28, 11 tests) and header-based API versioning
    (`src/api/rest/version.rs`, `Accepts-version`, landed 2026-07-08
    alongside T-20, 5 tests) — both had **zero** `spec/13` task, no
    `spec/14` row, and the integrity one had no `spec/12` mention either
    (versioning was already in `AGENTS/restful.md`, just not `spec/`).
    Added `spec/13` T-24/T-25 plus an `AU-2` task for key-rotation/policy
    hot-reload (landed 2026-08-01, also missing), `spec/12`, `spec/07`
    env-var tables (`COURSE_INTEGRITY_MAC_KEY[_FILE|_ID|S_RETIRED]`,
    `COURSE_PASETO_KEYS_REFRESH_SECS`), `spec/14`, `spec/15`'s stale v0.4
    entry (still called Fluvio "next" though T-23 shipped it 2026-08-03),
    `spec/09`'s tier table (no FHIR row, no `/api/whoami` or
    `/api/records|audit/verify` mention), `AGENTS/restful.md` (missing
    `/api/whoami` + the two verify endpoints), and `spec/08`'s module
    tree (missing `streaming/`, `privacy/`, `validation/`, `fhir/`,
    `compliance/`, `relay.rs` entirely). (3) **A genuine spec bug, not
    drift**: `spec/06` FR-20 and `spec/04`'s glossary both listed "LMS
    id" as a deterministic identifier scheme that short-circuits
    matching to `1.0` — but `IdentifierType::is_deterministic()`
    explicitly excludes `LmsCourseId` (it has a doctest asserting
    exactly `!LmsCourseId.is_deterministic()`); the deterministic set is
    DOI/Wikidata/**LOM**/OER/URI/UUID, and "LOM" (IEEE Learning Object
    Metadata) appears to have been mistyped as "LMS id" at some point —
    fixed both. (4) **Stale test counts everywhere**: `AGENTS/testing.md`
    said "109+", `spec/11`/`spec/14` said 42 — the live count
    (`cargo test --lib`) is 125 (123 run + 2 DB-gated `#[ignore]`).
    Rewrote `AGENTS/testing.md`'s per-module table against a live
    `--list` run (14 modules were entirely missing, `api::rest::auth`
    alone carrying 25 untabulated tests) and added the two missing
    `tests/*.rs` files (`enforcement.rs`, `fluvio_relay.rs`) that
    `duplicate_detection.rs`/`api_integration_test.rs` overshadowed.
    (5) `spec/10`'s bulk-import design section still said the loco
    `bg_pg` worker feature; this crate is already on loco 1.0.1, which
    renamed it to `worker` (per `CHANGELOG.md` 2026-08-02) — fixed the
    stale name in design prose for code that doesn't exist yet.
    Everything else checked out: `AGENTS/models.md`'s `Course` +
    `CourseInstance` field tables verified byte-accurate against
    `src/models/course.rs`/`course_instance.rs`; `AGENTS/matching.md`'s
    weights (0.35/0.15/0.15/0.10/0.10/0.15) verified against
    `course-matcher`'s `MatchConfig::default`; quick-start commands and
    the docker-compose port mapping (host 8084 → container 8080)
    verified against `config/development.yaml`.

  - *`care-pathway/care-pathway-service-with-loco` done 2026-08-04.*
    Confirmed `CLAUDE.md` is still the documented thin `@AGENTS.md`
    one-liner (no drift), no `AGENTS/` directory (correct for a newer
    loco-idiomatic crate), and `spec/index.md` §12 already states this
    crate's most load-bearing claim — it is the family's **reference
    implementation** of all four control-driving compliance frameworks
    in `agents/share/compliance-for-healthcare.md` §2 — prominently and
    accurately; SOUP/SBOM, profile/terminology validation, and the
    erasure-vs-immutable-chain story were all already correctly
    reflected. What was **not** reflected: two real, shipped, tested
    compliance-critical features had **zero spec presence** anywhere —
    (1) **keyed HMAC integrity MACs + external-witness chain
    checkpoints** (`src/compliance/mac.rs` + `checkpoint.rs`, landed
    2026-07-27, embedding the shared `integrity-mac` crate with
    per-domain HKDF subkeys) closing the one gap the hash chain alone
    cannot see — deletion of its own tail, which leaves no successor to
    break and so verifies perfectly, or vacuously if every row is gone.
    `GET`/`POST /api/compliance/checkpoint{,/verify}` and the
    `integrity_key`/`integrity_resign` CLI tasks existed in code and in
    the family-wide `agents/share/runbooks/integrity-activation.md`
    runbook, but nowhere in this crate's own `spec/§6/§12/§13`,
    `AGENTS.md`, `README.md`, or `index.md` — added a full §12.1
    subsection, a §13 task entry, and endpoint rows/tables to all four
    docs. (2) **AU-2 (key rotation + ABAC policy hot-reload without a
    restart) and pagination**, both landed 2026-08-01 and documented in
    `CHANGELOG.md`, but the spec's own §9 still flatly asserted "No
    refresh loop — periodic re-fetch on key rotation is a future item"
    and §16 carried it as an *open question*, while `auth.rs` had long
    since shipped `spawn_key_refresh`/`spawn_policy_watcher` — the
    exact kind of drift DOC-2 exists to catch, since a reader of the
    spec alone would conclude a rotated key locks out a running
    process. Fixed §9, resolved the §16 open question, and added §13
    task entries for both. `README.md`'s "Status" section had drifted
    furthest: it listed Tantivy search, the Phase-3 Fluvio sink,
    privacy, and the key-refresh loop as **"Deferred"** — all four had
    shipped weeks earlier — while still describing search as `ILIKE`;
    rewrote the whole section against `CHANGELOG.md` and confirmed the
    crate is tagged `care-pathway-service-v0.1.0` (2026-08-04), so §15's
    "still-unreleased" framing was also stale. Same `ILIKE`/cap-50 and
    missing-pagination staleness fixed in `index.md`'s worked-flow
    example and `AGENTS.md`'s/`README.md`'s endpoint tables. `AGENTS.md`'s
    `src/` layout tree was the widest gap of the whole pass — it still
    matched roughly the 2026-07-04 auth-pivot state and named none of
    `src/compliance/` (8 files), `src/fhir/` (4 files), or the
    `insights`/`instances`/`compliance`/`fhir` controllers, the
    `tasks/`/`workers/`/`bulk/` modules, or `version.rs`; rewrote it
    against a live `find src -name '*.rs'`. Cross-checked `cargo test
    --lib -- --list` (246 tests, matching spec's own count claim — no
    drift there) and the `tests/*.rs` file list, adding the four
    request-suite files (`instances.rs`, `insights.rs`,
    `event_outbox.rs`/`outbox_audit.rs`, `enforcement.rs`) §11 didn't
    name. `compliance/lifecycle.md` (crate-root IEC 62304 evidence) was
    already accurate on inspection — no fix needed.

  - *`project-portfolio-management/project-portfolio-management-service-with-loco`
    done 2026-08-04.* Confirmed the one-recursive-`Plan`-collection
    model (`kind` an optional label, never a matching/search gate) is
    described accurately throughout, and `CLAUDE.md` is already the
    documented one-line `@AGENTS.md` include (no fix needed, unlike the
    five older crates worker's pass found). Found and fixed the
    still-`[ ]`-for-shipped-work pattern DOC-2 exists to catch, at
    unusual scale: **spec §13 had CRUD, matching-engine wiring, record
    merge, OpenAPI/Swagger, Prometheus, PASETO auth, and blanket
    enforcement all still checked `[ ]`** despite every one being
    implemented and tested (this crate's original MVP task list was
    never checked off item-by-item as PPM-phase work landed on top of
    it) — marked each `[x]` with an implementation pointer. Two bullets
    each blurred a shipped half with a deferred half under one
    checkbox: "Operational sub-resources — tasks/goals/issues" (tasks
    landed 2026-07-20; goals/issues did not) and "Derived views —
    timeline + burndown" (burndown landed 2026-07-20; timeline did
    not) — split into accurate sub-items. A stale duplicate "Privacy"
    `[ ]` item directly contradicted the dated 2026-08-02 entry above
    it recording the same feature as done — resolved the contradiction.
    **A shipped, tested, reachable feature with zero spec presence
    anywhere**: row-level integrity verification (`src/compliance/` —
    SHA-256 + SHA3-256 digests + a keyed HMAC-SHA256 MAC via the shared
    `integrity-mac` crate, `GET /api/compliance/{records,audit}/verify`,
    landed 2026-07-27/28 alongside the rest of the family's same-day
    integrity rollout) had no `spec/13` task, no `spec/9` endpoint
    entry, and wasn't in `AGENTS.md`'s layout tree or endpoint table —
    added all three. **The bigger structural gap**: spec §9 (this
    crate's API-surface single source of truth) covered only the MVP
    plans-CRUD core plus collaboration/automation/prioritisation
    (§9.4a) — it named **zero** routes for seven other fully-shipped
    route groups (governance/visibility/strategy = PPM Phases A/B/C,
    executive insights, oversight, and the engineering-team core:
    tasks board/sprints/burndown/velocity/standup/DevOps), each with
    its own controller, migration, and dated §13 entry, but never
    folded into §9. Added §9.9–§9.15 (one subsection per area, route
    table + landing date + §13 cross-reference) rather than leaving the
    crate's own "single source of truth" silent on roughly two-thirds
    of its real surface. Mirrored the same gaps into `AGENTS.md`'s
    layout tree (`controllers/{engineering,insights,oversight,
    compliance}.rs`, `src/{engineering,insights,snapshots}.rs`,
    `src/compliance/`, `version.rs`, six migrations were all missing)
    and its endpoint table (added the integrity-verify row). `README.md`
    and `index.md` were both stale in the same three ways: `GET /search`
    still described as `ILIKE` (replaced by Tantivy 2026-08-02),
    goals/issues/timeline presented as live sub-resources/views
    alongside the actually-wired tasks/burndown, and README's own
    "Status" section still listed Tantivy search and privacy as
    **deferred** after both had shipped — rewrote the route tables and
    Status/worked-flow sections, pointing at the new spec §9.9–§9.15
    for the expanded surface rather than duplicating seven route tables
    a third time. Verified `cargo test --lib` (205 passed, matching the
    count spec §13's most recent entries already cite — no drift there)
    and `cargo fmt --check` clean after the doc-only changes.

  - *`event/event-service-with-loco` done 2026-08-04.* Confirmed it is
    genuinely the api-versioning reference `agents/share/api-versioning.md`
    names it: `AGENTS/restful.md` already correctly documents the
    version-free `Accepts-version` header (no stale `/api/v1` in any
    live doc; the `/api/v1` strings that remain are historical, dated
    `CHANGELOG.md` entries, correctly left unedited). Found the same
    "shipped feature with zero spec presence" gap DOC-2 keeps finding:
    **row-level integrity verification** (`src/compliance/` — SHA-256 +
    SHA3-256 digests + a keyed HMAC-SHA256 MAC via the shared
    `integrity-mac` crate, `GET /api/records/verify` +
    `GET /api/audit/verify`, landed 2026-07-28) had no `spec/13` task,
    no `spec/14` row, no `spec/12` compliance-table row, no
    `AGENTS/restful.md` endpoint entry, and no `CHANGELOG.md` entry —
    added all five. Also found and fixed a genuine **superseded-task**
    case distinct from the others' "shipped but undocumented" shape:
    `spec/13` T-4 ("Production Fluvio publisher — implement
    `FluvioEventPublisher : EventProducer`") was still `[ ]`, but the
    real production-delivery need it names was solved a different way
    by T-11 (transactional outbox + relay + `FluvioSink`, done
    2026-08-03) — the literal `EventProducer`/`FluvioProducer` design
    was never built and `FluvioProducer` (`src/streaming/producer.rs`)
    is dead code still carrying its original `todo!()`, unreferenced
    from `AppState` or any router. Marked T-4 done-via-T-11 with the
    dead-code note (left in place — a code deletion belongs to a
    follow-up PR, not a docs audit), the same treatment T-1 already had
    for its own supersession by T-10. `spec/14-implementation-status.md`
    §14.2's gap table was consequently stale in three ways: it listed
    the now-resolved T-1 and T-4 as open gaps and omitted T-9 (bulk
    import/export, still genuinely `[ ]`) entirely — fixed. `spec/16`
    OQ-1 (Encounter vs Appointment) was marked resolved to match T-1's
    own "done, superseded" note. `spec/01`, `spec/02`, `spec/08`, and
    `spec/15` each carried at least one stale "planned"/"stub" claim for
    something already shipped — blanket auth enforcement (spec/01), the
    production Fluvio publisher (spec/02's "out of scope" list),
    `spec/08`'s module-layout tree (still 15 endpoints, missing
    `compliance/`, `relay.rs`, `metrics.rs` entirely, and its trait
    table still said "`EventProducer` | `InMemoryEventPublisher`
    (Fluvio planned)" with no mention of the `EventSink`/`FluvioSink`
    path that actually shipped), and `spec/15`'s roadmap listing
    Prometheus, the FHIR capability statement/Bundle, and Fluvio
    production as all still-future when each had already landed —
    fixed all four. `AGENTS/testing.md` and `spec/11` were missing
    `bridge_bench.rs` from the benchmarks table and
    `tests/enforcement.rs` / `tests/fluvio_relay.rs` from the
    integration-tests list entirely (same shape course's DOC-2 pass
    found) — added. The `CLAUDE.md`/`AGENTS.md` split matched the
    family's already-converged answer (person/worker/place/thing's
    precedent): event's 224-line `CLAUDE.md` was mostly a subset of
    `index.md`, but a real diff pass found several pieces genuinely
    absent from `index.md` — the Location/Party/Offer field detail,
    cross-event links (`Replaces`/`ReplacedBy`/`Refer`/`Seealso`), the
    entire Privacy/Consent section, and the full Validation-rule list —
    folded all of them into `index.md` before thinning `CLAUDE.md` to
    `@AGENTS.md` and fixing `AGENTS.md`'s doc-hierarchy table (same
    inverted `CLAUDE.md`-groups-with-`README.md` claim the other four
    crates had). While folding, also fixed two overclaims already
    present in `index.md` itself (not inherited from `CLAUDE.md`):
    "Prometheus metrics endpoint (future enhancement)" when
    `GET /metrics.prom` was already live, and "Distributed tracing with
    OpenTelemetry" / "OpenTelemetry metrics and traces" as delivered
    when `src/observability/mod.rs`'s OTLP exporter is still a `TODO`
    stub (per the honest matrix in `agents/share/overview.md`) — plus a
    stale "Authentication: planned" line under Security & Compliance
    when PASETO verification + the blanket guard shipped 2026-07-04.
    Everything else checked out: `AGENTS/matching.md`'s weights
    verified byte-accurate against `src/matching/scoring.rs`;
    `AGENTS/models.md`'s `EventType` 29-variant list verified against
    `src/models/mod.rs`; `README.md` already symlinks to `index.md`;
    quick-start commands verified runnable (`.env.example` present,
    `cargo loco start` path matches `AGENTS.md`).

  - *`case/case-service-with-loco` done 2026-08-04.* This crate carries
    an unusually large amount of session history (TUT-1/3/4/6, BUS-1,
    QA-CUST-SQL, COMPOSE-WORKER) and needed the deepest pass of the
    numbered-shape crates so far. Cross-checked the specific items
    flagged for extra care: `examples/policies/closed-case-write-deny.json`
    and `examples/api/case.http`'s status-casing bug are real but live
    in the repo-root `examples/` tree, not this crate — confirmed and
    left untouched, per the note not to "fix" case-service for a
    cookbook issue. The relay is correctly documented as a plain
    background loop (`crate::relay::spawn` from `App::after_routes`),
    **not** conflated with the loco-worker pattern the bulk-job feature
    uses — no fix needed there. `CLAUDE.md` is still the documented
    thin `@AGENTS.md` one-liner. H-5's CHANGELOG-vs-Cargo.toml version
    gap doesn't appear anywhere in `spec/13` as a contradictory claim —
    confirmed, nothing to fix. The real, substantial finding: **an
    entire compliance suite (six landed 2026-07-25..27 features) had
    zero `spec/13` presence and was actively contradicted by `spec/12.0`'s
    own "not yet adopted" list and by `src/compliance/mod.rs`'s own
    module-doc table** — GDPR Art. 17 erasure (`POST /{pid}/erase`),
    row-level `content_hash` record integrity
    (`GET /records/verify`), external-witness chain checkpoints
    (`GET /checkpoint`, `POST /checkpoint/verify`), a keyed HMAC-SHA256
    integrity MAC (default-off, no key ⇒ no MAC), and a CycloneDX SBOM +
    service-identification surface (`GET /api/compliance`,
    `GET /api/compliance/sbom`) were all live, tested (DB-gated
    `tests/requests/cases.rs` erasure/record-integrity suites + DB-free
    unit tests in every `src/compliance/*.rs`), and reachable, but
    `spec/index.md` §12.0's "Not yet adopted" line still listed GDPR
    Art. 17 erasure, row-level integrity, and the SOUP/SBOM bundle as
    outstanding, and `src/compliance/mod.rs`'s own doc comment claimed
    the same. Added a new §12.0.1 documenting all five controls (dates,
    migrations, env vars, endpoints, gating) and corrected the "still
    not adopted" list to what's actually still missing (GDPR
    residency/lawful-basis/Art. 9 declarations; the FHIR **ONC/HTI**
    conformance layer specifically — profile/terminology validation,
    `$validate`, SMART, Bulk Data — not "no FHIR", since the base FHIR
    R5 `Task` CRUD/search surface is itself landed and was *also*
    undocumented in `§9`, fixed alongside). Fixed the source-of-drift
    doc comment in `src/compliance/mod.rs` too (`cargo fmt`/`clippy`
    clean after). Also fixed: (1) a stale `§13` item claiming CI
    "does not pass `--ignored`" — false since 2026-08-01, this crate is
    now enrolled in `ci/db-suites.txt` and the `test-db` stage runs the
    gated suites; marked done with the correction. (2) BUS-1's own
    closing note said "BUS-2 … and BUS-3 … remain" — both landed
    2026-08-03 per the family capability matrix; corrected to name the
    one thing genuinely still open (no deployment points
    `CASE_FLUVIO_ENDPOINT` at a live broker). (3) `§14`/`§15` (current
    implementation status / roadmap) were missing cross-service
    `subject_of` links, FHIR, the durable bus, and the whole compliance
    suite from their "done" summaries — refreshed both. (4) `AGENTS.md`'s
    endpoint table and layout tree were missing roughly a dozen real,
    mounted routes (links, FHIR, all six compliance endpoints) and six
    `src/` modules/`migration/` entries — added. (5) `README.md`'s
    "Status" section still described Tantivy search, the Fluvio sink,
    and privacy as pending/tracked when all three had shipped, and its
    API table still called `?q=` search "case-insensitive" (i.e. the
    pre-Tantivy `ILIKE` behaviour) — rewrote both. (6) `index.md`'s
    worked-flow block was missing the same dozen routes as `AGENTS.md` —
    added. `cargo fmt --check` / `cargo clippy --all-targets` / `cargo
    check --lib` all clean after every edit.

  - *`authentication/authentication-service-with-loco` done 2026-08-04.*
    The 11th and last crate in this batch — the family's central SSO
    provider and reference loco.rs crate, so checked more thoroughly
    than the rest, including a live run (test Postgres on a non-default
    port, `cargo loco db migrate` + `cargo loco start` + real
    `signup`/redeem/compliance-verify/audit-recent curls) rather than
    inspection alone. `CLAUDE.md` reconfirmed already the thin
    `@AGENTS.md` one-liner (no drift); `README.md`/`index.md`'s
    quick-start and dev-console magic-link retrieval both verified live
    and accurate as written.

    Found and fixed several real gaps, same shapes the other ten
    crates' passes turned up: (1) **A shipped, tested, reachable
    feature with zero spec presence**: keyed integrity verification
    (`src/compliance/` — SHA-256/SHA3-256 digests + an HMAC-SHA256 MAC
    over `auth_events`, `GET /api/compliance/audit/verify`, landed
    2026-07-28) had no `spec` entry anywhere, no `CHANGELOG.md` entry,
    and wasn't in `AGENTS.md`'s endpoint/config/layout tables — added
    all of it (spec §6.13/§10/§13/§16; `AGENTS.md` endpoint row,
    config vars, layout tree). (2) **A live, security-relevant doc/code
    mismatch inside the source itself**: `src/controllers/compliance.rs`'s
    own doc comment claimed the endpoint sat "behind the blanket auth +
    ABAC guard when `AUTH_REQUIRE_AUTH` is on" — false, copied unadapted
    from a sibling crate (case-service, which genuinely has that guard);
    this crate has **no blanket `/api/*` guard at all**, and the
    endpoint is live-confirmed reachable with no token
    (`curl localhost:5150/api/compliance/audit/verify` → `200`, no
    PII in the response — row counts/ids only). Fixed the doc comment
    to state the real (unauthenticated) behaviour and added it as an
    explicit open question (spec §16) + an unchecked `§13` sub-task,
    flagged rather than silently gated, since deciding *how* to gate it
    is a code decision outside this pass's scope. (3) **Two internal
    spec self-contradictions**: §5/§10 still called the sessions-table
    idle/absolute-TTL reshape "pending §13" and CSRF "remaining", while
    §13 itself already showed both `[x]` done 2026-07-05 (confirmed
    live against the migration + `is_active` code); and §14/§15 called
    the ABAC HTTP admin-API attribute-assignment surface "deferred"/
    "next" when §13 already showed it `[x]` done the same day — all
    four fixed to match the code and each other. (4) **A live-verified
    factual error**: spec §6.3 flatly asserted magic-link redemption
    "no longer returns a bearer token," but a real redeem response
    (`GET /api/auth/magic-link/{token}`) is
    `{"token":"v4.public…","pid":…,…}` — `views::auth::LoginResponse`
    still carries a transitional bearer token in the body pending
    front-end BFF adoption, exactly as §13's T-12 sub-item already (and
    correctly) said; fixed §6.3 to match, and fixed the struct's own
    stale `RS256 access token` doc comment (RS256 has been decommissioned
    since the PASETO pivot). (5) **`GET /api/auth/audit/recent`'s Auth
    column read "—" (open) in both `AGENTS.md` and `README.md`**, though
    SEC-A2 (2026-07-13, and spec §12) revised it to `access=admin`-gated
    months ago — live-confirmed `401 missing authorization header`
    without a token; fixed both tables, and two further spec cross-refs
    (§6.9, §13/T-9) that still called it "open". (6) Confirmed the
    `.cargo/config.toml` `loco = "run --"` alias genuinely makes
    `cargo loco start`/`db migrate` work when run from inside this
    directory (live-tested), unlike person/worker's older crates
    (TUT-2's finding) — added a clarifying note to `AGENTS.md`'s Run row
    rather than leaving the claim to work by unstated luck. `cargo
    build` / `cargo test --lib` (84 passed) / `cargo clippy --bins` all
    clean after every edit; `Cargo.lock` churn from the local build was
    reverted before committing, untouched.

    **Not fixed, deliberately** — flagged instead of silently patched,
    per this task's own instruction to flag rather than silently change
    code on an ambiguous security question: whether
    `/api/compliance/audit/verify` should require authentication at
    all, and if so how, given this crate has no existing blanket-guard
    mechanism to extend (spec §16 open question, §13 sub-task).

- [ ] **DOC-3 (L)** Matcher crates' `spec/`, `AGENTS.md`,
  `README.md`/`index.md`: person, worker, place, thing, event, course,
  organization, care-pathway, case, project-portfolio-management
  matchers (10 crates). Same audit shape as DOC-2 — these follow the
  §1–§25 SDD shape (distinct from the service crates' §1–§18), confirm
  each still matches that shape and its own `spec/` isn't describing
  scoring rules the code has since changed. Also check rustdoc
  (`///`/`//!`) comments on public scoring functions and types are
  accurate, not just present (`#![deny(missing_docs)]` already
  guarantees presence — accuracy is the gap this checks for). Note for
  `person-matcher`/`worker-matcher` specifically: `Scorer` and
  `Normalizer::phonetic_code` are now also depended on by
  `link-graph-service-with-loco/src/suggest/` (LNK-4, this session) —
  if a doc describes these as used only within their own service's
  matching pipeline, that's now stale.

  - *`person/person-matcher-rust-crate` done 2026-08-04.* `CLAUDE.md`
    was already the thin `@AGENTS.md` one-liner (no drift to fix
    there). Confirmed no doc claims `Scorer`/`Normalizer::phonetic_code`
    are used only within person-matcher's own pipeline — nothing false
    to correct on that specific point, but added the previously-absent
    fact: `link-graph-service-with-loco` (LNK-4) is a real, live
    second consumer (`Cargo.toml` path dep, `src/suggest/` using
    `Scorer::jaro_winkler_similarity`, `Normalizer::phonetic_code`, and
    `Gender`), confirmed via grep. The larger find: **spec was ahead of
    code, not behind it** — `spec/08-domain-model.md` (§8.1, §8.5,
    §8.6a), `spec/12-algorithm-specifications.md` (§12.2), and
    `spec/13-configuration-specification.md` (§13.1) all described
    `relationships`/`tags` fields, a `RelationshipRef`/`RelationKind`
    type, `relationships_score`/`tags_score` breakdown fields, and
    `relationships_weight`/`tags_weight` defaults as if already live,
    while `spec/23-tasks-and-acceptance-criteria.md` correctly lists
    this as open (**T-33**/**T-34**, unchecked) and neither the field,
    the type, nor the weight exists anywhere in `src/` (confirmed by
    grep — zero hits). Fixed all four sections to mark the feature
    explicitly **planned, not yet implemented**, cross-referencing
    T-33/T-34, rather than silently deleting the design content (it's
    legitimate forward design, just mis-shelved as current behaviour).
    Also found and fixed two rustdoc-adjacent drifts of the "code
    changed, doc didn't" kind DOC-3 is chiefly aimed at:
    `AGENTS/matching-algorithm.md`'s "Deterministic Logic" quick-view
    section listed only 6 of the 42 identifier schemes and omitted
    passport-book agreement entirely (the "Full Branch List" appendix
    lower in the same file was already complete and correct — only the
    top summary was stale), and its "Component Scoring — Full Table"
    likewise stopped at 12 of 42 schemes with no note; both now state
    the true scope. `AGENTS/architecture.md`'s "Public Surface" code
    block was missing `BloodType`/`PassportBook` from the `models`
    re-export list (both real, both in `lib.rs` since T-26/T-29+), and
    its "God modules" guidance cited a stale "~1,000 lines" split
    threshold that `matcher.rs` (3,455 lines) and `identifiers.rs`
    (4,058 lines) have long since passed by design (42-scheme
    boilerplate reads best kept together) — reworded rather than
    picking an equally-arbitrary new number. `AGENTS/testing.md` had
    one stale "(planned, see spec §18.4)" marker on property-based
    tests that the same file's own "Property Tests" section, twenty
    lines down, already correctly marks delivered (T-6 ✅) — fixed the
    stale marker. Verified clean: `cargo test` (417 lib + 233
    integration + 176 doctests + 13 adapter-contract + 11 property, all
    passing), `cargo test --doc`, `cargo clippy --all-targets -- -D
    warnings`, `cargo run --example basic_usage`; `README.md` confirmed
    a real symlink to `index.md`; `index.md`'s weight table (30/20/20/
    15/5/5/5/5) verified byte-accurate against `MatchConfig::default()`
    in `src/matcher.rs`; `AGENTS/national-person-identifiers.md`'s
    "42-scheme parser reference" section (the canonical one) checked
    accurate — its separate, older 14-row background table at the top
    of the same file is pre-existing research context, not a claim
    about the 42 implemented schemes, and was left alone. All edited
    files re-confirmed under 40 KB.

  - *`care-pathway/care-pathway-matcher-rust-crate` done 2026-08-04.*
    `spec/index.md` (this crate's single-file §1–§25 shape — confirmed
    it, not a split-file layout) had a genuine **internal
    self-contradiction**, the same failure class DOC-2 kept finding in
    the service crates: §6 (Domain model), §7 (Configuration), §13.1,
    §13.2, and the §5 algorithm-overview diagram all described
    `tags`/`relationships` (`CarePathway.tags`, `CarePathway.
    relationships`, `MatchConfig::relationships_weight`/`tags_weight`,
    `MatchBreakdown::relationships_score`/`tags_score`,
    `RelationshipRef`/`RelationKind`) in the present tense as if
    already shipped, while §21's re-export list even claimed
    `RelationshipRef`/`RelationKind` were part of the live `lib.rs`
    contract — none of it exists in `src/` (verified by grep across
    every `src/*.rs` and both test files: zero hits), and §23's own
    task queue correctly lists implementing both as still `[ ]`,
    matching a `CHANGELOG.md [Unreleased]` entry that says outright
    "Code implementation is tracked in spec §23." Fixed by annotating
    §5/§6/§7/§13.1/§13.2/§21 as **planned, not yet implemented**
    (cross-referenced to §23) rather than deleting the design content —
    consistent with this crate's own `AGENTS/spec-driven-development.md`
    policy ("when spec and code disagree, the spec is right... open a
    task in §23," not silently rewritten to match code), and with the
    ground rule to flag rather than quietly change scoring behaviour
    since implementing the feature is a separate three-part PR. Every
    other section checked out against the code: the six live components'
    weights (name 0.30 / condition 0.25 / pathway_code 0.15 /
    care_setting 0.10 / interventions 0.10 / keywords 0.10, threshold
    0.85, strict/lenient 0.95/0.70), the three deterministic
    short-circuits (R-0 identifier schemes, R-1 provider+pathway_code,
    R-2 same_as), and the SEC-M2 empty-code guard are all byte-accurate
    against `src/config.rs`/`src/matcher.rs`. Rustdoc spot-check
    (`lib.rs`, `matcher.rs`, `scoring.rs`, `care_pathway.rs`,
    `config.rs`, `normalize.rs`, `phonetic.rs`, `error.rs`, `main.rs`)
    found no stale `///`/`//!` comments — every one matches current
    behaviour. `CLAUDE.md` is already the documented one-line
    `@AGENTS.md` include (no drift to fix, unlike the six older service
    crates DOC-2 found). `README.md`/`index.md`/`AGENTS.md`/`AGENTS/
    *.md` needed no changes — none of them mention `tags`/
    `relationships` and their usage examples/weight tables already
    matched the code. Verified live: `cargo test` (10 unit-test modules
    + 10-test `tests/public_api.rs` + 4-test `tests/property_tests.rs`
    + 7 doctests, all green), `cargo clippy --all-targets -- -D
    warnings` clean, `cargo fmt --check` clean, and the sibling
    `care-pathway-service-with-loco/tests/matching.rs` bridge test
    §20 claims exists (read-only confirmation only — that crate is out
    of this task's scope). Did not touch
    `care-pathway-service-with-loco` per this task's boundary.

  - [x] `case/case-matcher-rust-crate` done. Found and fixed a real
    spec/code mismatch, not a doc-only staleness: `spec/index.md` §5
    (algorithm overview), §6 (domain model), §7 (configuration), §13a
    (relationships), §13b (tags), and §21 (compatibility/re-exports)
    described a `tags` field/`tags_score`/`tags_weight` and a
    `relationships` field/`RelationshipRef`/`RelationKind`/
    `relationships_score`/`relationships_weight` as though already
    implemented — none of them exist in `src/case.rs`, `src/config.rs`,
    `src/scoring.rs`, or `lib.rs`'s re-exports (confirmed by grep + read
    of every `src/*.rs`). §23's own task list correctly already listed
    implementing both as open work, so the crate's spec-first
    discipline was being followed but the normative sections read as
    present-tense fact with no cross-reference to §23. Annotated all six
    sections "planned — see §23" / "not yet implemented" (kept the
    design content per "spec is right, don't silently launder"; did not
    invent code) and corrected §21 to drop `RelationshipRef`/
    `RelationKind` from the *current* re-export list (that part was a
    flat factual error, not just forward-looking). Also added the
    SEC-M2 trivial-identifier/trivial-`same_as` skip behaviour to §15/
    §16 and `AGENTS/matching-algorithm.md` (landed per `CHANGELOG.md`
    `[Unreleased]`, previously undocumented in spec), and the `fuzz/`
    cargo-fuzz harness (SEC-I2, also in `CHANGELOG.md` `[Unreleased]`)
    to spec §24 and `AGENTS/testing.md`. Verified: `CaseStatus` has no
    `#[serde(rename_all)]` and nothing in spec/README/index/AGENTS
    claims a lowercase wire form — spec §6 correctly states bare
    PascalCase (`"Docket"`, `"InProgress"`). `CLAUDE.md` was already
    the thin `@AGENTS.md` include (no DOC-2-style drift to fix).
    `README.md`/`index.md` are accurate, not symlinks, and all 7
    doctests + `cargo test --doc` pass. `AGENTS/*.md` (index,
    matching-algorithm, normalization, spec-driven-development, testing)
    were otherwise accurate against the live code. `cargo test`,
    `cargo clippy --all-targets --all-features -- -D warnings`, and
    `cargo fmt --check` all clean (docs-only change; no `src/` edits).
    All touched files stay well under 40 KB (`spec/index.md` 13.9 KB
    after edits).

  - [x] `organization/organization-matcher-rust-crate` done. Same bug
    class as the care-pathway and case matcher notes above (this
    template's tags/relationships drift is now confirmed systemic, not
    a one-off): `spec/index.md` §5 (algorithm overview), §6 (domain
    model), §7 (configuration), §14a (relationships), §14b (tags), and
    §21 (compatibility/re-exports) described `Organization.tags`,
    `Organization.relationships`, `RelationshipRef`/`RelationKind`,
    `MatchConfig::relationships_weight`/`tags_weight`, and
    `MatchBreakdown::relationships_score`/`tags_score` in the present
    tense as already shipped — confirmed absent from every `src/*.rs`
    (`organization.rs`, `config.rs`, `scoring.rs`) and from `lib.rs`'s
    actual re-export list by grep + read. §23's own task queue already
    correctly listed both as open `[ ]` work, and the `CHANGELOG.md
    [Unreleased]` entry for relationships was already honestly titled
    "spec-only" — the drift was confined to §5/§6/§7/§14a/§14b/§21
    reading as normative present-tense fact with no cross-reference to
    §23. Annotated all six "planned, not yet implemented — see §23"
    (design content kept, not deleted, per this crate's own
    `AGENTS/spec-driven-development.md` "spec is right, don't launder a
    gap into code" doctrine) and corrected §21 to the real `lib.rs`
    list (dropped the false `RelationshipRef`/`RelationKind`/
    `relationships_score`/`tags_score` claims). Also found and fixed a
    second, independent gap: `spec/§24` (Testing strategy) and
    `AGENTS/testing.md` named only unit tests, `tests/public_api.rs`,
    and doctests — both missing the `proptest` property suite
    (`tests/property_tests.rs`, SEC-M6, 7 tests) and the `cargo-fuzz`
    harness (`fuzz/`, SEC-I2, 2 targets), both already shipped earlier
    in the same `CHANGELOG.md [Unreleased]` section; added both to
    spec §24 and a new "Property-based tests" / "Fuzzing" pair of
    sections in `AGENTS/testing.md`. Checked SEC-M5 (LEI ISO 7064 MOD
    97-10 / GLN GS1 mod-10 check-digit validation) specifically per
    this task's prompt: it lives entirely in the sibling
    `organization-service-with-loco`'s `src/validation.rs`, not in this
    matcher crate at all (confirmed by grep — zero hits for
    check-digit/MOD-97/GS1 anywhere in `src/`, and
    `tests/property_tests.rs`'s own module doc says so explicitly), so
    there was nothing to document here; noted in `CHANGELOG.md` so a
    future pass doesn't go looking for it in the wrong crate.
    Everything else checked out: `CLAUDE.md` is already the documented
    one-line `@AGENTS.md` include (no drift, unlike the six older
    *service* crates DOC-2 found — this newer matcher never had the
    bloated version); `AGENTS.md`, `AGENTS/matching-algorithm.md`,
    `AGENTS/normalization.md`, and `AGENTS/spec-driven-development.md`
    (already harmonized against course-matcher residue by an earlier
    session per `CHANGELOG.md`) are all byte-accurate against
    `src/matcher.rs`/`src/normalize.rs`/`src/config.rs`; `README.md`
    and `index.md` are two real, non-symlinked files, mention neither
    tags nor relationships, and `index.md`'s hand-computed worked
    example (name 0.950/address 1.000/url 0.988/founding 0.500/
    keywords 0.333 → 0.849 `Medium`) was independently re-verified
    against a scratch integration test calling the real engine — exact
    match, not rounded coincidentally. Rustdoc spot-check across
    `lib.rs`/`matcher.rs`/`scoring.rs`/`organization.rs`/`config.rs`/
    `normalize.rs`/`phonetic.rs` found no stale `///`/`//!` comments.
    Verified live: `cargo test --lib` (45 passed), `cargo test --test
    public_api` (11), `cargo test --test property_tests` (7), `cargo
    test --doc` (7), `cargo clippy --all-targets -- -D warnings`
    clean, `cargo fmt --check` clean. Did not touch
    `organization-service-with-loco` per this task's boundary. All
    touched files stay well under 40 KB.

  - [x] **`project-portfolio-management/project-portfolio-management-matcher-rust-crate`
    done 2026-08-04 — the 10th and last crate in this batch.** Unlike
    the case/care-pathway/organization matcher notes above, this crate
    had **no** tags/relationships spec-vs-code drift — §13.1/§13.2
    (relationships/tags) already correctly describe them as
    implemented, matching `src/matcher.rs`/`src/config.rs` exactly
    (verified byte-accurate: nine weights sum to 1.0, threshold 0.85,
    strict/lenient 0.95/0.70). This crate's own §23 already flags
    `spec/index.md` as a single un-split file rather than the sibling
    matchers' numbered-file layout — a known, honestly-declared
    deviation, not a fresh finding. What was found and fixed: (1) a
    **stale rustdoc comment** in `tests/property_tests.rs`'s module
    doc claiming "the kind gate pins any cross-kind pair to a 0.0
    non-match" — the exact opposite of current behaviour (the kind
    gate was removed 2026-07-20; the property itself is named
    `different_kinds_are_not_gated` and asserts no gate fires), fixed
    to state the true "no kind gate" invariant. (2) The matching
    **`CHANGELOG.md` SEC-M6 bullet**, which still described a
    `kind_gate_blocks_all_cross_kind_pairs` property as if it were the
    current test — annotated as superseded by the 2026-07-20 kind-gate
    removal rather than rewriting history. (3) `src/lib.rs`'s crate-doc
    strategy summary said "parent portfolio" for the timeframe/parent
    component list — stale `portfolio_ref`-era wording; corrected to
    "parent plan (`parent_ref`)". (4) **`AGENTS/testing.md` and
    `spec/index.md` §23/§24 didn't mention the `proptest` suite
    (`tests/property_tests.rs`, SEC-M6, 6 tests) or the `fuzz/`
    cargo-fuzz harness (SEC-I2, 2 targets) at all** — both real,
    shipped, and already in `CHANGELOG.md [Unreleased]` — added a
    "Property-based tests" / "Fuzzing" section pair to
    `AGENTS/testing.md` and corresponding §23 task entries + an updated
    §24/status-line test count in `spec/index.md` (55 unit + 10
    integration was stale; live count is 57 unit + 10 integration + 6
    property + 7 doctests). (5) `AGENTS.md`'s file-layout table said
    `normalize.rs` holds "fold, code, fold_set" — incomplete; it also
    exports `url` and `iso_date_to_days` (both load-bearing, R-2 and
    the timeframe component respectively) — added. `CLAUDE.md` was
    already the documented thin `@AGENTS.md` one-liner (no drift to
    fix). `README.md`/`index.md` are two real, non-symlinked files
    (consistent with case/care-pathway/organization-matcher's same
    choice, not the person-matcher symlink — accepted family drift);
    their usage snippets and the 9-row weight table are accurate and
    all 7 doctests pass. Rustdoc spot-check across
    `lib.rs`/`matcher.rs`/`scoring.rs`/`plan.rs`/`config.rs`/
    `normalize.rs`/`phonetic.rs`/`error.rs` found no other stale
    `///`/`//!` comments. Verified live: `cargo test` (57 unit + 10
    `public_api` + 6 `property_tests` + 7 doctests, all green),
    `cargo clippy --all-targets --all-features -- -D warnings` clean,
    `cargo fmt --check` clean, `cargo run` demo output matches its
    described behaviour. Did not touch
    `project-portfolio-management-service-with-loco` per this task's
    boundary. All touched files stay well under 40 KB.

  *(As of this sub-note: person, care-pathway, case, organization, and
  project-portfolio-management matchers are done under DOC-3; worker,
  place, thing, event, and course matchers were still in flight
  concurrently and had not yet landed their own sub-notes above at the
  time this note was written — re-check before ticking the parent
  DOC-3 box.)*

  - *`course/course-matcher-rust-crate` done 2026-08-04.* Same
    tags/relationships spec-ahead-of-code pattern found independently
    in care-pathway-matcher and case-matcher above, confirmed by grep
    across every `src/*.rs`: spec §5 (algorithm overview, plus §5.1/
    §5.2), §6 (domain model, plus §6.1/§6.2), §7 (configuration), and
    §13a (tags) all described `Course.relationships`/`Course.tags`,
    `RelationshipRef`/`RelationKind`, `MatchConfig::relationships_weight`/
    `tags_weight`, and `MatchBreakdown::relationships_score`/`tags_score`
    in the present tense as already-shipped algorithm components — none
    exist in `src/course.rs`, `src/config.rs`, `src/scoring.rs`, or
    `src/matcher.rs`. §23's own task queue already correctly lists both
    as open (`T-11` relationships, `T-12` tags), and `CHANGELOG.md`
    `[Unreleased]` says outright the tags addition was "Specced… code
    follow-up tracked as §23 T-12" — so, as with the two sibling
    crates, the spec-first discipline was followed but the normative
    sections gave no in-line signal a reader wouldn't get without also
    checking §23. Fixed by annotating §5/§5.1/§5.2/§6/§6.1/§6.2/§7/§13a
    (plus the §13a ToC entry in `spec/index.md`) "planned, §23 T-11/
    T-12 — not yet implemented," keeping the design content rather than
    deleting it, and restating the shipped six-component pseudocode/
    field list/weight table as what `match_courses` actually runs
    today. Also confirmed the sibling concern this task flags by
    name — whether spec §15's deterministic-identifier list mistypes
    "LOM" as "LMS id" (a real bug found elsewhere in the family this
    session, per `course-service-with-loco`'s own DOC-2 pass) — is
    **not** present here: `spec/15-identifier-short-circuits.md`
    correctly lists `Doi`/`Wikidata`/`Lom`/`Oer`/`Uri`/`Uuid`, matching
    `IdentifierScheme::is_deterministic` in `src/course.rs` exactly (the
    provider-scoped `LmsCourseId` variant is separately, correctly
    listed as NOT deterministic). Separately found and fixed a stale
    test count: `index.md` and `AGENTS/testing.md` both said "76 unit
    tests," but `cargo test --lib -- --list` reports 78 (confirmed by
    direct `#[test]` grep across `src/*.rs` too); also added the
    previously-undocumented `tests/proptests.rs` (6 SEC-M6 property
    tests) and the `fuzz/` cargo-fuzz harness (two libFuzzer targets,
    SEC-I2) to `index.md`, `AGENTS/testing.md`, and spec §24 — both
    shipped per `CHANGELOG.md` `[Unreleased]` with zero prior mention
    in any doc. Rustdoc spot-check (`lib.rs`, `matcher.rs`, `scoring.rs`,
    `course.rs`, `config.rs`, `normalize.rs`, `phonetic.rs`, `error.rs`)
    found no stale `///`/`//!` comments; `IdentifierScheme` (12
    variants), `EducationalLevel` (12 + `Custom`), and
    `LearningResourceType` (11 + `Custom`) variant counts in spec §6 all
    match the code exactly. `CLAUDE.md` is already the documented
    one-line `@AGENTS.md` include (no DOC-2-style drift to fix).
    `README.md` is a real symlink to `index.md`; both `index.md` code
    blocks (deterministic + probabilistic worked examples) use real
    public-API calls and were hand-verified against `matcher.rs`/
    `scoring.rs`. `AGENTS/index.md`, `AGENTS/normalization.md`, and
    `AGENTS/spec-driven-development.md` needed no changes — none
    mention `tags`/`relationships` and stayed accurate throughout.
    Verified live: `cargo test --lib` (78 passed), `cargo test --doc`
    (13 passed), `cargo test --test public_api` (16 passed),
    `cargo test --test proptests` (6 passed), `cargo fmt --check`
    clean, `cargo clippy --all-targets -- -D warnings` clean. Did not
    touch `course/course-service-with-loco` (sibling crate, out of this
    task's boundary). All edited files re-confirmed well under 40 KB.

  - *`worker/worker-matcher-rust-crate` done 2026-08-04.* `CLAUDE.md`
    already the thin `@AGENTS.md` one-liner (no drift to fix). Checked
    the `link-graph-service-with-loco` LNK-4 note directly rather than
    assuming symmetry with person-matcher's finding: `worker-matcher`
    is genuinely **not** a link-graph dependency (its `Cargo.toml`
    comment explicitly says "depended on (not `worker-matcher`)"
    because `person-matcher`'s `Scorer`/`Gender` alone cover both sides
    of the person/worker comparison, confirmed via grep of
    `src/suggest/mod.rs` — zero `worker_matcher` references) — so no
    doc change was needed on that point, and none was made. Found the
    **same relationships/tags spec-ahead-of-code drift** person-matcher's
    note above describes, independently in this crate:
    `spec/08-domain-model.md` (§8.1, §8.5, §8.6a), `spec/12-algorithm-
    specifications.md` (§12.2), and `spec/13-configuration-specification.md`
    (§13.1) all described `relationships`/`tags` fields, a
    `RelationshipRef`/`RelationKind` type, `relationships_score`/
    `tags_score` breakdown fields, and `relationships_weight`/
    `tags_weight` defaults as if already live, while `spec/23-tasks-
    and-acceptance-criteria.md` correctly lists this as open (**T-33**/
    **T-34**, unchecked) and neither the field, the type, nor the
    weight exists anywhere in `src/` (confirmed by grep — zero hits).
    Fixed all four sections to mark the feature explicitly **planned,
    not yet implemented**, cross-referencing T-33/T-34, rather than
    deleting the design content. Also fixed a genuine rustdoc-accuracy
    gap of the kind this task's checklist item 2 targets:
    `MatchingEngine::deterministic_match`'s doc comment enumerated only
    12 of the 42 national-identifier schemes under "Returns `true` iff
    **any** of the following hold," reading as an exhaustive list when
    `deterministic_identifier_match`'s `schemes` slice in the same file
    covers all 42 — reworded to state the true scope and point at
    `AGENTS/matching-algorithm.md`'s full branch table. Fixed a second
    stale rustdoc claim: `src/normalizer.rs`'s module doc said email
    and middle-name normalisation were out of scope "(see spec tasks
    T-11 and OQ-1 respectively)," but both shipped long ago
    (`Normalizer::normalize_email` is implemented and tested;
    `middle_name` runs through the same `Normalizer::normalize_name`
    path as given/family names) — corrected. Fixed a wrong filename in
    `spec/23` T-17.1: it pointed at `AGENTS/national-worker-
    identifiers.tsv`, which doesn't exist — the real file is
    `AGENTS/national-person-identifiers.md`'s companion
    `national-person-identifiers.tsv` (confirmed the 7 T-17.1 scheme
    rows are indeed still missing from it, so the task itself stays
    open, just correctly named now). Checked `CHANGELOG.md`
    `[Unreleased]` against spec per this task's item 1 and found three
    landed-but-undocumented items: the `fuzz/` cargo-fuzz harness
    (SEC-I2), the SEC-M2/M3 false-identity-match guards, and the
    `.github/workflows/spec-drift.yml` T-7 CI check — none appeared in
    `spec/18-testing-strategy.md`, `spec/19-build-tooling-and-release.md`,
    or `spec/20-security-privacy-and-compliance.md` despite all three
    being real, shipped, and (for T-7) actively enforcing the spec-first
    discipline on every PR; added §18.6, §19.4, and a security
    paragraph respectively. `index.md` had the more concrete bug this
    task's item 4 exists to catch: three of its "Basic Example" /
    "Configurable Matching" / "Detailed Match Breakdown" code blocks
    called `.nhs_number(...)`, `nhs_number_weight`, and
    `.nhs_number_score` — none of which exist (the real names are all
    `uk_nhs_number*`, confirmed by grep of `src/models.rs` and
    `src/matcher.rs`) — so those snippets would not compile; fixed all
    three plus the `Cargo.toml` version pin (`"0.1.0"`, four majors
    stale against the real `0.6.1`). Two more sections of the same file
    flatly contradicted *other sections of the same file*: "Limitations"
    claimed "Single Identifier Scheme: … other national identifier
    schemes are not currently validated" and "No Batch Processing:
    Processes pairs of workers," while the "Features" list eleven lines
    above already documents 42 validated schemes and the file's own
    "## Batch Scoring" section below demonstrates `match_one_to_many`/
    `rank_one_to_many`; and "Future Enhancements" still listed "Support
    for other national identifiers," "Batch matching API," and
    "Performance benchmarks" as open `[ ]` TODOs when all three ship
    (42 schemes, the batch API, and `benches/match_pair.rs` criterion
    benchmarks — all documented elsewhere in the same file). Rewrote
    both sections to state current reality and point at what's
    genuinely still open (T-33/T-34, T-9.1, phone-country breadth).
    Also refreshed the stale "Worker Data Model" / "Matching Algorithm"
    sections (single-scheme-era content predating the 42-scheme
    expansion) to summarise the real field groups and link the
    authoritative spec tables rather than re-duplicating them. Verified
    clean: `cargo build`, `cargo test --lib` (417 passed), `cargo test`
    (176 doctests, including the edited `deterministic_match` example),
    `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
    Did not touch `worker/worker-service-with-loco` or
    `link/link-graph-service-with-loco` (out of this task's boundary).
    All edited files re-confirmed under 40 KB.

  - *`place/place-matcher-rust-crate` done 2026-08-04.* `CLAUDE.md`
    already the thin `@AGENTS.md` one-liner (no drift to fix).
    Rustdoc/index.md/AGENTS quick-start examples spot-checked by
    actually compiling and running them (temp `examples/*.rs`, deleted
    after, `git status` confirmed clean) — all matched the code exactly
    except the biggest finding: **spec §3.1/§6.10/§6.11/§7 described a
    `setting`/`tags` matching component (fields, scorer algorithms,
    `MatchConfig` weights, `MatchBreakdown` scores) as if shipped**,
    but `src/models.rs`/`src/matcher.rs` have neither — `CHANGELOG.md`'s
    own "Unreleased" section already (correctly) calls this
    "implementation pending," but the spec's normative sections never
    said so, so `Place` was documented as "17 fields" when the real
    struct has 15, `MatchBreakdown` as "11 score fields" vs the real 9,
    and a `PlaceBuilder` `tags()`/`add_tag()` API that doesn't exist —
    exactly the self-contradiction-within-the-repo shape this audit
    exists to catch, and directly against this crate's own spec §9.3
    rule ("when spec and code disagree, the code is what shipped —
    update the spec to match, flag the divergence in the CHANGELOG").
    Fixed by demoting the design to a clearly marked "planned, not
    implemented" subsection (new §3.1.3) in `spec/03`, `spec/06`,
    `spec/07`, and fixing the same stale "17 fields" copy in
    `spec/12-glossary-cross-reference.md`; the design itself was kept,
    not deleted, since it's real forward work tracked in the
    CHANGELOG — flagging per this task's instruction rather than
    silently dropping it. Second finding, verified by actually running
    the numbers: worked example §11.1 (Eiffel Tower / La Tour Eiffel)
    claimed `name_score ≈ 1.00` ("the alternate matches exactly") and a
    renormalised score of `≈ 0.984`; the real computed values are
    `name_score ≈ 0.84` (`"la tour eiffel"` vs `"tour eiffel"` is not an
    exact match) and `score ≈ 0.938` — fixed the arithmetic and the
    claim; §11.2/§11.3 were independently verified correct via the same
    compile-and-run method, no changes needed. Third: `scripts/
    spec-drift-check.sh` still grepped `git diff --name-only` for an
    exact-match file literally named `spec.md`, a leftover from before
    the single-file-to-directory migration — since that filename can
    never appear in a diff again, the CI drift gate would silently
    FAIL every legitimate PR that updates `src/matcher.rs` and the
    correct `spec/*.md` file together (it would misreport the spec as
    unsynced) — fixed to match on the `^spec/` path prefix, and fixed
    the same script's own stale `spec.md §23 T-7` comment (§23 doesn't
    exist; `AGENTS/spec-driven-development.md` explicitly names this
    exact stale-reference pattern as a thing to fix on sight). Fourth:
    `AGENTS/release.md` and `AGENTS/security-and-privacy.md` both told
    contributors to run `cargo audit` before release, but the crate's
    actual CI gate (`.github/workflows/security.yml`) runs `cargo deny
    check` instead (superseding `cargo audit` per that workflow's own
    comment) — updated both, and added the missing checklist line.
    Fifth: `mimalloc` is a real `Cargo.toml` runtime dependency (used by
    `src/main.rs`'s musl-gated global allocator) absent from both
    `AGENTS/coding-style.md`'s and `AGENTS/security-and-privacy.md`'s
    "current direct runtime dependencies" lists — added, with the
    demo-binary-only caveat. Sixth: `fuzz/` (SEC-I2, 3 libFuzzer
    targets) and `deny.toml` (SEC-I1) are real, CI-wired additions with
    zero mention anywhere in `AGENTS.md`/`AGENTS/testing.md` — added a
    Fuzzing section to `AGENTS/testing.md` and both to `AGENTS.md`'s
    file-layout tree (which also still said `spec.md` there — fixed to
    `spec/`). Verified clean throughout: `cargo fmt --check`, `cargo
    clippy --all-targets -- -D warnings`, `cargo test` (163 lib + 12
    adapter-contract + 37 integration + 11 property + 97 doctest, all
    passing). Did not touch `place/place-service-with-loco` (sibling
    crate, out of this task's boundary). All edited files re-confirmed
    well under 40 KB.

  - *`thing/thing-matcher-rust-crate` done 2026-08-04.* This crate's
    spec follows its own §1–§13 shape (not the family's typical §1–§25
    for matchers — `AGENTS/spec-driven-development.md` states this
    explicitly, "the spec runs §1–§13," and it is internally
    consistent, so nothing to fix there). `CLAUDE.md` was already the
    thin `@AGENTS.md` one-liner (no DOC-2-style drift). Found the same
    tags/relationships spec-ahead-of-code pattern as the case/
    care-pathway/organization/course/person/worker matcher notes above,
    confirmed independently by grep across every `src/*.rs` (zero
    hits): `spec/03-data-model.md` (§3.1, §3.3.1, §3.4, §3.7),
    `spec/05-matching-engine.md` (§5.9.1, §5.9.2, §5.10), and
    `spec/06-per-field-scoring-algorithms.md` (§6.6, §6.8) described a
    `relationships: Vec<RelationshipRef>` field, `RelationshipRef`/
    `RelationKind` types, a `tags: Vec<String>` field,
    `relationships_score`/`tags_score`, and `relationships_weight`/
    `tags_weight` as though already shipped; `CHANGELOG.md
    [Unreleased]` already honestly labels both "spec-only... Code
    follow-up (not yet implemented)," but the spec sections themselves
    carried no in-line signal. Annotated all eight sections
    "**Not yet implemented**" (kept the design content, not deleted)
    and added **OQ-E** to `spec/10-open-questions.md` — this crate has
    no `§13`-as-tasks convention (§13 here is References; its own
    `AGENTS/spec-driven-development.md` says outstanding work
    consolidates into §10), so OQ-E is now the load-bearing pointer
    back to the CHANGELOG follow-up. Fixed a genuine **internal
    self-contradiction**: `spec/03-data-model.md` §3.7's own Rust code
    block already correctly showed the real 10-field `MatchBreakdown`
    (matching `src/matcher.rs` exactly), while the prose two lines
    below asserted "carries **12** score fields" — the two disagreed
    with each other, not just with the code. Fixed a **factual error**
    in §7.3: it claimed `MatchBreakdown` (along with `Thing`) is
    `#[non_exhaustive]`, but grepping `src/*.rs` shows only `Thing` and
    `MatchingError` carry the attribute (confirmed against this same
    crate's own `AGENTS.md`/`AGENTS/architecture.md`, which already
    stated the correct two-item list) — means adding
    `relationships_score`/`tags_score` later will actually be a
    struct-literal-breaking change, now stated. Fixed a **stale
    cross-reference** in `spec/04-normalisation.md` pointing the
    phonetic-bonus reader at "§6.5" (same_as/additional_types Jaccard)
    instead of the real §6.7 (phonetic).

    Independently rediscovered the same **live CI bug** the
    `place/place-matcher-rust-crate` sub-note above landed moments
    earlier: `scripts/spec-drift-check.sh` (wired into
    `.github/workflows/spec-drift.yml` on every PR) checked for a
    changed file literally named `spec.md`, but this crate's spec is a
    directory (`spec/*.md` — split from a single file in an earlier
    revision per the repo-root `AGENTS.md` "Historical note");
    reproduced the bug standalone (`grep -Fx "spec.md"` against a
    fixture diff containing `spec/05-matching-engine.md` never
    matches) before fixing it to check `^spec/` instead, and
    re-confirmed the fix passes the same fixture — meaning **every
    prior PR that touched `src/matcher.rs` would have failed this
    check regardless of how thoroughly `spec/` was updated alongside
    it**, the opposite of a silent gap. Also fixed the same script's
    (and the workflow's) stale `spec.md §23 T-7` header-comment
    reference — §23 doesn't exist in this crate's §1–§13 shape.
    Also independently found the same **`cargo audit` vs `cargo deny`
    mismatch**: `AGENTS/security-and-privacy.md` and `AGENTS/
    release.md` both told contributors to run `cargo audit`, but this
    crate's actual `.github/workflows/security.yml` runs `cargo deny
    check` (confirmed by reading the workflow, whose own comment says
    it supersedes `cargo-audit`) — fixed both docs and added the
    missing `cargo deny check` line to the release checklist.

    Also fixed: `spec/05-matching-engine.md` §5.6 omitted the public
    `Scorer::optional_field_score` from its "Scorer exposes" list even
    though `tests/adapter_contract.rs`'s own doc comment already cites
    "spec §5.6" as documenting it; `spec/09` referenced "a task in
    §13" for drift resolution when this crate's §13 is References, not
    tasks (repointed to §10, matching its own `AGENTS/
    spec-driven-development.md`); a stale "(10 tests)" adapter-contract
    count in both `spec/09` and `AGENTS/testing.md` (real count is 11,
    confirmed via `cargo test --test adapter_contract`); stray
    double-curly-brace template artifacts (`MatchResult {{ score, ...
    }}`) in `AGENTS/testing.md`; `mimalloc` missing from the dependency
    lists in `AGENTS/security-and-privacy.md` and `AGENTS/release.md`
    (it's a real `[dependencies]` entry, used only by the
    `src/main.rs` demo binary's musl `#[global_allocator]`, not linked
    into the library); and `fuzz/` (SEC-I2, already in `CHANGELOG.md
    [Unreleased]`) missing from `AGENTS.md`'s file-layout diagram.
    Rustdoc spot-check (`lib.rs`, `matcher.rs`, `scorer.rs`,
    `normalizer.rs`, `models.rs`) found no other stale `///`/`//!`
    comments — spot-verified the Soundex worked-example table in
    `AGENTS/normalization.md` (Robert/Rupert → R163, Ashcraft → A261,
    Smith/Smyth → S530) against the real `Normalizer::phonetic_code`
    output via a throwaway example binary (deleted after) — all five
    matched exactly. `README.md` is a real symlink to `index.md`;
    neither mentions tags/relationships and every code snippet in
    `index.md` uses real, current public API. `AGENTS/architecture.md`,
    `AGENTS/matching-algorithm.md`, `AGENTS/coding-style.md` needed no
    changes — all already accurate. Verified clean throughout:
    `cargo test --lib` (86), `cargo test --test adapter_contract` (11),
    `cargo test --doc` (67), full `cargo test` (all suites green). This
    was a docs/CI-script-only change — no `src/` edits — so no clippy/
    fmt re-check was needed beyond confirming the test suite still
    passes. Did not touch `thing/thing-service-with-loco` (sibling
    crate, out of this task's boundary). All edited files re-confirmed
    well under 40 KB.

  - *`event/event-matcher-rust-crate` done 2026-08-04.* `CLAUDE.md`
    already the thin `@AGENTS.md` one-liner. `spec/index.md` confirms
    this crate runs §1–§13, not the family's §1–§25 matcher shape
    (`spec-driven-development.md` says so explicitly, "SDD artefacts
    that some projects split across multiple files are consolidated
    into the numbered sections") — a genuine, deliberate difference
    from `person-matcher`/`worker-matcher`, not drift, so left as is.
    Biggest finding, the **same shape as place-matcher's** (confirming
    the pattern is systemic, not one-off): `spec/03-data-model.md`,
    `spec/05-matching-pipeline.md`, `spec/06`, and `spec/07` presented
    the planned `relationships`/`tags` fields, their
    `MatchBreakdown`/`MatchConfig` members, and §5.2.1's "eleven
    weighted components" as already-shipped fact, while
    `CHANGELOG.md`'s own "Unreleased" section already (correctly)
    labels both "implementation pending" — `Event` was documented as
    "26 fields" when the real struct has 24, `MatchBreakdown` as "13
    score fields" vs the real 11, and `RelationshipRef`/`RelationKind`
    types that don't exist in `src/models.rs` at all (confirmed by
    grep). Against this crate's own spec §9.3 ("code wins on
    divergence"). Fixed with "planned, not yet implemented" callouts
    in all four files rather than deleting the design (real tracked
    work per the CHANGELOG). Second, much larger finding: **every file
    in `AGENTS/` (architecture.md, coding-style.md,
    matching-algorithm.md, normalization.md, release.md,
    security-and-privacy.md, spec-driven-development.md) was still
    describing the crate's pre-0.5.0 domain** — `Place`, `PlaceBuilder`,
    `PlaceCategory`, `PlaceId`/`PlaceIdScheme`, `match_places`, a
    coordinates/address/place-id weight table, and (most seriously) a
    fabricated `MatchingEngine::score_phone` method plus
    `MatchConfig::phone_default_country`/`gmail_dot_folding` fields
    that never existed in the Event domain (`Event` carries no `phone`
    or `email` field at all — confirmed by grep; `normalize_phone*`/
    `normalize_email` are unused library-only utilities, per
    `spec/04-normalisation.md`'s own explicit note). `AGENTS.md` and
    `CLAUDE.md` themselves were already correctly rewritten for Event
    at the 0.5.0 rebrand; only the `AGENTS/*.md` topic guides were
    missed, for all seven files — the largest single-crate `AGENTS/`
    staleness found in this batch so far. Rewrote all seven files
    against the real `src/models.rs`/`src/matcher.rs` surface: correct
    default-weight table (name/start_date/end_date/location/category/
    country_code/event_ids/organizer/performers/url), the real worked
    examples (Glastonbury Festival, RustConf, strict-mode Cafe
    Centrale/Central), and an explicit note that this crate does
    **not** implement window-overlap temporal scoring —
    `start_date`/`end_date` are scored independently by Gaussian decay
    over endpoint distance, and window-overlap is
    `spec/10-open-questions.md` OQ-C, still open. Third:
    `agents/share/overview.md`'s matcher table flatly claimed
    event-matcher does "Time-bounded event matching with
    window-overlap" — false per the above (verified by grep across
    `src/`: zero occurrences of "overlap" outside a doc comment and
    OQ-C itself) — fixed the one row to describe the real Gaussian
    endpoint-decay algorithm and point at OQ-C, re-reading the file
    immediately before editing given nine sibling agents editing
    adjacent crates concurrently. `README.md`/`index.md` were already
    accurate (no Place references, no premature relationships/tags
    claims) and its code examples were spot-checked against the real
    API; `cargo test --doc` reconfirmed 74 passing doctests unchanged
    before and after. Did not touch `event/event-service-with-loco`
    (sibling crate, out of this task's boundary). All edited files
    re-confirmed well under 40 KB.

- [ ] **DOC-4 (L)** Front-end crates' `spec/`, `AGENTS.md`,
  `README.md`/`index.md`: all 11 SvelteKit front-ends (person, worker,
  place, thing, event, course, organization, care-pathway, case,
  project-portfolio-management, authentication). Cross-check each
  against what actually shipped this session (merge/link/bulk/review
  screens landed in FE-1..FE-4 — confirm each front-end's own `spec/`
  and `AGENTS.md` mention them, since they landed in code+tests but a
  doc pass wasn't guaranteed for every one). Confirm the BFF env-var
  names documented match what `src/lib/server/config.ts` (or
  equivalent) actually reads — TUT-1 and TUT-2 both found a stale
  `.env.example` this session (case, person); check the other 9 rather
  than assume they're fine.

- [ ] **DOC-5 (M)** Library crates' `spec/`, `AGENTS.md`/`CLAUDE.md`,
  `README.md`/`index.md`: `authentication-verifier-rust-crate`,
  `integrity-mac-rust-crate`, `link/entity-ref-rust-crate`. Smaller
  crates, lighter audit; `entity-ref` has no `CHANGELOG.md` at all
  (found during H-5) — decide whether that's a real gap to fix here or
  a separate call.

- [ ] **DOC-6 (M)** `link-graph-service-with-loco`'s `spec/`,
  `AGENTS.md`, `README.md`/`index.md`. **Queue this after LNK-4's
  T-29..T-33 chain finishes** (active file conflicts otherwise) — by
  the time this runs, `spec/13-tasks.md` and `spec/16-open-questions.md`
  will have moved substantially from whatever a doc pass would see
  mid-chain.

- [ ] **DOC-7 (L)** The five consumer apps' docs — `case-folder`,
  `patient-flow`, `workforce-planning-management`,
  `contact-relationship-management`, `content-management-system`. Each
  has a cross-cutting `spec/` (the SDD trio `requirements.md` /
  `design.md` / `tasks.md`, distinct from the numbered shape) plus a
  per-edition service + front-end, each with its own `AGENTS.md`.
  Confirm `spec/tasks.md` (each app's live queue, not the numbered
  shape's §13) reflects real status, and that each app's service/
  front-end pair's docs agree with each other on what's actually
  wired up.

- [ ] **DOC-8 (S)** Once DOC-2..DOC-7 land, a final sweep: re-grep for
  the family-wide anti-patterns found along the way (duplicated
  capability tables, stale `.env.example` files, "Backend-only"-style
  absolute claims that stopped being true) across anything DOC-2..7
  didn't individually call out, and confirm no `AGENTS.md`/`CLAUDE.md`/
  `AGENTS/*.md` file crossed 40 KB as a result of this pass's edits.
