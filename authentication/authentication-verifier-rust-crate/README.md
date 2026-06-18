# authentication-verifier

Offline **PASETO v4.public verification** for Main X Index peer services.

The [authentication-service](../authentication-service-with-loco) is
the federation's single sign-on provider: it exchanges a server-side
cookie session for short-lived **PASETO v4.public** (Ed25519-signed)
cross-service tokens, and publishes its Ed25519 public key(s) at
`/.well-known/paseto-keys`. Every other service verifies those tokens
**offline** with this crate — fetch the key set once at boot, build a
`Verifier`, then verify each bearer token locally. No shared secret, no
per-request introspection hop.

> **v0.2.0 pivots from RS256 JWT to PASETO v4.public** (BREAKING). Per the
> canonical design
> ([authentication-sessions.md](../../agents/share/authentication-sessions.md)),
> JWT is removed from the auth path: the session is a Postgres-backed
> cookie session and the only cross-service token is a short-lived PASETO.
> See [CHANGELOG.md](./CHANGELOG.md) for the migration note.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Design: [authentication-sessions.md](../../agents/share/authentication-sessions.md) §5
- Issuing service: [authentication-service-with-loco](../authentication-service-with-loco)

## Quick start

```toml
[dependencies]
authentication-verifier = "0.2"
# or, to fetch the key set over HTTPS at boot:
# authentication-verifier = { version = "0.2", features = ["fetch"] }
# in-monorepo alternative (path dependency):
# authentication-verifier = { path = "../authentication-verifier-rust-crate" }
```

```rust
use authentication_verifier::{Verifier, Claims, VerifyError};

// Boot: build from a paseto-keys document you already have…
let keys: serde_json::Value = serde_json::from_str(keys_body)?;
let verifier = Verifier::from_paseto_keys_value(&keys, "authentication-service", "main-x-service")?;

// …or fetch it over HTTPS (requires the `fetch` feature):
// let verifier = Verifier::from_paseto_keys_url(
//     "https://auth.example.gov/.well-known/paseto-keys",
//     "authentication-service",
//     "main-x-service",
// ).await?;

// Per request: verify the bearer token locally.
let claims: Claims = verifier.verify(bearer_token)?;
println!("authenticated subject: {}", claims.sub); // the user pid (UUID)
```

Construct the `Verifier` once and share it behind an `Arc`; `verify`
is read-only and allocation-light.

## API summary

| Item | Purpose |
|---|---|
| `Verifier::from_paseto_keys_value(&keys, issuer, audience)` | Build from an in-memory `paseto-keys` document. Loads Ed25519 public keys only (other algorithms are skipped); an empty key set is permitted and rejects every token with `UnknownKid`. |
| `Verifier::from_paseto_keys_url(url, issuer, audience).await` | Fetch the key set over HTTPS, then build. **Requires `features = ["fetch"]`** (pulls in `reqwest` with rustls). |
| `Verifier::verify(token) -> Result<Claims, VerifyError>` | Select the key by the token **footer `kid`**, check the PASETO v4.public (Ed25519) signature, then enforce `iss`, `aud`, `exp`, and `nbf`. |
| `Verifier::key_count() -> usize` | Number of Ed25519 keys loaded from the key set. |
| `Claims` | Verified claims: `sub` (user pid, UUID string), `iss`, `aud`, `iat`, `nbf`, `exp`, `sid` (originating auth-service session), `scope`/`roles`. |

### `VerifyError` variants

| Variant | Meaning |
|---|---|
| `Keys(String)` | The key-set document was missing or structurally invalid (no key array; Ed25519 entry missing `kid` or public-key material; bad key bytes). |
| `MissingKid` | The token footer carries no `kid`, so no key can be selected. |
| `UnknownKid(String)` | No loaded key matches the token's `kid` — stale key cache, wrong issuer, or forgery. Refetch the key set to pick up a key rotation. |
| `Paseto(String)` | Signature, issuer, audience, expiry, or not-before validation failed (or the token was unparseable). |
| `Fetch(String)` | Fetching the key set over HTTP failed (only with the `fetch` feature). |

## The paseto-keys / `kid` contract

This crate is the peer-side mirror of the auth-service's own token
verification: same PASETO v4.public format, same `Claims` shape, same
footer-`kid` selection — but keyed off the *published* key set rather
than a locally held key, so any service can embed it.

- The service publishes Ed25519 public keys at `/.well-known/paseto-keys`,
  each entry carrying a `kid` and the base64url (no padding) public-key
  bytes.
- Tokens are **PASETO v4.public**; the **footer** carries the `kid` that
  selects the verifier key — a stable identifier, set on every token.
- `Claims` is duplicated **byte-identically** between the service and
  this crate by convention; the service's cross-crate contract test
  pins the round-trip.
- Default issuer/audience: `authentication-service` /
  `main-x-service`.
- Revocation (signout) is enforced at the auth service via the session.
  Peers honour a token until `exp`; the service keeps TTLs short
  (default ~300 s) to bound that window.

## Testing

```bash
cargo test                  # offline unit tests (throwaway Ed25519 keypair)
cargo test --features fetch # also compile the HTTP loader
```

## License

MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
