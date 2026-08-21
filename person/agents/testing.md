# Testing — Entity-Level Orientation

Each subproject owns its pyramid; the entity owns the **two seams**.
Normative strategy: entity [spec §11](../spec/11-testing-strategy.md).

## Per-subproject suites

| Subproject | Run | Shape | Guide |
|---|---|---|---|
| person-service | `cargo test --lib` · `cargo test --test api_integration_test` (needs PostgreSQL) · `cargo bench` | ~100 unit + 7 HTTP integration + 3 Criterion suites | [service agents/testing.md](../person-service-with-loco/agents/testing.md) |
| person-matcher | `cargo test` (fresh checkout, no env vars) | Pure-library suite + doctests; clippy `-D warnings` clean; **no PII in fixtures** | [matcher agents/testing.md](../person-matcher-rust-crate/agents/testing.md) |
| person-front-end | `pnpm test` (vitest) · `pnpm test:e2e` (smoke, no service) · `bin/e2e` (integration, live service) | 8 unit + 6 smoke + 9 golden-path integration | [front-end README — Testing](../person-front-end-with-svelte/README.md) |

## Seam 1 — service ↔ matcher (bridge suite)

```bash
cd person-service-with-loco
cargo test --test duplicate_detection
```

14 black-box tests driving service records through
`adapter::to_matcher_person` into `MatchingEngine::match_persons`.
Pins **both** the adapter's field routing and the matcher's scoring
(identical clones, deterministic short-circuits, NHS-URI routing,
strict ⊆ lenient, sparse-record edges). Add a bridge test whenever the
adapter gains a routing rule or the service surfaces a new matcher
component.

## Seam 2 — front-end ↔ service (golden paths)

```bash
cd person-service-with-loco && podman compose up -d   # service at :8080
cd ../person-front-end-with-svelte && bin/e2e          # health-checks, then playwright
```

9 idempotent tests over real HTTP: list, create, 409 surfacing,
detail, edit, soft delete, match breakdown, merge, audit. Currently
blocked on a pre-existing service issue — entity
[spec §13 E-2](../spec/13-tasks.md).

## Entity-level done-ness

A seam-touching PR is done when: entity spec edited, owning crate
spec + code + tests edited, and the relevant seam suite is green. A
single-subproject PR follows that subproject's own testing guide and
does not need this page.
