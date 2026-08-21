# Subprojects — Authentication Entity

Unlike the sibling entities (service + matcher + front-end), this
entity is service + **verifier library** + front-end. There is no
matcher — nothing to match.

> **Auth model source of truth:**
> [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> The session is a server-side httpOnly **cookie session**; cross-service
> auth is short-lived **PASETO v4.public** (published Ed25519 key at
> `/.well-known/paseto-keys`). RS256 JWT + JWKS are **decommissioned**.
> **Pivot in progress** — the service code follow-up is tracked in the
> service spec §13; the verifier and front-end are already harmonized.

## The three subprojects

| Subproject | Kind | Responsibility |
|---|---|---|
| [authentication-service-with-loco](../authentication-service-with-loco/) | loco.rs web service | **Issuance.** Magic-link sign up / sign in / sign out, cookie-session establishment, short-lived PASETO v4.public minting, published-key publication at `/.well-known/paseto-keys`, session recording + revocation. Also the family's reference loco.rs application. *(RS256-era runtime until the spec §13 follow-up.)* |
| [authentication-verifier-rust-crate](../authentication-verifier-rust-crate/) | Rust library | **Verification.** Offline PASETO v4.public verification against the published Ed25519 key(s): `kid`-based key selection, signature + `iss`/`aud`/`exp` checks. Dependency-light by design; `fetch` feature adds HTTP key loading via reqwest. (Already harmonized.) |
| [authentication-front-end-with-svelte](../authentication-front-end-with-svelte/) | SvelteKit SPA + BFF | **Operator UI.** `/signup`, `/signin`, `/verify`, `/` (dashboard + sign out). The SvelteKit server acts as a **BFF**: the browser holds only the httpOnly `__Host-mxi_session` cookie (no token, no `localStorage`); the BFF mints/forwards the PASETO server-side. Deliberately dependency-light — no data grid, no Lily/SVAR. (Already harmonized.) |

## Dependency direction

```
peer services (person, worker, place, ...)
        │  embed (Cargo dependency)
        v
authentication-verifier ──────► authentication-service
        (trusts, at runtime)      /.well-known/paseto-keys
                                        ^
                                        │  REST (issuance only; via BFF)
                          authentication-front-end-with-svelte
```

- **Peers depend on the verifier** at compile time and on nothing else
  in this entity at request time.
- **The verifier depends on the service's published Ed25519 key(s)** at
  boot time only — fetch `/.well-known/paseto-keys` once, cache for the
  process lifetime, refetch on `UnknownKid` after a key rotation.
- **The front-end (BFF) depends on the service's REST API** for the
  sign-in flow; it holds the cookie session and mints a short-lived
  PASETO per outbound call. Peer front-ends never see a token in the
  browser.
- Service and verifier mirror the `Claims` struct **by convention**,
  pinned by the entity spec §5.3 and by the service's cross-crate
  contract test (`tests/sign_verify_contract.rs`, which takes the
  verifier as a dev-dependency — entity spec §13 T-4). There is no
  runtime code dependency between them.

## How to run each

### Service (needs PostgreSQL)

```bash
cd authentication-service-with-loco
export DATABASE_URL=postgres://loco:loco@localhost:5432/authentication_service_development
cargo loco start          # http://localhost:5150 (auto-migrates in dev)
cargo test --lib          # DB-free unit tests
cargo clippy --bins
```

Magic links are printed to the service console in development (no
SMTP). Dev signing keys are committed under `config/keys/` (RSA in the
RS256-era runtime; Ed25519 PASETO is the target per the service spec §13).

### Verifier (library — no run target)

```bash
cd authentication-verifier-rust-crate
cargo test                          # offline unit tests
cargo test --features fetch         # include the HTTP published-key path
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
