# Testing — Place entity

Each subproject runs its own pyramid; the entity layer adds the
contract seams. Run the suite of every subproject you touched, plus
the bridge tests if you went near the adapter.

## Per-subproject

| Subproject | Run | Inventory | Guide |
|---|---|---|---|
| place-service | `cargo test` (+ `cargo bench`) | 104 unit + 67 integration + 14 bridge tests + 16 Criterion benchmarks | [service AGENTS/testing.md](../place-service-with-loco/AGENTS/testing.md) |
| place-matcher | `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo doc --no-deps` | unit + integration + property tests + doctests; all three commands must be clean before declaring success | [matcher AGENTS/testing.md](../place-matcher-rust-crate/AGENTS/testing.md) |
| place-front-end | `pnpm test` (Vitest) + `pnpm test:e2e` (Playwright) + `pnpm check` | 8 unit (mocked fetch) + 6 e2e smoke (run without a live service) | [front-end AGENTS/testing.md](../place-front-end-with-svelte/AGENTS/testing.md) |

## Contract seams (what the entity cares about)

### Service ↔ matcher: the bridge suite

```bash
cd place-service-with-loco
cargo test --test duplicate_detection
```

[`tests/duplicate_detection.rs`](../place-service-with-loco/tests/duplicate_detection.rs)
drives service records through `adapter::to_matcher_place` into
`MatchingEngine::match_places`, pinning both the adapter routing rules
(entity [spec §5.3](../spec/05-domain-model.md)) and the matcher's
scoring: identical clones ≥ 0.95, GLN / OSM deterministic
short-circuits, geo-distance ranking, field-routing pins, negative
cases, config presets. **Add a bridge test whenever the adapter gains
a routing rule or the matcher exposes a new scored component the
service surfaces** (entity FR-19).

### Front-end ↔ service: wire-type tests

`tests/unit/client.test.ts` + `tests/unit/places.test.ts` pin envelope
unwrapping and repository behaviour against `src/lib/api/types.ts`.
They mock `fetch` — they prove the front-end matches its *copy* of the
contract, not the live service.

## Known gaps

- **No live trio test** (front-end → service → database) — entity
  [spec §13](../spec/13-tasks.md) E-9.
- **Front-end `pnpm install` / `pnpm test` unverified** — E-8.
- Wire types are hand-mirrored, not schema-checked — entity
  [spec §16](../spec/16-open-questions.md) OQ-4.

## Conventions

- Well-known synthetic places for readability (Central Park, Eiffel
  Tower); realistic coordinates; valid 13-digit GLNs.
- **No real personal data** anywhere in fixtures — phones from
  drama-reserved / fictitious ranges, `example.org` emails (matcher
  [AGENTS/security-and-privacy.md](../place-matcher-rust-crate/AGENTS/security-and-privacy.md)).
