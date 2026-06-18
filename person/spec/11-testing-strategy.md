## 11. Testing Strategy

Each subproject owns its test pyramid; the entity level cares about
the **seams** — the two integration contracts (§5.3, §5.4) must each
be pinned by tests on at least one side.

### 11.1 Per-subproject inventories

| Subproject | Layers | Reference |
|---|---|---|
| person-service | ~100 unit tests (matching, search, validation, privacy, models) + 7 HTTP integration tests + **14 bridge tests** + 3 Criterion benchmark suites | [service spec §11](../person-service-with-loco/spec/11-testing-strategy.md), [AGENTS/testing.md](../person-service-with-loco/AGENTS/testing.md) |
| person-matcher | Pure-library suite: `cargo test` green on a fresh checkout, doctests compile, clippy `-D warnings` clean; no PII in fixtures | [matcher spec §18](../person-matcher-rust-crate/spec/18-testing-strategy.md), [AGENTS/testing.md](../person-matcher-rust-crate/AGENTS/testing.md) |
| person-front-end | 8 vitest unit tests (ApiClient + PersonRepository, mocked fetch) + 6 playwright e2e smoke (API-down resilience) + 9 playwright integration golden-paths against a live service | [front-end spec §11](../person-front-end-with-svelte/spec/11-testing-strategy.md) |

### 11.2 Seam 1 — service ↔ matcher (bridge tests)

[`tests/duplicate_detection.rs`](../person-service-with-loco/tests/duplicate_detection.rs)
in the service crate pins **both sides** of the adapter contract:
field-routing rules (telecom → phone/email, address renames,
identifier-system-URI routing, tax-ID default) **and** the matcher's
scoring output (identical-clone ≥ 0.95, deterministic short-circuits,
ordering invariants, strict ⊆ lenient). A regression on either side
fails here first. Run: `cargo test --test duplicate_detection`.

Rule: when the adapter gains a routing rule or the matcher exposes a
new scoring component the service surfaces, a bridge test MUST land in
the same PR.

### 11.3 Seam 2 — front-end ↔ service (golden paths)

`tests/integration/golden-paths.spec.ts` in the front-end drives the
live SvelteKit preview against a running service over real HTTP: list,
create + 409 surfacing, detail, edit, soft delete, match breakdown,
merge, audit. Idempotent (timestamped names + REST `DELETE` cleanup).
Run: `bin/e2e` (health-checks the service first).

Known blocker: end-to-end validation against a live service is
blocked on a pre-existing service issue — front-end spec §16 OQ-5;
tracked here as §13 E-2.

### 11.4 Entity-level acceptance

A change to the cross-subproject contract (§5.3–§5.5, §6 FR-19–FR-21)
is **done** when:

1. This spec is edited,
2. The owning subproject's spec + code + tests are edited (three-part
   PR), and
3. The relevant seam suite (§11.2 or §11.3) passes.
