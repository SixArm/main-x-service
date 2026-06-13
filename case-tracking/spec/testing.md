# Testing strategy (project-level)

> Part of the [Case Tracking specification](index.md). Per-edition
> detail: [loco testing](../case-tracker-service-with-rust/spec/testing.md),
> [svelte testing](../case-tracker-front-end-with-svelte/spec/testing.md).

## Principles

- **Test against the contract, not the implementation.** Request tests
  assert status codes + JSON shapes, never HTML strings.
- **Stub the upstreams, don't mock the boundary you're testing.** Both
  editions run against the Loco edition's in-process `StubClient`
  upstreams so a full round-trip is exercised without standing up five
  real services.
- **NHS Number validation is unit-tested on both sides** with the same
  worked examples (see [nhs-number.md](nhs-number.md)).
- **Hermetic e2e.** Front-end tests generate fresh Modulus-11-valid NHS
  Numbers and unique titles so runs don't collide on shared state.

## Stub mode (the shared test harness)

The Loco edition's `USE_UPSTREAM_STUBS=1` swaps every Main-X-Service for
an in-process `StubClient` and seeds it with demo data. The API responds
normally; clients can't tell the difference. This is the prerequisite
for the Svelte Playwright suite and for local UI iteration.

```bash
cd case-tracker-service-with-rust
USE_UPSTREAM_STUBS=1 cargo run -- start   # JSON API on :5150, seeded
```

## CI gates (both editions green before merge)

| Edition | Gates                                                                                 |
| ------- | ------------------------------------------------------------------------------------- |
| Loco    | `cargo check` · `cargo clippy -- -D warnings` · `cargo fmt --check` · `cargo test`    |
| Svelte  | `npm run check` · `npm run build` · `npm run test:e2e` (against stub-mode API)        |

See each subproject's `spec/testing.md` for the full test inventory.
