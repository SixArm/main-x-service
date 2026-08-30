# Main X Index Rust crates

@agents/share/overview.md

## Subprojects

Each subproject's **`spec/` directory** is the single source of truth
for that subproject. Code conforms to the spec, not the other way
round, and a behavioural change is one PR carrying all three parts:
spec edit, code edit, test edit.

Two spec shapes exist, and they are different on purpose:

| Shape | Used by | Layout | Live task queue |
|---|---|---|---|
| **Numbered** | entity service crates, matcher crates, front-end projects | `spec/01-purpose-and-vision.md` … (§1–§18 for service/front-end crates; matcher crates vary¹) | §13 |
| **Topic-named** | the five consumer applications | `spec/<topic>.md` plus the SDD trio `requirements.md` / `design.md` / `tasks.md` | `spec/tasks.md` |

The numbered shape suits a crate with one entity and a fixed set of
concerns; the topic shape suits an application whose modules are the
natural unit. Neither is being migrated to the other — a rename that
buys nothing is churn.

¹ The matcher crates' own §1–§25 shape ([overview.md](agents/share/overview.md))
is not evenly rolled out: person, worker, and course run the full
`01-…` … `25-…` spread; place, thing, and event stop at `13-references.md`;
organization, care-pathway, case, and project-portfolio-management
(portfolio) are each still one `spec/index.md`, with an open task per
crate to split into the numbered shape.

> Historical note: these were single `spec.md` files. They are
> directories now, and no live `spec.md` remains (the ones under
> `target/package/` are packaging artifacts of old releases).

See any per-crate `agents/spec-driven-development.md` for the
discipline, including the anti-patterns.

### Service crates

| Crate | Entity | Docs |
|---|---|---|
| [person-service](person/person-service-with-loco/) | Person (general) | [spec](person/person-service-with-loco/spec/index.md) · [index](person/person-service-with-loco/index.md) |
| [worker-service](worker/worker-service-with-loco/) | Worker (workforce / professional, incl. assessments) | [spec](worker/worker-service-with-loco/spec/index.md) · [index](worker/worker-service-with-loco/index.md) |
| [place-service](place/place-service-with-loco/) | Place (schema.org/Place) | [spec](place/place-service-with-loco/spec/index.md) · [index](place/place-service-with-loco/index.md) |
| [thing-service](thing/thing-service-with-loco/) | Thing (schema.org/Thing) | [spec](thing/thing-service-with-loco/spec/index.md) · [index](thing/thing-service-with-loco/index.md) |
| [event-service](event/event-service-with-loco/) | Event (schema.org/Event, time-bounded) | [spec](event/event-service-with-loco/spec/index.md) · [index](event/event-service-with-loco/index.md) |
| [course-service](course/course-service-with-loco/) | Course (+ `CourseInstance` sub-resource) | [spec](course/course-service-with-loco/spec/index.md) · [index](course/course-service-with-loco/index.md) |
| [authentication-service](authentication/authentication-service-with-loco/) | User — the central SSO provider; **the reference loco.rs crate** | [spec](authentication/authentication-service-with-loco/spec/index.md) · [index](authentication/authentication-service-with-loco/index.md) |
| [organization-service](organization/organization-service-with-loco/) | Organization (schema.org/Organization) | [spec](organization/organization-service-with-loco/spec/index.md) · [index](organization/organization-service-with-loco/index.md) |
| [care-pathway-service](care-pathway/care-pathway-service-with-loco/) | Care pathway (clinical pathway) | [spec](care-pathway/care-pathway-service-with-loco/spec/index.md) · [index](care-pathway/care-pathway-service-with-loco/index.md) |
| [case-service](case/case-service-with-loco/) | Case (governmental case tracking) | [spec](case/case-service-with-loco/spec/index.md) · [index](case/case-service-with-loco/index.md) |
| [project-portfolio-management-service](project-portfolio-management/project-portfolio-management-service-with-loco/) | Plan (one recursive tree; `kind` is a label, not a gate) | [spec](project-portfolio-management/project-portfolio-management-service-with-loco/spec/index.md) · [index](project-portfolio-management/project-portfolio-management-service-with-loco/index.md) |

What each service actually carries — the honest per-crate capability
matrix — is in [overview.md](agents/share/overview.md); duplicating it
here is how the two stop agreeing.

### Matcher crates

The matcher crates are reusable, dependency-light Rust libraries for
pairwise record comparison. They follow their own SDD shape (see each
crate's `spec/` for its full §1–§25 structure), distinct from the
service-crate shape (§1–§18). Use them standalone, or embed them in
the corresponding service crate's matching layer.

| Crate | Entity | Docs |
|---|---|---|
| [person-matcher](person/person-matcher-rust-crate/) | Person | [spec](person/person-matcher-rust-crate/spec/index.md) · [index](person/person-matcher-rust-crate/index.md) |
| [worker-matcher](worker/worker-matcher-rust-crate/) | Worker | [spec](worker/worker-matcher-rust-crate/spec/index.md) · [index](worker/worker-matcher-rust-crate/index.md) |
| [place-matcher](place/place-matcher-rust-crate/) | Place | [spec](place/place-matcher-rust-crate/spec/index.md) · [index](place/place-matcher-rust-crate/index.md) |
| [thing-matcher](thing/thing-matcher-rust-crate/) | Thing | [spec](thing/thing-matcher-rust-crate/spec/index.md) · [index](thing/thing-matcher-rust-crate/index.md) |
| [event-matcher](event/event-matcher-rust-crate/) | Event | [spec](event/event-matcher-rust-crate/spec/index.md) · [index](event/event-matcher-rust-crate/index.md) |
| [course-matcher](course/course-matcher-rust-crate/) | Course | [spec](course/course-matcher-rust-crate/spec/index.md) · [index](course/course-matcher-rust-crate/index.md) |
| [organization-matcher](organization/organization-matcher-rust-crate/) | Organization | [spec](organization/organization-matcher-rust-crate/spec/index.md) · [index](organization/organization-matcher-rust-crate/index.md) |
| [care-pathway-matcher](care-pathway/care-pathway-matcher-rust-crate/) | Care pathway | [spec](care-pathway/care-pathway-matcher-rust-crate/spec/index.md) · [index](care-pathway/care-pathway-matcher-rust-crate/index.md) |
| [case-matcher](case/case-matcher-rust-crate/) | Case | [spec](case/case-matcher-rust-crate/spec/index.md) · [index](case/case-matcher-rust-crate/index.md) |
| [project-portfolio-management-matcher](project-portfolio-management/project-portfolio-management-matcher-rust-crate/) | Plan | [spec](project-portfolio-management/project-portfolio-management-matcher-rust-crate/spec/index.md) · [index](project-portfolio-management/project-portfolio-management-matcher-rust-crate/index.md) |

Each matcher's components, weights, and deterministic short-circuits
are in its own `spec/`; [overview.md](agents/share/overview.md)
summarises them one line each.

### Library crates

Peer-side support libraries (not services, not matchers). Dependency-light,
and published to crates.io for downstream consumers where noted below —
two of the three are; Integrity MAC is not yet (see its row).

| Crate                                                                         | Entity                                                                                                                                                                                                                                                                                                                                    | Spec                                                                    | Index                                                               |
| ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------------- |
| [Authentication Verifier](authentication/authentication-verifier-rust-crate/) | User — peer-side **offline PASETO v4.public (Ed25519) verification** for the [Authentication Service](authentication/authentication-service-with-loco/); fetches/holds the service's `/.well-known/paseto-keys`, mirrors the `Claims` shape, verifies `kid`/`iss`/`aud`/`exp`. Published to crates.io as `authentication-verifier` (0.9). | [spec](authentication/authentication-verifier-rust-crate/spec/index.md) | [index](authentication/authentication-verifier-rust-crate/index.md) |
| [Integrity MAC](integrity/integrity-mac-rust-crate/) | — (cross-cutting) — keyed integrity **MACs** (HMAC-SHA256, FIPS 198-1) with production-grade key handling for every service carrying a tamper-evidence tier: HKDF-SHA256 subkeys per (service, domain) so a tag cannot transfer between purposes or between services sharing one key; key-file sourcing that takes precedence over the environment and never falls back; root-key zeroization; placeholder refusal; key generation from the OS CSPRNG. Extracted 2026-07-27 rather than copied per service — a key-handling defect in a copy would make MACs forgeable while every test stayed green. **Not published to crates.io** — every consumer is in-tree via a Cargo `path` dependency; unlike Entity Ref below, no release has been cut. | — | — |
| [Entity Ref](link/entity-ref-rust-crate/) | — (cross-cutting) — the [cross-service-linking](agents/share/cross-service-linking.md) contract: `EntityRef` (the `entity_type:uuid` URN value type) + the closed v1 `EdgeKind` registry (`is_symmetric`/`is_temporal`/`inverse`/`sensitivity`/`permits`). Designed to be copied per project but never actually was — it is a real Cargo `path` dependency of eight crates (`link-graph-service-with-loco` plus person/worker/case and four consumer apps). **Published to crates.io as `entity-ref` (0.2.0, 2026-08-05)** — correcting an earlier "not yet published" claim here; every in-tree consumer still takes the `path` dependency rather than the crates.io release. | — | — |

### Front-end projects

SvelteKit front-ends sit alongside their service crates. Each is an
independent SPA built on SvelteKit 2 + Svelte 5 runes + SVAR Svelte
DataGrid + Lily Design System Svelte Headless, calling the sibling
service's REST API. Their per-project `spec/` follows the same
§1–§18 SDD shape as the service crates. Drift between front-ends is
accepted (see `feedback_front_end_drift` memory) — no shared package.

| Project                                                                                                                                | Consumes                                                                                                             | Spec                                                                                                  | Changelog                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| [person-front-end-with-svelte](person/person-front-end-with-svelte/)                                                                   | [person-service](person/person-service-with-loco/)                                                                   | [spec](person/person-front-end-with-svelte/spec/index.md)                                             | [CHANGELOG](person/person-front-end-with-svelte/CHANGELOG.md)                                             |
| [worker-front-end-with-svelte](worker/worker-front-end-with-svelte/)                                                                   | [worker-service](worker/worker-service-with-loco/)                                                                   | [spec](worker/worker-front-end-with-svelte/spec/index.md)                                             | [CHANGELOG](worker/worker-front-end-with-svelte/CHANGELOG.md)                                             |
| [place-front-end-with-svelte](place/place-front-end-with-svelte/)                                                                      | [place-service](place/place-service-with-loco/)                                                                      | [spec](place/place-front-end-with-svelte/spec/index.md)                                               | [CHANGELOG](place/place-front-end-with-svelte/CHANGELOG.md)                                               |
| [thing-front-end-with-svelte](thing/thing-front-end-with-svelte/)                                                                      | [thing-service](thing/thing-service-with-loco/)                                                                      | [spec](thing/thing-front-end-with-svelte/spec/index.md)                                               | [CHANGELOG](thing/thing-front-end-with-svelte/CHANGELOG.md)                                               |
| [event-front-end-with-svelte](event/event-front-end-with-svelte/)                                                                      | [event-service](event/event-service-with-loco/)                                                                      | [spec](event/event-front-end-with-svelte/spec/index.md)                                               | [CHANGELOG](event/event-front-end-with-svelte/CHANGELOG.md)                                               |
| [course-front-end-with-svelte](course/course-front-end-with-svelte/)                                                                   | [course-service](course/course-service-with-loco/)                                                                   | [spec](course/course-front-end-with-svelte/spec/index.md)                                             | [CHANGELOG](course/course-front-end-with-svelte/CHANGELOG.md)                                             |
| [authentication-front-end-with-svelte](authentication/authentication-front-end-with-svelte/)                                           | [authentication-service](authentication/authentication-service-with-loco/)                                           | [spec](authentication/authentication-front-end-with-svelte/spec/index.md)                             | [CHANGELOG](authentication/authentication-front-end-with-svelte/CHANGELOG.md)                             |
| [organization-front-end-with-svelte](organization/organization-front-end-with-svelte/)                                                 | [organization-service](organization/organization-service-with-loco/)                                                 | [spec](organization/organization-front-end-with-svelte/spec/index.md)                                 | [CHANGELOG](organization/organization-front-end-with-svelte/CHANGELOG.md)                                 |
| [care-pathway-front-end-with-svelte](care-pathway/care-pathway-front-end-with-svelte/)                                                 | [care-pathway-service](care-pathway/care-pathway-service-with-loco/)                                                 | [spec](care-pathway/care-pathway-front-end-with-svelte/spec/index.md)                                 | [CHANGELOG](care-pathway/care-pathway-front-end-with-svelte/CHANGELOG.md)                                 |
| [case-front-end-with-svelte](case/case-front-end-with-svelte/)                                                                         | [case-service](case/case-service-with-loco/)                                                                         | [spec](case/case-front-end-with-svelte/spec/index.md)                                                 | [CHANGELOG](case/case-front-end-with-svelte/CHANGELOG.md)                                                 |
| [project-portfolio-management-front-end-with-svelte](project-portfolio-management/project-portfolio-management-front-end-with-svelte/) | [project-portfolio-management-service](project-portfolio-management/project-portfolio-management-service-with-loco/) | [spec](project-portfolio-management/project-portfolio-management-front-end-with-svelte/spec/index.md) | [CHANGELOG](project-portfolio-management/project-portfolio-management-front-end-with-svelte/CHANGELOG.md) |

### Consumer applications

Application subprojects that **consume** the index services rather than
being one of the matcher-backed index entities. They follow a different
internal shape (a cross-cutting `spec/` plus a per-edition service +
front-end) and are not part of the entity trio tables above.

> Note: distinct from the **`case`** index entity above. `case` is a
> matcher-backed registry of case _identities_ (deduplicated, matchable);
> **`case-folder`** is an operational app that tracks the physical
> _location_ of NHS paper case-note folders, consuming the person /
> place / worker services.

| Project | Purpose | Editions |
|---|---|---|
| [case-folder](case-folder/spec/index.md) | NHS paper case-note folder location tracking — "where is the folder for NHS Number X right now?", with a barcode/QR/RFID move audit trail. | [service](case-folder/case-folder-service-with-rust/spec/index.md) · [front-end](case-folder/case-folder-front-end-with-svelte/spec/index.md) |
| [patient-flow](patient-flow/spec/index.md) | NHS hospital patient flow and bed management — live bed state, ward whiteboards, the SAFER/Red2Green inpatient journey, rule-checked allocation, capacity at a glance. Owns operational state; references person / worker / place / organization by `EntityRef`. | [service](patient-flow/patient-flow-service-with-rust/spec/index.md) · [front-end](patient-flow/patient-flow-front-end-with-svelte/spec/index.md) |
| [workforce-planning-management](workforce-planning-management/spec/index.md) | All-in-one HR across the employee lifecycle — talent acquisition, workforce management, HR service delivery, talent development, payroll and compensation. Owns the employment relationship; references person / worker / organization / course. | [service](workforce-planning-management/workforce-planning-management-service-with-rust/spec/index.md) · [front-end](workforce-planning-management/workforce-planning-management-front-end-with-svelte/spec/index.md) |
| [contact-relationship-management](contact-relationship-management/spec/index.md) | Customer and prospect relationships — sales automation, consent-first marketing, service and support, derived analytics. Owns relationship state; identity dedup stays upstream. Distinct from the governmental `case` registry. | [service](contact-relationship-management/contact-relationship-management-service-with-rust/spec/index.md) · [front-end](contact-relationship-management/contact-relationship-management-front-end-with-svelte/spec/index.md) |
| [content-management-system](content-management-system/spec/index.md) | Headless content management — content modelling and authoring (block documents, not stored HTML), digital assets, the editorial lifecycle, localization, delivery and SEO, and derived content insights. Owns content and editorial state; readers are not modelled at all. | [service](content-management-system/content-management-system-service-with-rust/spec/index.md) · [front-end](content-management-system/content-management-system-front-end-with-svelte/spec/index.md) |

Delivery status for each lives in its own `spec/tasks.md`, which is the
single source of truth — repeating it here is how the two drift.

### Cross-cutting services

Infrastructure services that span the entity trios rather than owning one
entity. They are not matcher-backed and have no front-end of their own.

| Service                                                                         | Purpose                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Spec                                                    |
| ------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| [link-graph-service-with-loco](link/link-graph-service-with-loco/spec/index.md) | **Read-model aggregator** for cross-service entity linking — consumes every entity's event stream (+ the new `linked`/`unlinked` events) and serves the queryable cross-service graph (`neighbors` / `single-view` / freshness). The read side of the hybrid topology in [cross-service-linking.md](agents/share/cross-service-linking.md); each entity service owns its own link **writes** (`entity_links` + events). v1 edges: `same_identity` (person↔worker), `works_at`/`member_of` (person→org), `employed_by` (worker→org), `subject_of` (case→person). | [spec](link/link-graph-service-with-loco/spec/index.md) |

## Continuous integration

Both remotes run the same checks, because `origin` pushes to Codeberg
**and** GitHub: `.github/workflows/ci.yml` and `.woodpecker.yml`. Every
step shells out to `scripts/ci-check.sh`, so the two platforms run
byte-identical commands — a check that only fails on one platform is a
check nobody trusts.

There is **no root `Cargo.toml`**: each crate is its own workspace, so
nothing can use `--workspace` to cover the tree. `scripts/ci-crates.sh`
discovers the ~55 crates and feeds them to each stage.

| Stage | Command | Notes |
|---|---|---|
| `fmt` | `cargo fmt --check` | one pass over every crate |
| `docs` | index + content scan | repo-wide, no build: the agents directory is lowercase ([spec](spec/agents-directory-name-is-lowercase/index.md)) |
| `clippy` | `cargo clippy --all-targets -- -D warnings` | `-D warnings` is what keeps `#![warn(clippy::pedantic)]` at zero |
| `test` | `cargo test` | DB-gated suites stay skipped |
| `test-db` | `cargo test -- --ignored` | only crates enrolled in [`ci/db-suites.txt`](ci/db-suites.txt), against a Postgres service |
| `deny` | `cargo deny check` | advisories + licences, where a `deny.toml` exists |
| `evidence` | SBOM render | IEC 62304 §8.1.2 / FD&C §524B |
| `fuzz` | `cargo +nightly fuzz run <target>` | `FUZZ_SECONDS` (default 30) per target, for the `fuzz/` sub-crates; a short smoke, not exhaustive fuzzing |
| `msrv` | `cargo +<msrv> check --all-targets` | asserts every `rust-version` equals [`ci/msrv.txt`](ci/msrv.txt), then compiles against that toolchain |
| `bench` | `cargo bench --no-run` | compiles the Criterion benches (`--all-targets` already type-checks them; this proves they *link*) |

**The MSRV is the current stable minus two** — today **1.96**, from
stable 1.98. It is declared per crate (`rust-version` in every
`[package]`, since there is no root manifest to inherit from), sourced
from [`ci/msrv.txt`](ci/msrv.txt), and enforced by the `msrv` stage. At
N-2 the MSRV's minor version now coincides with the
`rust-toolchain.toml` pin (1.96.1, what we build with) — the pin still
carries a patch version the MSRV does not, so the two remain distinct
numbers with distinct jobs (§3 of the policy doc), they simply moved
closer together than they were at N-3. The binding constraint is still
the dependency graph, not our code: loco-rs / sea-orm / sqlx require
1.94, so 1.96 clears it with one release of headroom. See
[`spec/rust-msrv-n-minus-2/index.md`](spec/rust-msrv-n-minus-2/index.md) for the
policy and the bump procedure.

**`ci/db-suites.txt` is an allowlist, not a denylist.** A crate joins it
once its `--ignored` suite has been observed green, so CI starts green and
stays meaningful; the file records why each excluded crate is excluded.
The DB-gated suites are where the compliance controls are actually proven
— no unit test can show that an audit digest survives a Postgres JSONB
round-trip, or that a raw-SQL row edit is detected.

Run any stage locally exactly as CI does:

```sh
scripts/ci-check.sh clippy                       # every crate
scripts/ci-check.sh test-db care-pathway/care-pathway-service-with-loco
```

### The local test database

`test-db` needs a Postgres, so **every service crate carries a
`compose.test.yaml`** — one `postgres:18-alpine` container with the same
superuser (`loco`/`loco`), port (5432), database name, and extensions CI
provides, and its data directory on tmpfs so every start is a clean
initdb. Podman, not Docker.

```sh
scripts/test-db.sh up   <crate>   # start it, wait until healthy
scripts/ci-check.sh test-db <crate>
scripts/test-db.sh down <crate>   # also: psql · logs · url · status · down-all
```

Two at once want the same port, so move one:
`TEST_DB_PORT=5433 scripts/test-db.sh up <crate>`.

`test-db` is a no-op for a crate not in [`ci/db-suites.txt`](ci/db-suites.txt);
`DB_SUITES_FORCE=1` runs it anyway, which is how a crate gets **observed**
green before being added to that allowlist.

## AI agent guidance

- [`llms.txt`](llms.txt) / [`llms.json`](llms.json) — a curated,
  size-bounded (<40KB each) map of this repo's most important content,
  for a tool that wants a starting point rather than a full-tree
  crawl. Defined by [`spec/llms-json-and-llms-txt/index.md`](spec/llms-json-and-llms-txt/index.md);
  update both together with this file whenever a subproject, matcher,
  front-end, or shared doc is added, renamed, or removed.
- [`sixarm-services-skill/`](sixarm-services-skill/SKILL.md) /
  [`sixarm-services-maintainer-skill/`](sixarm-services-maintainer-skill/SKILL.md) —
  Claude Code skills for, respectively, explaining what this system
  does and making a change to it. Defined by
  [`spec/agent-skills/index.md`](spec/agent-skills/index.md).

## Shared reference docs

@agents/share/index.md

@agents/share/architecture.md
@agents/share/dataflow.md
@agents/share/match-search-merge.md
@agents/share/match.md
@agents/share/search.md
@agents/share/merge.md
@agents/share/privacy.md
@agents/share/jwt.md
@agents/share/authentication-sessions.md
@agents/share/authorization-attributes.md
@agents/share/security.md
@agents/share/auditability.md
@agents/share/cross-service-linking.md
@agents/share/bulk-import-export.md
@agents/share/availability.md
@agents/share/observability.md
@agents/share/restful.md
@agents/share/fhir.md
@agents/share/api-versioning.md
@agents/share/loco.md
@agents/share/rust-loco-stack.md
@agents/share/postgresql.md
@agents/share/locales.md
@agents/share/compliance-for-healthcare.md
@agents/share/compliance-for-technology.md

## Common features

### Data management

- Create, read, update, and delete (CRUD) records
- Soft delete with complete audit trails
- Multiple identifiers per record (type + system + value)
- Identity documents (passport, driver's license, etc., where the entity supports them)
- Multiple contacts per record (phone / email / address)
- Automatic event stream publishing for all CRUD operations

### Matching

- **Probabilistic matching** — weighted fuzzy scoring
- **Deterministic matching** — rule-based with short-circuit (tax-ID, document, GLN, …)
- **Configurable scoring** — thresholds and weights are tunable
- **Components** — string similarity (Jaro-Winkler, Levenshtein, Soundex), date proximity, geo (Haversine), identifier exact-match
- **Score breakdown** — full per-component scores in API responses

### Data quality & validation

- Required-field enforcement
- Date / range validation (no future birth dates, lat/lon bounds, GLN check digit, …)
- Email and phone format checks
- Address validation (requires locality, postal code, or country)
- Document validation (number required, expiry check, issue-before-expiry)
- Phone normalization (E.164-like)
- Address standardization (title-case locality, uppercase region/country, abbreviation expansion)
- Validation integrated into create/update handlers (returns `422`)

## Per-subproject docs

Every subproject ships these four, and they mean the same thing
everywhere:

| File | Role |
|---|---|
| `spec/` | **the single source of truth** — requirements, design, and the live task queue |
| `index.md` | navigation aid with worked examples |
| `README.md` | user-facing intro: what it is, quick start, environment variables |
| `CLAUDE.md` | a one-line `@AGENTS.md` include, loaded by Claude Code at session start |
| `AGENTS.md` | working agreements: ground rules, family conventions, known gotchas |
| `CHANGELOG.md` | Keep a Changelog format |

> Some service crates' `README.md` is a **symlink to `index.md`** —
> edit `index.md`.

### The `agents/` directory: older subprojects only

The six original entity crates (person, worker, place, thing, event,
course) and the matcher crates carry an `agents/` directory of
reference docs — `index.md`, `spec-driven-development.md`,
`models.md`, `matching.md`, `restful.md`, `testing.md`. Twenty-two
newer subprojects do not, and that is a decision rather than a gap:
those files restate what the spec already says, and a restatement
that nobody regenerates is exactly the drift the SDD discipline
exists to prevent. Newer subprojects keep the same material in
`spec/` and point at it from `AGENTS.md`.

If you are adding a subproject, follow the newer pattern. If you are
editing an older one, keep its `agents/` in step with its `spec/` —
or delete the file rather than let it rot.

### Where planning lives

There is no per-subproject `plan.md`. Plan content lives in the spec
(§8–§12 in the numbered shape, `design.md` in the topic shape).

Task queues live in the spec too, but the file differs by shape:
**numbered** subprojects keep theirs in `spec/13-*` (§13), and
**topic-named** ones in `spec/tasks.md`. The repo root additionally
keeps cross-cutting [`plan.md`](plan.md) and [`tasks.md`](tasks.md)
for work that spans subprojects.
