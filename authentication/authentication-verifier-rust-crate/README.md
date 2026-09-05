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

> **v0.5.0 adds ownership templates + environment attributes**
> (additive). A `when` value `$sub` / `$email` resolves to the caller's
> identity (`resource.owner: ["$sub"]` = owned by the caller), and
> `Policy::evaluate_with_context` adds an `env.*` namespace for
> request-time / network context (§4/§10). Prior methods unchanged.

> **v0.6.0 adds obligations** (additive). An allow rule may attach
> `obligations` (e.g. `"mask"`, `"audit"`) that the `Decision` carries
> for the enforcement point to honour — the engine does not interpret
> them (§11). `Decision::requires("mask")` is the convenience check.

> **v0.7.0 adds `ReloadablePolicy`** (additive). A thread-safe holder
> (`new` / `current` / `store`) that lets a service **hot-swap** the
> active policy at runtime — no restart — with a lock-light per-request
> read path. The reload trigger is the service's concern.

> **v0.8.0 adds `ReloadableVerifier`** (additive). The same holder shape
> for the `Verifier`, so a service can **hot-swap its key set for key
> rotation** — e.g. via a periodic re-fetch of `/.well-known/paseto-keys`
> — without a restart. Keep the current keys on a failed fetch.

> **Unreleased since v0.8.0.** `Cargo.toml` is still `0.8.0`; three
> further changes have landed in code/tests but not yet in a dated
> `CHANGELOG.md` release heading (see [CHANGELOG.md](./CHANGELOG.md)
> `[Unreleased]`): hardened `from_paseto_keys_url` (HTTPS-only except
> loopback HTTP, timeout, no redirects, a 64 KiB body cap), an ABAC fix
> so a negated `resource.`/`env.` rule can't match vacuously when no
> resource/environment was supplied, a `fuzz/` cargo-fuzz harness, and
> (2026-07-27) the verifier becoming **algorithm-agile**: a key naming
> an algorithm this build doesn't implement is kept and diagnosed as
> `UnsupportedAlgorithm` rather than silently dropped — see
> `spec/index.md` §5.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Design: [authentication-sessions.md](../../agents/share/authentication-sessions.md) §5,
  [authorization-attributes.md](../../agents/share/authorization-attributes.md)
- Issuing service: [authentication-service-with-loco](../authentication-service-with-loco)

## Quick start

```toml
[dependencies]
authentication-verifier = "0.8"
# or, to fetch the key set over HTTPS at boot:
# authentication-verifier = { version = "0.8", features = ["fetch"] }
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
| `Verifier::from_paseto_keys_value(&keys, issuer, audience)` | Build from an in-memory `paseto-keys` document. Every entry with a `kid` is loaded; an entry naming an algorithm this build doesn't implement is *kept* (diagnosable, not silently skipped — see "Algorithm agility" below) but only Ed25519 entries are usable; a duplicate `kid` is an error, not last-wins; an empty key set is permitted and rejects every token with `UnknownKid`. |
| `Verifier::from_paseto_keys_url(url, issuer, audience).await` | Fetch the key set, then build. **Requires `features = ["fetch"]`** (pulls in `reqwest` with rustls). Requires `https://`, except `http://` to a loopback host (dev/CI); 10 s timeout; no redirects; 64 KiB response cap. |
| `Verifier::verify(token) -> Result<Claims, VerifyError>` | Select the key by the token **footer `kid`**, dispatch on its algorithm, check the PASETO v4.public (Ed25519) signature, then enforce `iss`, `aud`, `exp`, and `nbf`. |
| `Verifier::key_count() -> usize` | Number of **usable** (Ed25519) keys loaded. |
| `Verifier::unsupported_key_count() -> usize` / `Verifier::algorithms() -> Vec<String>` | How many loaded keys this build cannot use, and the algorithm labels seen — for logging/metrics during an algorithm rollout. |
| `ReloadableVerifier` | A hot-swappable verifier holder (v0.8): `new(verifier)`, `current() -> Arc<Verifier>` (per-request snapshot), `store(verifier)` (runtime key-set swap for rotation, with no restart). |
| `Claims` | Verified claims: `sub` (user pid, UUID string), `iss`, `aud`, `iat`, `nbf`, `exp`, `sid` (originating auth-service session), `attrs` (ABAC subject attributes; empty on pre-0.3 tokens), plus `scope`/`roles` (deprecated for authorization). |
| `Decision` | `{ allowed, reason, obligations }` — the outcome; `obligations` (v0.6) are the deciding allow rule's advisory instructions (`"mask"`, `"audit"`) for the enforcement point. `Decision::requires("mask")` checks one. |
| `Policy` / `Rule` / `Action` / `Decision` (the `abac` module, re-exported at the root) | The shared ABAC engine: `Policy::from_json` loads a configured policy, `Policy::default_policy()` is the built-in coarse tier, `Policy::evaluate(&claims, action, entity)` decides first-match-wins with default allow-read / deny-mutation. Pure — no I/O, no clock, no panics. |
| `Policy::evaluate_with_resource(&claims, action, entity, &resource)` | As `evaluate`, plus a `BTreeMap<String, Vec<String>>` of record-level **resource attributes** matched by `resource.*` `when` keys (v0.4). A service passes attributes of the record it just loaded so policies can gate on e.g. record sensitivity. |
| `Policy::evaluate_with_context(&claims, action, entity, &resource, &env)` | As above, plus **environment attributes** matched by `env.*` `when` keys (v0.5) — request-time / network context the service supplies (e.g. `env.after_hours`), keeping the engine deterministic. A `when` value `$sub`/`$email` resolves to the caller's identity for ownership rules. |
| `ReloadablePolicy` | A hot-swappable policy holder (v0.7): `new(policy)`, `current() -> Arc<Policy>` (per-request snapshot), `store(policy)` (runtime swap). Lets a service reload the policy without a restart; the reload trigger is the service's concern. |

### `VerifyError` variants

| Variant | Meaning |
|---|---|
| `Keys(String)` | The key-set document was missing or structurally invalid (no key array; Ed25519 entry missing `kid` or public-key material; bad key bytes; a duplicate `kid`). |
| `Malformed(String)` | The token isn't a structurally valid `v4.public` token, or its footer isn't `{"kid": ...}` — distinct from a signature failure. |
| `MissingKid` | The token footer carries no `kid`, so no key can be selected. |
| `UnknownKid(String)` | No loaded key matches the token's `kid` — stale key cache, wrong issuer, or forgery. Refetch the key set to pick up a key rotation. |
| `Paseto(String)` | The Ed25519 signature check failed. |
| `Claim(String)` | The signature was valid but `iss` / `aud` / `exp` / `nbf` did not satisfy the policy. |
| `UnsupportedAlgorithm { kid, algorithm }` | The `kid` selected a key whose algorithm this build doesn't implement. Distinct from `UnknownKid` on purpose: refetching the key set will not fix it, upgrading the binary will (algorithm agility, 2026-07-27). |
| `Fetch(String)` | Fetching the key set over HTTP failed (only with the `fetch` feature). |

### Algorithm agility

Since 2026-07-27, `Verifier` dispatches on each key's *declared*
algorithm rather than assuming Ed25519, so a future non-Ed25519 key in
the published set can't silently verify as if it were one. Keys for an
algorithm this build doesn't implement are **kept, not dropped** — a
token naming one fails as `UnsupportedAlgorithm` (upgrade the binary),
not `UnknownKid` (refetch the key set), which is the correct diagnosis
for a partial algorithm rollout. See
[authentication-sessions.md](../../agents/share/authentication-sessions.md)
§5.1 for why this exists ahead of any actual algorithm switch.

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
cargo test --features fetch # also compile and run the HTTP loader + its SEC-V1 tests
```

The repo's own CI (`scripts/ci-check.sh test`) runs this crate with
`--features fetch` (AV-1) — without it, `from_paseto_keys_url` and its
SEC-V1 HTTPS-only / timeout / no-redirect / body-cap tests would never
be compiled, let alone run, by CI.

## License

MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
