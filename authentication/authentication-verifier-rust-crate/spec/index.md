# Authentication Verifier — Specification

> **Single source of truth.** Code conforms to this spec, not the other
> way around. A behavioural change is a three-part PR: spec edit + code
> edit + test edit. Live work queue is §13; open questions are §16.
>
> Issuing service:
> [authentication-service-with-loco](../../authentication-service-with-loco/spec/index.md).
> Entity-level contract:
> [../../spec/index.md](../../spec/index.md).
> Canonical design (single source of truth for the auth model):
> [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
> §5 (PASETO v4 public + offline verification + this crate's shape).

## 1. Purpose and vision

A reusable, dependency-light Rust library that lets any Main X Index
peer service verify the authentication-service's short-lived
cross-service tokens **offline**. The token format is **PASETO
v4.public** (Ed25519-signed): fetch the published Ed25519 public
key(s) once at boot, then verify every bearer token locally. No shared
secret, no per-request introspection hop.

> **Pivot (v0.2.0).** This crate previously verified **RS256 JWTs**
> against a JWKS. Per
> [authentication-sessions.md](../../../agents/share/authentication-sessions.md),
> JWT is removed from the auth path: the human session is a Postgres-backed
> cookie session, and the only cross-service token is a short-lived PASETO
> v4.public. The crate keeps its **role** (peer-side, offline,
> dependency-light verification) but changes its **implementation** from
> RS256-JWT/JWKS to PASETO v4.public.

> **ABAC (v0.3.0, additive).** Per
> [authorization-attributes.md](../../../agents/share/authorization-attributes.md),
> the crate is also the family's shared **authorization** foundation:
> `Claims` gains the `attrs` subject-attribute map (absent on pre-0.3
> tokens ⇒ empty; no re-issue needed), and the `abac` module ships the
> one pure policy engine the nine entity services call from their
> blanket `/api/*` guards — instead of nine per-crate role/RBAC
> implementations. `scope`/`roles` are deprecated for authorization.

## 2. Scope

In scope: PASETO-keys parsing (Ed25519 public keys), footer-`kid`-based
key selection, PASETO v4.public signature verification, `iss` / `aud` /
`exp` / `nbf` enforcement, optional HTTPS key-set fetching (`fetch`
feature), and the **ABAC policy engine** (the `abac` module: policy
parsing, the built-in default policy, pure first-match-wins evaluation
over the `attrs` claim + derived action + entity).

Out of scope: token issuance, sessions/revocation (auth-service only),
PASETO `local` (symmetric) tokens, non-Ed25519 algorithms, key-set
refresh scheduling (callers refetch on `UnknownKid`), framework-specific
extractors, **action derivation** (each service's guard maps HTTP
method + its destructive named POSTs to an `Action`), policy
configuration loading from env/file (callers read
`<ENTITY>_ABAC_POLICY` / `_FILE` and hand the JSON to
`Policy::from_json`), and attribute **assignment/sourcing**
(auth-service only).

## 3. Stakeholders and users

Peer service crates (person / worker / place / thing / event / course /
organization / care-pathway) that accept the federation's bearer
tokens; the loco conversion uses this crate instead of re-implementing
verification per service.

## 4. Glossary

See the entity glossary
([../../spec/04-glossary.md](../../spec/04-glossary.md)): PASETO, `kid`,
`sid`, `pid`. **PASETO v4.public** — a versioned, Ed25519-signed,
asymmetric token format (a trusted alternative to JWT). **Footer** — the
PASETO trailer (here carrying `kid`), authenticated but not encrypted.

## 5. Domain model — the public API contract

- **`Verifier`** — Ed25519 public keys indexed by `kid` + pinned issuer
  and audience. Constructed once, shared behind an `Arc`; `verify` is
  read-only.
  - `from_paseto_keys_value(&serde_json::Value, issuer, audience)` —
    loads every published Ed25519 public-key entry (requires `kid` and
    the public-key material); skips non-Ed25519 entries; **permits an
    empty key set** (boots before the key source is reachable; rejects
    everything with `UnknownKid`).
  - `from_paseto_keys_url(url, issuer, audience)` *(feature `fetch`)* —
    GET the `/.well-known/paseto-keys` document, then delegate to
    `from_paseto_keys_value`.
  - `verify(token) -> Result<Claims, VerifyError>` — parse the PASETO
    footer → require `kid` → look up key → verify the v4.public
    signature + `iss`/`aud`/`exp`/`nbf`.
  - `key_count() -> usize`.
- **`Claims`** — `sub` (user pid, UUID string), `iss`, `aud`, `iat`
  (unix s), `nbf` (unix s), `exp` (unix s), `sid` (originating
  auth-service session, for revocation correlation), `scope`/`roles`
  (**deprecated for authorization** — kept on the wire; the ABAC guard
  ignores them), and `attrs` (`BTreeMap<String, Vec<String>>`,
  `#[serde(default)]`, omitted when empty — the ABAC subject
  attributes, e.g. `access: ["write"]`, `svc: ["true"]`).
  **Byte-identical** to the service's `auth::Claims`; pinned by the
  service's cross-crate contract test.
- **`VerifyError`** — `Keys(String)`, `MissingKid`,
  `UnknownKid(String)`, `Paseto(String)` (signature / claim / parse
  failure), and `Fetch(String)` (feature `fetch`).
- **`abac` module** (re-exported at the crate root) — the shared
  authorization engine per
  [authorization-attributes.md](../../../agents/share/authorization-attributes.md)
  §2–§5:
  - `Action` — the derived request action (`Read` / `Write` / `Delete`
    / `Destructive`); each service's guard derives it from the HTTP
    method plus the crate's documented destructive named POSTs.
  - `Policy` / `Rule` / `ActionPattern` / `Effect` — the JSON policy
    document (`Policy::from_json`; unknown rule fields ignored,
    forward-compatible) and `Policy::default_policy()` (the built-in
    §5 coarse tier: `svc=true` ⇒ everything, `access=admin` ⇒
    destructive+write, `access=write` ⇒ write).
  - `Policy::evaluate(&claims, action, entity) -> Decision` — ordered
    first-match-wins allow/deny; no match ⇒ default allow-read /
    deny-mutation. `when` is a conjunction; a value list is an OR;
    `!`-prefixed values negate; `delete` implies `destructive` for
    rule matching; pseudo-attributes `sub` / `email` / `entity`
    resolve from the verified claims/resource and cannot be shadowed
    by `attrs`. Pure and total: no I/O, no clock, no panics.
  - `Policy::evaluate_with_resource(&claims, action, entity, &resource)`
    (v0.4) — as `evaluate`, plus a `BTreeMap<String, Vec<String>>` of
    **record-level resource attributes** matched by `resource.<name>`
    `when` keys (e.g. `resource.sensitivity`). The `resource.`
    namespace is disjoint from subject attributes (no spoofing via the
    token); under plain `evaluate` every `resource.*` key resolves
    empty. `evaluate` delegates here with an empty map.
  - `Policy::evaluate_with_context(&claims, action, entity, &resource,
    &env)` (v0.5) — as above, plus **environment attributes** matched
    by `env.<name>` keys (request-time / network context, e.g.
    `env.after_hours`; the caller supplies the clock so the engine stays
    deterministic). A `when` **value** `$sub` / `$email` is a template
    resolving to the caller's identity, so a rule expresses ownership
    (`resource.owner: ["$sub"]`). `evaluate_with_resource` delegates
    here with an empty env.
  - `Decision` — `allowed` + `reason` (deciding rule index or the
    default decision) + `obligations` (v0.6) — the deciding allow
    rule's advisory instructions (`"mask"` / `"audit"`) the enforcement
    point must honour; empty on a deny/default. `Decision::requires`
    checks one. So a 403 body and the audit trail can state exactly why,
    and a conditional allow can carry a mask/audit obligation.

### The PASETO-keys / `kid` contract

- The service publishes its Ed25519 public key(s) at
  `/.well-known/paseto-keys` (the JWKS analog), each entry carrying a
  `kid` and the base64url (no padding) Ed25519 public-key bytes.
- Tokens are **PASETO v4.public**; the **footer** carries the `kid` that
  selects the verifier key, so rotation never needs a shared secret.
- Defaults at the service: issuer `authentication-service`, audience
  `main-x-service`, token TTL ~300 s (5 min; derived from the session).

## 6. Functional requirements

1. A v4.public token signed by the auth-service verifies and
   round-trips all claims (`sub`, `iss`, `aud`, `iat`, `nbf`, `exp`,
   `sid`, `scope`/`roles`).
2. Expired (`exp`) tokens, not-yet-valid (`nbf`) tokens, wrong-audience
   tokens, wrong-issuer tokens, tampered tokens, and garbage strings are
   rejected via `VerifyError::Paseto`.
3. A missing footer `kid` yields `MissingKid`; an unmatched `kid`
   yields `UnknownKid(kid)`.
4. A key-set document without a key array, or an Ed25519 entry missing
   `kid` / public-key material, yields `Keys(...)` at construction.
5. Non-Ed25519 key entries are skipped silently; an empty key set
   constructs successfully.
6. The `attrs` claim round-trips mint→verify; a token without an
   `attrs` member (pre-0.3) verifies with an empty map.
7. The ABAC engine evaluates per
   [authorization-attributes.md](../../../agents/share/authorization-attributes.md)
   §4–§5: first match wins; deny-before-allow pins the deny; `!`
   negation matches absence; `*` covers every action; `delete` implies
   `destructive` (but not vice versa); empty `when` matches everyone;
   an empty value list matches no one; no rule matched ⇒ read allowed,
   everything else denied. Malformed policy JSON is an `Err` from
   `Policy::from_json` (never a panic); callers fall back to
   `Policy::default_policy()`.

## 7. Non-functional requirements

- `#![forbid(unsafe_code)]`.
- Dependency-light core: a PASETO v4 library (e.g. `rusty_paseto`),
  `serde`, `serde_json`, `thiserror`; `reqwest` only behind `fetch`.
- No async in the core path; `from_paseto_keys_url` is the only async fn.

## 8. Architecture

Two modules: `src/lib.rs` (verification — `Verifier`, `Claims`,
`VerifyError`, `fetch` feature) and `src/abac.rs` (authorization — the
policy engine, re-exported at the root). No I/O in the default feature
set; the engine is pure data + pure evaluation. Callers cache the
`Verifier` for the process lifetime and refetch on `UnknownKid` to pick
up key rotation (entity spec §13 T-5), and load their `Policy` once at
boot.

## 9. API surface

See §5. Crate name: `authentication-verifier` (lib
`authentication_verifier`).

## 10. Persistence

None. The crate is stateless; the published key-set document
(`/.well-known/paseto-keys`) is the caller's input.

## 11. Testing strategy

Offline unit tests in `src/lib.rs` using a committed throwaway Ed25519
keypair (never used in production): round-trip, expiry (`exp`),
not-yet-valid (`nbf`), audience, issuer, unknown-`kid`, missing-`kid`,
tampered token, garbage tokens, empty/malformed key set, non-Ed25519
skipping, and the FR4 malformed paths (entry missing `kid` / public-key
material, and unparsable public-key bytes). ABAC pins (FR6/FR7):
`attrs` round-trips mint→verify; a raw pre-0.3 payload without `attrs`
verifies to an empty map; and the engine suite in `src/abac.rs`
(default-policy tiers, first-match deny-before-allow, negation,
wildcard, delete-vs-destructive, empty-`when`/empty-value-list,
pseudo-attribute matching and non-shadowability, malformed-policy
errors, unknown-field tolerance, default-policy JSON round-trip).

The `fetch` feature is tested **offline by design**: a test exercises
the `from_paseto_keys_url` transport-error mapping (an unsupported URL
scheme must surface as `VerifyError::Fetch`, never a panic or `Keys`)
without opening a socket. A real server round-trip is deliberately
**not** exercised here — there is no mock-HTTP/wiremock dependency, to
keep the suite network-free and the crate dependency-light. The full
sign-then-fetch-then-verify round-trip is covered by the service
crate's **cross-crate contract test** (`tests/sign_verify_contract.rs`),
which also pins the `Claims` shape and footer-`kid` selection against the
service's signer.

## 12. Compliance

Claims may carry identity data (`sub`, `attrs`, `scope`/`roles`):
peers must not log them beyond the family's GDPR posture — `attrs`
values (department, clearance, purpose-of-use) are themselves personal
data. Verification is local, so no token ever transits to a third
party. `Decision.reason` deliberately names only the rule index, never
attribute values, so it is safe for 403 bodies and audit trails.

## 13. Tasks (live work queue)

- [x] **PASETO v4.public pivot (code follow-up).** *(2026-06-17 —
      shipped as v0.2.0)* Replaced the
      RS256-JWT/JWKS implementation in `src/lib.rs` with PASETO
      v4.public per §5/§6 and
      [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
      §5: swap the `jsonwebtoken`/RSA stack for a PASETO v4 library
      (e.g. `rusty_paseto`); rename `from_jwks_value`/`from_jwks_url`
      to `from_paseto_keys_value`/`from_paseto_keys_url`; parse the
      footer `kid`; verify the Ed25519 signature + `iss`/`aud`/`exp`/
      `nbf`; rename `VerifyError::Jwks`→`Keys` and
      `Jwt`→`Paseto`; keep the same `Claims` shape (now `sid` +
      `scope`/`roles`, no `jti`/`email`/`name`). Updated the throwaway
      keypair and all unit tests to Ed25519. Published to crates.io as
      `authentication-verifier` 0.2.
- [x] **ABAC (v0.3.0).** *(2026-07-05)* Added the `attrs` claim
      (`#[serde(default)]`, empty-map fallback for pre-0.3 tokens) and
      the `abac` module (`Policy` / `Rule` / `Action` / `ActionPattern`
      / `Effect` / `Decision`, `Policy::from_json`,
      `Policy::default_policy`, first-match-wins `evaluate` with
      default allow-read / deny-mutation) per
      [authorization-attributes.md](../../../agents/share/authorization-attributes.md)
      §3–§4; deprecated `scope`/`roles` for authorization; engine +
      wire-pin tests. Version 0.2.0 → 0.3.0 (additive).
- [x] **Record-level resource attributes (v0.4.0).** *(2026-07-05)*
      Added `Policy::evaluate_with_resource(claims, action, entity,
      resource)` and the `resource.<name>` `when` namespace per
      [authorization-attributes.md](../../../agents/share/authorization-attributes.md)
      §9 (record-level attributes feed the decision; disjoint from
      subject attributes; `evaluate` delegates with an empty map).
      Engine tests: sensitivity-gated deny below an admin allow,
      delegation identity, negation, namespace disjointness. Version
      0.3.0 → 0.4.0 (additive).
- [x] **Ownership templates + environment attributes (v0.5.0).**
      *(2026-07-05)* `$sub`/`$email` `when`-value templates (ownership,
      §4) + `Policy::evaluate_with_context` with the `env.<name>`
      namespace (request-time/network context, §10). Additive:
      `evaluate`/`evaluate_with_resource` unchanged. Engine tests:
      `$sub` ownership, literal-`$`, `env` time-window deny, empty-env
      delegation identity. Version 0.4.0 → 0.5.0.
- [x] **Hot-reloadable verifier for key rotation (v0.8.0).**
      *(2026-07-05)* `ReloadableVerifier` (`RwLock<Arc<Verifier>>`;
      `new`/`current`/`store`; poison-safe; no `Debug`) lets a service
      swap its key set at runtime (periodic re-fetch of
      `/.well-known/paseto-keys`) so key rotation needs no restart; keep
      current keys on a failed fetch. Additive; `Verifier` unchanged.
      Test: `store` swaps the key set while a prior `current()` snapshot
      is preserved. Version 0.7.0 → 0.8.0.
- [x] **Hot-reloadable policy (v0.7.0).** *(2026-07-05)*
      `ReloadablePolicy` (`RwLock<Arc<Policy>>`; `new`/`current`/`store`;
      poison-safe) lets a service swap the active policy at runtime with
      a lock-light read path; the trigger is the service's concern.
      Additive; engine unchanged. Test: `store` swaps for new readers
      while a prior `current()` snapshot is preserved. Version
      0.6.0 → 0.7.0.
- [x] **Obligations (v0.6.0).** *(2026-07-05)* `Rule.obligations` +
      `Decision.obligations` (both `#[serde(default)]`) +
      `Decision::requires`, per §11: an allow rule attaches advisory
      instructions (`"mask"`/`"audit"`) the enforcement point honours;
      the engine carries but does not interpret them; deny/default carry
      none. Additive. Engine tests: allow surfaces obligations,
      deny/default carry none, first-match precedence, default-policy
      allows carry none. Version 0.5.0 → 0.6.0.
- [ ] Refetch-on-`UnknownKid` helper (or document the pattern per
      entity spec §13 T-5 key rotation).
- [ ] Property-test the PASETO-keys parser against fuzzed documents.

### Done (RS256-JWT era, superseded by the PASETO pivot)

- [x] Pin every validated claim rule with an offline unit test.
      *(2026-06-13)*
- [x] Crate-level lints: `#![forbid(unsafe_code)]`,
      `#![warn(clippy::pedantic)]`, `#![deny(missing_docs)]` all land
      green. *(2026-06-13)*
- [x] Pin the FR4 malformed-JWKS paths with targeted unit tests.
      *(2026-06-15)*
- [x] Pin the `fetch`-feature transport-error mapping offline.
      *(2026-06-15)*

## 14. Implementation status

**PASETO v4.public shipped (v0.2.0, 2026-06-17).** The shipped
`src/lib.rs` implements the PASETO v4.public surface of §5/§6
(`from_paseto_keys_*`, footer-`kid` selection, Ed25519 verification via
`rusty_paseto`); the RS256-JWT/JWKS implementation (v0.1.x) is removed.
The doc set, `fetch` feature, offline-test discipline, and
packageability (`cargo package --list`) carried over unchanged in shape.

**ABAC shipped (v0.3.0, 2026-07-05).** `Claims.attrs` + the `abac`
policy engine per §5/§6 FR6–FR7; additive (pre-0.3 tokens verify
unchanged). 31 unit + 4 doc tests green, clippy clean.

## 15. Roadmap

v0.1 (RS256-JWT, superseded): core JWKS verification + `fetch`. v0.2.0:
**PASETO v4.public pivot** — `from_paseto_keys_*`, footer-`kid`
selection, Ed25519 verification; same `Claims` role. A **BREAKING**
change (see [CHANGELOG.md](../CHANGELOG.md)). **v0.3.0 (here): ABAC** —
the `attrs` claim + the shared `abac` policy engine; additive. Later:
rotation ergonomics (refetch-on-`UnknownKid`), record-level resource
attributes and environment attributes if the shared design adopts them
(authorization-attributes.md §9), removal of `scope`/`roles` in a
future major.

## 16. Open questions

- Should the crate offer an Axum extractor, or stay framework-free and
  let each service wrap it? (Currently framework-free.)
- Multiple audiences per verifier, if peers ever get distinct `aud`s.
- ~~PASETO library choice~~ — resolved: `rusty_paseto` (v4 public,
  `default-features = false`) ships in v0.2.0 and builds under
  `#![forbid(unsafe_code)]` (shared-doc §10).

## 17. References

- [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
  — canonical auth/session design; §5 is this crate's contract.
- [authorization-attributes.md](../../../agents/share/authorization-attributes.md)
  — canonical ABAC design; §2–§5 are the `attrs` claim + engine
  contract.
- [src/lib.rs](../src/lib.rs) — implementation + rustdoc.
- [../../spec/index.md](../../spec/index.md) — entity-level contract.
- [../../AGENTS/verification.md](../../AGENTS/verification.md) — peer
  integration guide.
- [PASETO](https://paseto.io/) — Platform-Agnostic Security Tokens;
  v4.public = Ed25519 (RFC 8032). `rusty_paseto` crate.

## 18. Change control

Update this spec in the same PR as any behavioural change. Bump
[CHANGELOG.md](../CHANGELOG.md) under `[Unreleased]`.
