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

> **v0.3.0 adds ABAC** (additive). Per
> ([authorization-attributes.md](../../agents/share/authorization-attributes.md)),
> verified `Claims` carry subject attributes in the new `attrs` claim,
> and the `abac` module ships the family's shared, pure policy engine
> (`Policy::evaluate` over attrs + action + entity, first-match-wins,
> default allow-read / deny-mutation). `scope` / `roles` are deprecated
> for authorization. Pre-0.3 tokens verify unchanged (`attrs` ⇒ empty).

> **v0.4.0 adds record-level resource attributes** (additive). Per §9,
> `Policy::evaluate_with_resource` feeds attributes of the specific
> loaded record into the decision via `resource.*` `when` keys (e.g.
> deny write on a high-sensitivity record unless `access=admin`).
> `Policy::evaluate` is unchanged.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Design: [authentication-sessions.md](../../agents/share/authentication-sessions.md) §5,
  [authorization-attributes.md](../../agents/share/authorization-attributes.md)
- Issuing service: [authentication-service-with-loco](../authentication-service-with-loco)

## Quick start

```toml
[dependencies]
authentication-verifier = "0.4"
# or, to fetch the key set over HTTPS at boot:
# authentication-verifier = { version = "0.4", features = ["fetch"] }
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

### Authorization (ABAC)

After verifying, decide **what the caller may do** with the shared
policy engine — the same code all nine entity services embed in their
blanket `/api/*` guards:

```rust
use authentication_verifier::{Action, Policy};

// Boot: load the configured policy, falling back to the built-in
// default (svc=true ⇒ everything; access=admin ⇒ destructive+write;
// access=write ⇒ write; otherwise read-only).
let policy = std::env::var("PLACE_ABAC_POLICY")
    .ok()
    .and_then(|json| Policy::from_json(&json).ok())
    .unwrap_or_else(Policy::default_policy);

// Per request: derive the action from the HTTP method (plus the
// crate's destructive named POSTs), then evaluate.
let decision = policy.evaluate(&claims, Action::Write, "place");
if !decision.allowed {
    // 403 — the credential is valid but the policy denied it.
    eprintln!("forbidden: {}", decision.reason);
}
```

`401` means missing/bad credential (verification failed); `403` means
valid credential, policy denied. See
[authorization-attributes.md](../../agents/share/authorization-attributes.md)
for the attribute model, policy language, and default policy.

## API summary

| Item | Purpose |
|---|---|
| `Verifier::from_paseto_keys_value(&keys, issuer, audience)` | Build from an in-memory `paseto-keys` document. Loads Ed25519 public keys only (other algorithms are skipped); an empty key set is permitted and rejects every token with `UnknownKid`. |
| `Verifier::from_paseto_keys_url(url, issuer, audience).await` | Fetch the key set over HTTPS, then build. **Requires `features = ["fetch"]`** (pulls in `reqwest` with rustls). |
| `Verifier::verify(token) -> Result<Claims, VerifyError>` | Select the key by the token **footer `kid`**, check the PASETO v4.public (Ed25519) signature, then enforce `iss`, `aud`, `exp`, and `nbf`. |
| `Verifier::key_count() -> usize` | Number of Ed25519 keys loaded from the key set. |
| `Claims` | Verified claims: `sub` (user pid, UUID string), `iss`, `aud`, `iat`, `nbf`, `exp`, `sid` (originating auth-service session), `attrs` (ABAC subject attributes; empty on pre-0.3 tokens), plus `scope`/`roles` (deprecated for authorization). |
| `Policy` / `Rule` / `Action` / `Decision` (the `abac` module, re-exported at the root) | The shared ABAC engine: `Policy::from_json` loads a configured policy, `Policy::default_policy()` is the built-in coarse tier, `Policy::evaluate(&claims, action, entity)` decides first-match-wins with default allow-read / deny-mutation. Pure — no I/O, no clock, no panics. |
| `Policy::evaluate_with_resource(&claims, action, entity, &resource)` | As `evaluate`, plus a `BTreeMap<String, Vec<String>>` of record-level **resource attributes** matched by `resource.*` `when` keys (v0.4). A service passes attributes of the record it just loaded so policies can gate on e.g. record sensitivity. |

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
