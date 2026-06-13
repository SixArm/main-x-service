# Subprojects — Authentication Entity

Unlike the sibling entities (service + matcher + front-end), this
entity is service + **verifier library** + front-end. There is no
matcher — nothing to match.

## The three subprojects

| Subproject | Kind | Responsibility |
|---|---|---|
| [authentication-service-rust-crate](../authentication-service-rust-crate/) | loco.rs web service | **Issuance.** Magic-link sign up / sign in / sign out, RS256 JWT signing, JWKS publication at `/.well-known/jwks.json`, session recording + revocation. Also the family's reference loco.rs application. |
| [authentication-verifier-rust-crate](../authentication-verifier-rust-crate/) | Rust library | **Verification.** Offline RS256 verification against the published JWKS: `kid`-based key selection, signature + `iss`/`aud`/`exp` checks. Dependency-light by design; `fetch` feature adds HTTP JWKS loading via reqwest. |
| [authentication-front-end-with-svelte](../authentication-front-end-with-svelte/) | SvelteKit SPA | **Operator UI.** `/signup`, `/signin`, `/verify`, `/` (dashboard + sign out). Stores the JWT in `localStorage` as the federation's bearer credential. Deliberately dependency-light — no data grid, no Lily/SVAR. |

## Dependency direction

```
peer services (person, worker, place, ...)
        │  embed (Cargo dependency)
        v
authentication-verifier ──────► authentication-service
        (trusts, at runtime)      /.well-known/jwks.json
                                        ^
                                        │  REST (issuance only)
                          authentication-front-end-with-svelte
```

- **Peers depend on the verifier** at compile time and on nothing else
  in this entity at request time.
- **The verifier depends on the service's JWKS** at boot time only —
  fetch once, cache for the process lifetime, refetch on
  `UnknownKid` after a key rotation.
- **The front-end depends on the service's REST API** for the
  sign-in flow; sibling front-ends reuse the stored token without
  talking to the auth service at all.
- Service and verifier mirror the `Claims` struct **by convention**,
  pinned by the entity spec §5.3 and by the service's cross-crate
  contract test (`tests/sign_verify_contract.rs`, which takes the
  verifier as a dev-dependency — entity spec §13 T-4). There is no
  runtime code dependency between them.

## How to run each

### Service (needs PostgreSQL)

```bash
cd authentication-service-rust-crate
export DATABASE_URL=postgres://loco:loco@localhost:5432/authentication-service_development
cargo loco start          # http://localhost:5150 (auto-migrates in dev)
cargo test --lib          # DB-free unit tests
cargo clippy --bins
```

Magic links are printed to the service console in development (no
SMTP). Dev RSA keys are committed under `config/keys/`.

### Verifier (library — no run target)

```bash
cd authentication-verifier-rust-crate
cargo test                          # 9 offline unit tests
cargo test --features fetch         # include the HTTP JWKS path
```

Embed in a peer service:

```toml
[dependencies]
authentication-verifier = { path = "../../authentication/authentication-verifier-rust-crate", features = ["fetch"] }
```

### Front-end (needs the service running)

```bash
cd authentication-front-end-with-svelte
cp .env.example .env      # PUBLIC_API_BASE_URL=http://localhost:5150
pnpm install
pnpm dev                  # http://localhost:5173
pnpm run check            # svelte-check, strict
```

## See also

- [`verification.md`](verification.md) — the peer-side verification
  contract in detail.
- Entity spec [§8 Architecture](../spec/08-architecture.md) — flow
  diagrams and deployment topology.
