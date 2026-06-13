# authentication-verifier

Offline **RS256 JWT verification** for Main X Index peer services.

The [authentication-service](../authentication-service-rust-crate) is
the federation's single sign-on provider: it signs RS256 access tokens
and publishes its RSA public keys at `/.well-known/jwks.json`. Every
other service verifies those tokens **offline** with this crate — fetch
the JWKS once at boot, build a `Verifier`, then verify each bearer
token locally. No shared secret, no per-request introspection hop.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Issuing service: [authentication-service-rust-crate](../authentication-service-rust-crate)

## Quick start

```toml
[dependencies]
authentication-verifier = { path = "../authentication/authentication-verifier-rust-crate" }
# or, to fetch the JWKS over HTTPS at boot:
# authentication-verifier = { path = "...", features = ["fetch"] }
```

```rust
use authentication_verifier::{Verifier, Claims, VerifyError};

// Boot: build from a JWKS document you already have…
let jwks: serde_json::Value = serde_json::from_str(jwks_body)?;
let verifier = Verifier::from_jwks_value(&jwks, "authentication-service", "main-x-service")?;

// …or fetch it over HTTPS (requires the `fetch` feature):
// let verifier = Verifier::from_jwks_url(
//     "https://auth.example.gov/.well-known/jwks.json",
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
| `Verifier::from_jwks_value(&jwks, issuer, audience)` | Build from an in-memory JWKS document. Loads RSA keys only (non-`RSA` `kty` entries are skipped); an empty key set is permitted and rejects every token with `UnknownKid`. |
| `Verifier::from_jwks_url(url, issuer, audience).await` | Fetch the JWKS over HTTPS, then build. **Requires `features = ["fetch"]`** (pulls in `reqwest` with rustls). |
| `Verifier::verify(token) -> Result<Claims, VerifyError>` | Select the key by the token header `kid`, check the RS256 signature, then enforce `iss`, `aud`, and `exp`. |
| `Verifier::key_count() -> usize` | Number of RSA keys loaded from the JWKS. |
| `Claims` | Verified claims: `sub` (user pid, UUID string), `email`, `name`, `iss`, `aud`, `exp`, `iat`, `jti` (= the auth-service `sessions.jid`). |

### `VerifyError` variants

| Variant | Meaning |
|---|---|
| `Jwks(String)` | The JWKS document was missing or structurally invalid (no `keys` array; RSA entry missing `kid`/`n`/`e`; bad modulus/exponent). |
| `MissingKid` | The token header carries no `kid`, so no key can be selected. |
| `UnknownKid(String)` | No loaded key matches the token's `kid` — stale JWKS cache, wrong issuer, or forgery. Refetch the JWKS to pick up a key rotation. |
| `Jwt(jsonwebtoken::errors::Error)` | Signature, issuer, audience, or expiry validation failed. |
| `Fetch(String)` | Fetching the JWKS over HTTP failed (only with the `fetch` feature). |

## The JWKS / `kid` contract

This crate is the peer-side mirror of the auth-service's own token
verification: same RS256 algorithm, same `Claims` shape, same `kid`
selection — but keyed off the *published* JWKS rather than a locally
held key, so any service can embed it.

- The service publishes RSA signing keys as `{kty: "RSA", use: "sig",
  alg: "RS256", kid, n, e}` with `n`/`e` base64url (no padding).
- `kid` = base64url, no padding, of `SHA-256(big-endian RSA modulus
  bytes)` — a stable thumbprint, stamped into every token header.
- `Claims` is duplicated **byte-identically** between the service and
  this crate by convention; the service's cross-crate contract test
  pins the round-trip.
- Default issuer/audience: `authentication-service` /
  `main-x-service` (the service's `JWT_ISSUER` / `JWT_AUDIENCE`).
- Revocation (signout) is enforced only at the auth service. Peers
  honour a token until `exp`; the service keeps TTLs short (default
  3600 s) to bound that window.

## Testing

```bash
cargo test                  # offline unit tests (throwaway RSA keypair)
cargo test --features fetch # also compile the HTTP loader
```

## License

MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
