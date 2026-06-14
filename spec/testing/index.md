# Testing — monorepo-wide specification

This is the family-wide reference for the **testing strategy** across
the Main X Index. It is descriptive of what the code actually does, not
aspirational: where a layer is partial or deferred it says so.

The repo is a polyglot monorepo — Rust service crates, Rust matcher /
verifier library crates, and SvelteKit front-ends — and the testing
discipline differs by tier. The unifying rule is the same in every
tier: **the default, no-flags test command must pass with no database,
no broker, and no network.** Anything that needs infrastructure is
explicitly gated and opted into.

Each subproject keeps its own testing doc as the local source of truth.
The per-crate service docs are the most detailed:

- [person-service AGENTS/testing.md](../../person/person-service-rust-crate/AGENTS/testing.md)
- [worker-service AGENTS/testing.md](../../worker/worker-service-rust-crate/AGENTS/testing.md)

This page is the cross-cutting view that sits above them. Related
monorepo docs: [matching](../matching/index.md),
[architecture](../architecture/index.md),
[postgresql](../postgresql/index.md), [restful](../restful/index.md),
[validation](../validation/index.md),
[search](../search/index.md), [merge](../merge/index.md),
[auditability](../auditability/index.md),
[event-streaming](../event-streaming/index.md),
[observability](../observability/index.md), and
[authentication](../authentication/index.md).

---

## 1. The layered Rust strategy

Rust testing is layered. Each layer has a different cost, a different
set of dependencies, and a different command. The layers, from cheapest
and most-frequently-run to most expensive:

| Layer | Command | Needs DB / net? | What it pins |
| ----- | ------- | --------------- | ------------ |
| **Unit tests** (un-gated) | `cargo test --lib` | No | Matching algorithms, validation, privacy/masking, normalization, auth crypto (RS256/JWKS), OpenAPI document shape, in-memory streaming, model (de)serialization |
| **Bridge tests** (un-gated) | `cargo test --test duplicate_detection` / `--test matching` | No | The service ↔ matcher contract: adapter field-routing **and** matcher scoring, in one black-box suite |
| **Request / integration tests** (gated) | `cargo test -- --ignored` | Yes (Postgres) | Full HTTP request/response cycles against a live database |
| **Doc tests** (un-gated) | `cargo test --doc` | No | Public-API examples in `///` doc comments stay compilable and correct |
| **Benchmarks** | `cargo bench` (Criterion) | No | Statistical performance of matching / search / validation hot paths |

The design intent: a contributor on a database-less laptop runs
`cargo test` and gets a fully green, meaningful signal. The
infra-dependent layer is additive, run with an explicit flag and in CI.

### 1.1 Un-gated unit tests

Unit tests live in `#[cfg(test)] mod tests` blocks at the bottom of the
source file they exercise, run by `cargo test --lib`, and touch **no
external dependency** — no database, no network, no filesystem beyond
`tempfile::tempdir()` for an ephemeral Tantivy index. They are the bulk
of the suite (the person- and worker-service crates carry ~99 unit
tests each).

Coverage tracks the business-logic layer from
[architecture](../architecture/index.md):

| Module | What's tested |
| ------ | ------------- |
| `matching::algorithms` | Name / DOB / gender / address / identifier / tax-ID / document scoring |
| `matching::phonetic` | Soundex encoding, matching, similarity |
| `matching::scoring` | Probabilistic + deterministic scoring, confidence classification |
| `search::*` | Tantivy schema, index/search, fuzzy + name-year search (against a temp index) |
| `validation` | Required-field enforcement, phone normalization, address standardization |
| `privacy` | Field masking, record masking |
| `models::*` | Construction + serde round-trips |
| `auth` (loco services) | RS256 verification, `kid`/`iss`/`aud`/`exp` checks, JWKS parsing |
| `openapi` (loco services) | Hand-written OpenAPI 3 document shape |

These are pure and deterministic: same input, same result, no ordering
or environment sensitivity. See the per-module breakdown in
[person-service AGENTS/testing.md](../../person/person-service-rust-crate/AGENTS/testing.md).

### 1.2 Bridge tests

The service crates do not re-implement matching — they embed the
canonical sibling matcher crate (see [matching](../matching/index.md))
and project their domain model into the matcher's type. That projection
is the seam most likely to drift, so it gets a dedicated **bridge
test** that pins *both sides of the contract at once*:

- the adapter's **field-routing rules** (which service field lands in
  which matcher slot), and
- the matcher's **scoring behaviour** on the projected records.

In the person and worker services this is
[`tests/duplicate_detection.rs`](../../person/person-service-rust-crate/tests/duplicate_detection.rs):
each test builds one or two service-side records, projects them through
`matching::adapter::to_matcher_person`, runs the canonical
`MatchingEngine`, and asserts on `MatchResult { score, is_match,
confidence, breakdown }`. It is un-gated (no DB) and black-box. The
person suite has 18 tests, the worker suite 15; categories include
identical / near-duplicate scoring, deterministic short-circuits
(NHS number, tax-ID → US-SSN routing, passport books), the full
national-ID scheme-routing audit (all 26 matcher slots reached from a
`system`-URI fragment), negative cases, field-routing pinning, and
config-preset invariants (strict ⊆ lenient).

Add a bridge test whenever the adapter gains a routing rule, the matcher
exposes a new scoring component the service surfaces, or a regression
escapes the adapter's own `#[cfg(test)]` module.

The newer loco.rs services (care-pathway, organization, case) store the
matcher type **verbatim** as JSONB, so there is no adapter to drift; the
equivalent bridge test is
[`tests/matching.rs`](../../care-pathway/care-pathway-service-rust-crate/tests/matching.rs),
which proves the crate really embeds the canonical matcher (a shared
guideline-id deterministically scores `1.0`) and that the payload JSON
round-trips for JSONB storage. It is also un-gated.

### 1.3 DB-gated request / integration tests

These boot the whole application and drive real HTTP request/response
cycles against a live PostgreSQL. They are the only Rust layer that
needs infrastructure, and they are gated so plain `cargo test` stays
green DB-free — see §2.

### 1.4 Doc tests and benchmarks

`cargo test --doc` compiles and runs the examples in `///` doc comments
(e.g. the adapter-usage snippet in the service `restful` docs), keeping
public-API examples honest. Benchmarks use **Criterion** and live under
`benches/` (`matching_bench`, `search_bench`, `validation_bench`); run
all with `cargo bench`, or one suite with `cargo bench -- matching`.
Benchmarks are not part of the pass/fail gate.

---

## 2. The DB-gated convention in detail

The request/integration layer is opt-in by construction. Three
mechanisms combine to keep the default test run infrastructure-free
while still exercising the database path on demand.

### 2.1 `#[ignore]` keeps `cargo test` DB-free

Every test that needs Postgres carries an `#[ignore = "..."]` attribute
with a message naming the requirement and the opt-in command, e.g.:

```rust
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_create_care_pathway() { /* ... */ }
```

Plain `cargo test` skips these, so it is green on a database-less
machine. `cargo test -- --ignored` runs exactly the ignored set; CI runs
the whole suite with a Postgres service attached (§6). Where a contract
matters both ways, it is pinned **twice** — once un-gated and once
DB-backed. For example the blank-name → `422` rule is asserted by a
DB-free controller unit test *and* by a DB-gated request test, so the
validation contract has coverage even when the database is absent.

### 2.2 `#[serial]` for env / DB ordering

Request tests share one process-wide resource: the test database (and,
for some, process environment such as `DATABASE_URL`). Running them
concurrently would let one test's truncate or insert race another's
reads. They are therefore annotated `#[serial]` (from `serial_test`) so
the gated suite runs one test at a time against the shared database.
The un-gated unit and bridge tests have no shared mutable state and run
fully parallel.

### 2.3 CI provisions a per-service test database

Each service owns its own test database, named per service
(`person_service_test`, `worker_service_test`,
`care_pathway_service_test`, …) so concurrent CI jobs never collide.
In CI a `postgres` service container is started and the database URL is
handed to the test job via `DATABASE_URL` (§6). Locally the same URL is
exported by hand. Connection / pool sizing and the PostgreSQL version
baseline (18) are covered in
[postgresql §12](../postgresql/index.md).

### 2.4 `dangerously_truncate` / recreate between tests

The loco services run the gated suite against the `test` environment,
whose `config/test.yaml` sets the dangerous dev/test-only flags so each
boot starts from a clean schema:

```yaml
database:
  auto_migrate: true            # run migrations up on load
  dangerously_truncate: true    # truncate tables on load (test/dev only)
  dangerously_recreate: true    # recreate schema on load (test/dev only)
```

These flags are guarded to `test`/`development` and must never appear in
`production.yaml`. Combined with `#[serial]`, they give each request
test a deterministic, isolated starting state. The loco request harness
(`loco_rs::testing::request::<App, _, _>`) boots the app against this
environment and yields a `request` client plus the `AppContext`.

---

## 3. Front-end testing (SvelteKit)

Each entity has a sibling `*-front-end-with-svelte` SPA. Per the
accepted front-end-drift decision there is no shared test package; each
front-end copy-adapts the same two-layer setup. The reference is the
[care-pathway front-end](../../care-pathway/care-pathway-front-end-with-svelte/).

| Layer | Tool | Location | Command |
| ----- | ---- | -------- | ------- |
| Unit | **vitest** (jsdom) | `tests/unit/**/*.test.ts` | `pnpm test` (`vitest run`) |
| E2E smoke | **Playwright** (chromium) | `tests/e2e/*.spec.ts` | `pnpm test:e2e` |

### 3.1 Vitest unit tests

Vitest runs in a `jsdom` environment over the lean client layer that
every front-end carries: the `ApiClient` fetch wrapper (`client.ts`),
the entity repository (`care-pathways.ts` — CRUD + check-duplicates +
merge), the typed payloads (`types.ts`), config (`config.ts`), and any
auth/stores. The backend is never contacted: `fetch` is stubbed so a
broken request shape fails the assertion locally. Strict TypeScript
(`noUncheckedIndexedAccess`) plus `svelte-check` (`pnpm run check`,
expected 0/0) act as a static gate alongside the unit tests.

### 3.2 Playwright e2e smoke tests

Playwright drives a real browser over the handful of SPA routes (list /
create / detail / edit). The backend is stubbed **per-test** with
`page.route`, so no running Rust service is required — a wrong path or
method surfaces as an unhandled request and a failed assertion.

### 3.3 Why e2e runs against `vite preview`, not `vite dev`

The Playwright `webServer` builds the app and serves the **production
build** through `vite preview` rather than running `vite dev`:

```ts
webServer: {
  command: "npm run build && npm run preview -- --port 4173 --strictPort",
  url: "http://localhost:4173",
  // ...
}
```

The reason is a `vite dev` cold-start race. In dev mode Vite serves
unbundled ES modules and performs **dependency optimization** (esbuild
pre-bundling) lazily on first request; under a freshly launched browser
hitting the page immediately, module loading can race that
optimization/transform step and intermittently fail to load a module,
flaking the e2e run. `vite preview` serves the already-built, static,
correctly-typed ES modules with no on-the-fly optimization, so module
loading is deterministic. Testing the production build also matches what
ships.

> Note: a pre-existing build blocker (a Lily design-system dependency /
> static-symlink issue that prevented `vite build` from completing) has
> been fixed, so the build-then-preview harness runs end to end.

---

## 4. Conformance and quality gates

Beyond behavioural tests, every Rust crate must clear non-negotiable
formatting, lint, and binary-conformance gates. These are enforced in CI
(§6) and runnable locally.

| Gate | Command | Enforces |
| ---- | ------- | -------- |
| Formatting | `cargo fmt --all -- --check` | Canonical rustfmt formatting (no diffs) |
| Lints | `cargo clippy --all-targets --all-features -- -D warnings` (with `#![warn(clippy::pedantic)]` at each crate root) | Clippy clean across **every** crate — services, matchers, the verifier, migrations, and case-folder all produce zero output; `-D warnings` makes any warning fatal. `--all-targets` lints tests/examples/benches too, so the no-allow invariant covers the harness. This is the single, identical clippy command in every service CI workflow |
| Binary conformance | (compile-time) | `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`, and the `target_env = "musl"` MiMalloc global allocator block at the top of `lib.rs` / `main.rs` |
| Doc examples | `cargo test --doc` | `///` examples compile and pass |
| Markdown links | link check | Cross-doc relative links in `spec/**` and `AGENTS/**` resolve on disk |

The binary-conformance attributes are required at the top of every crate
root (`#![forbid(unsafe_code)]` / `#![deny(missing_docs)]`), so `deny`
means an undocumented public item is a *build* failure, not a lint —
documentation drift cannot merge. The `musl`-gated MiMalloc allocator
keeps the static-build conformance from the stack convention. Markdown
link integrity is part of the doc discipline: this very document was
written only after confirming each cross-link target exists on disk.

**No-suppression invariant.** The clippy gate is met by *fixing* lints,
not silencing them: there is **no `#[allow(clippy::…)]` / `#![allow(clippy::…)]`
anywhere in the tree** (verified by `grep -rn 'allow(clippy' --include='*.rs'`
returning nothing), and the gate may not be relaxed by relocating an
allow to a `Cargo.toml [lints]` table or to `#[expect]`. Pedantic lints
are resolved at the source — `doc_markdown` backticks, `#[must_use]`,
`# Errors`/`# Panics` docs, `TryFrom` instead of `as`, `Option<&T>`
params, `f64::EPSILON` comparisons in tests, and helper extraction for
`too_many_lines`. Test, example, and benchmark harness files are held to
the same bar (no harness-local allows). This keeps the lint signal
honest: a new pedantic finding must be fixed, never annotated away.

---

## 5. Verifiable without infrastructure vs infra-gated

Set expectations about what a given command can prove.

**Verifiable with no infrastructure** (default `cargo test`, `pnpm
test`, fmt/clippy/check):

- All matching, validation, privacy, normalization, phonetic, and
  serialization logic (unit tests).
- The service ↔ matcher contract end-to-end through the scorer (bridge
  tests) — the most behaviourally rich layer, and it needs nothing.
- Auth crypto (RS256 verification, JWKS parsing, claim checks).
- The OpenAPI document shape and the JSONB storage round-trip.
- Front-end client / repository / type logic and route smoke flows
  (backend stubbed).
- All conformance gates (fmt, clippy, doc tests, binary attributes).

**Infra-gated** (explicit opt-in, CI):

- HTTP request/response cycles, persistence, migrations, soft-delete,
  audit-log writes, and event publication that depend on a live
  database — the `#[ignore]`d request tests, run with
  `cargo test -- --ignored` against Postgres.
- A durable event broker is **not** exercised in tests: streaming is
  in-memory (Phase 1 envelope + `EventPublisher` seam), so event
  publication is asserted in-process, not against a real bus. See
  [event-streaming](../event-streaming/index.md).
- Full-text **search** beyond the un-gated Tantivy temp-index unit
  tests, and the `ILIKE` name search on the loco services, exercise the
  database in the gated layer. See [search](../search/index.md).

The headline: the richest correctness signal (bridge + unit) is
infra-free; the database adds coverage of the wiring, not the core
algorithms.

---

## 6. CI workflows

Each service crate ships a GitHub Actions workflow (the reference is
[care-pathway-service `.github/workflows/ci.yaml`](../../care-pathway/care-pathway-service-rust-crate/.github/workflows/ci.yaml)),
triggered on push to the default branch and on every pull request. It is
three jobs:

| Job | Step | Maps to |
| --- | ---- | ------- |
| `rustfmt` | `cargo fmt --all -- --check` | §4 formatting |
| `clippy` | `cargo clippy --all-targets --all-features -- -D warnings` | §4 lints |
| `test` | `cargo test --all-features --all`, with a `postgres` **service container** and `DATABASE_URL` in the job env | §1–§2 (un-gated + DB-gated together) |

The `test` job is where the gating pays off: because CI provisions a
Postgres service (with a `pg_isready` health check) and exports
`DATABASE_URL` for the named per-service test database, the full run
(plain tests **plus** the `#[ignore]`d request tests when invoked with
`--ignored`, or all when wired that way) executes against real
persistence — while local `cargo test` without that container stays
green DB-free. The clippy and fmt jobs need no services and run on every
push regardless.

> Some older non-loco crates historically split this across
> `test.yml` / `quality.yml` / `security.yml` (unit + doc tests, then a
> separate PostgreSQL-backed integration job); the consolidated
> three-job `ci.yaml` above is the current convention. The contract is
> the same either way: fmt + clippy gate on every change, and a
> Postgres-backed job runs the database layer.

---

## 7. Adding tests — quick rules

1. **Default-green rule.** A new test that needs a database, broker, or
   network must be `#[ignore]`d (Rust) or stubbed (front-end). Never
   make plain `cargo test` / `pnpm test` require infrastructure.
2. **Pin contracts twice when they matter both ways.** A
   validation/status-code contract worth a request test is usually also
   worth a DB-free unit/controller test.
3. **Bridge tests for the seam.** New adapter routing or a new surfaced
   matcher component gets a bridge-test assertion, not just a unit test
   inside the adapter.
4. **Serial for shared state.** Any test that touches the shared test
   database or process env is `#[serial]`.
5. **Spec-first.** A behavioural change is a three-part PR — spec edit +
   code edit + test edit — per the SDD discipline.
