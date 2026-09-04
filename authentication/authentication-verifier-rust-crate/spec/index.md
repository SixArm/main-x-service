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
feature), **algorithm agility** for the key set itself (a key set may
name algorithms this build does not implement — such keys are kept and
diagnosed, not silently dropped; see §5), and the **ABAC policy engine**
(the `abac` module: policy parsing, the built-in default policy, pure
first-match-wins evaluation over the `attrs` claim + derived action +
entity).

Out of scope: token issuance, sessions/revocation (auth-service only),
PASETO `local` (symmetric) tokens, **verifying** non-Ed25519 algorithms
(a key set may name one — §5 `VerifyError::UnsupportedAlgorithm` — but
this build signs/checks Ed25519 only), key-set refresh scheduling
(callers refetch on `UnknownKid`, or hold a `ReloadableVerifier`, §5,
and refresh on a timer), framework-specific extractors, **action derivation**
(each service's guard maps HTTP method + its destructive named POSTs to
an `Action`), policy configuration loading from env/file (callers read
`<ENTITY>_ABAC_POLICY` / `_FILE` and hand the JSON to
`Policy::from_json`), and attribute **assignment/sourcing**
(auth-service only).

## 3. Stakeholders and users

Every peer service crate in the family that accepts the federation's
bearer tokens: the entity registries (see the "Service crates" table in
the root [AGENTS.md](../../../AGENTS.md)), the consumer apps, and the
`link-graph-service-with-loco` read-model aggregator — each uses this
crate instead of re-implementing verification (and, since v0.3, ABAC)
per service. This list has grown well past the original six-then-eight
loco-conversion crates named in earlier drafts of this section; see §5
"Algorithm agility" for how to check the current count rather than
trusting a number pinned here.

## 4. Glossary

See the entity glossary
([../../spec/04-glossary.md](../../spec/04-glossary.md)): PASETO, `kid`,
`sid`, `pid`. **PASETO v4.public** — a versioned, Ed25519-signed,
asymmetric token format (a trusted alternative to JWT). **Footer** — the
PASETO trailer (here carrying `kid`), authenticated but not encrypted.

## 5. Domain model — the public API contract

- **`Verifier`** — verification keys indexed by `kid` + pinned issuer
  and audience. Constructed once, shared behind an `Arc`; `verify` is
  read-only.
  - `from_paseto_keys_value(&serde_json::Value, issuer, audience)` —
    loads every published key entry that carries a `kid` (requires
    public-key material for the ones it can use); **permits an empty
    key set** (boots before the key source is reachable; rejects
    everything with `UnknownKid`); rejects a **duplicate `kid`** (§ below)
    rather than resolving it last-wins.
  - `from_paseto_keys_url(url, issuer, audience)` *(feature `fetch`)* —
    GET the `/.well-known/paseto-keys` document, then delegate to
    `from_paseto_keys_value`. Hardened (SEC-V1, 2026-07-13): requires
    `https://`, except `http://` to a **loopback** host
    (`127.0.0.1`/`::1`/`localhost`, for dev/CI key servers); a 10 s
    timeout; redirects forbidden (so an `https` URL can't be bounced to
    plaintext); the response body capped at 64 KiB.
  - `verify(token) -> Result<Claims, VerifyError>` — parse the PASETO
    footer → require `kid` → look up key → dispatch on the key's
    declared algorithm → verify the v4.public signature +
    `iss`/`aud`/`exp`/`nbf`.
  - `key_count() -> usize` — number of **usable** (Ed25519) keys; a key
    for an algorithm this build does not implement is not counted here.
  - `unsupported_key_count() -> usize` *(algorithm agility, 2026-07-27)*
    — number of loaded keys whose algorithm this build does not
    implement. Non-zero mid-rollout is normal (the issuer publishes a
    new algorithm before every verifier understands it); worth exporting
    as a metric.
  - `algorithms() -> Vec<String>` *(algorithm agility)* — sorted,
    deduplicated algorithm labels this verifier holds keys for, usable
    or not, for logging what a key set actually advertises.
- **`ReloadableVerifier`** *(v0.8.0)* — `RwLock<Arc<Verifier>>`:
  `new(verifier)`, `current() -> Arc<Verifier>` (a per-request snapshot
  under a brief read-lock), `store(verifier)` (a brief write-lock
  swapping the key set at runtime, e.g. after a periodic re-fetch of
  `/.well-known/paseto-keys`, so a **key rotation** needs no restart). No
  `Debug` (key material never lands in a log). Poison-safe. A refresh
  should keep the current verifier on a fetch failure — never swap to an
  empty key set — so a transient auth-service outage cannot lock callers
  out. Mirrors `ReloadablePolicy`'s shape.
- **`Claims`** — `sub` (user pid, UUID string), `email`, `name`, `iss`,
  `aud`, `iat` (unix s), `nbf` (unix s, optional), `exp` (unix s), `sid`
  (originating auth-service session, for revocation correlation),
  `scope`/`roles` (**deprecated for authorization** — kept on the wire;
  the ABAC guard ignores them), and `attrs` (`BTreeMap<String, Vec<String>>`,
  `#[serde(default)]`, omitted when empty — the ABAC subject
  attributes, e.g. `access: ["write"]`, `svc: ["true"]`).
  **Byte-identical** to the service's `auth::Claims`; pinned by the
  service's cross-crate contract test.
- **`VerifyError`** — `Keys(String)` (malformed key-set document, or a
  duplicate `kid`), `Malformed(String)` (the token is not a structurally
  valid `v4.public` token, or its footer isn't `{"kid": ...}` — distinct
  from a signature failure), `MissingKid`, `UnknownKid(String)`,
  `Paseto(String)` (Ed25519 signature check failed), `Claim(String)`
  (signature valid but `iss`/`aud`/`exp`/`nbf` rejected it),
  `UnsupportedAlgorithm { kid, algorithm }` *(algorithm agility,
  2026-07-27)* — the `kid` selected a key whose algorithm this build
  does not implement; deliberately distinct from `UnknownKid` (that
  means "I hold no key for this signer, refetch might help"; this means
  "I hold the key and cannot use it, refetching will not help — upgrade
  this binary") — and `Fetch(String)` (feature `fetch`).

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

### Algorithm agility (be-ready posture, 2026-07-27)

`Verifier` dispatches on each loaded key's *declared* algorithm rather
than assuming Ed25519 (internally, an enum `VerificationKey::{Ed25519,
Unsupported}` — an unrecognised algorithm can only ever land in the
`Unsupported` variant, which carries no key material, so **verification
cannot silently fall through to a default** when a new variant is
added). A key set may name an algorithm this build does not implement;
such keys are **kept, not dropped**, so a token naming one fails as
`UnsupportedAlgorithm` (which says "upgrade this binary") rather than
`UnknownKid` (which invites a key-set refetch that would never help).
This is the family's readiness step for the Ed25519 signature being the
one Shor-vulnerable component in the auth path — see
[authentication-sessions.md](../../../agents/share/authentication-sessions.md)
§5.1 for why this is a be-ready-not-act-now problem and what the
realistic next-algorithm paths are. Nothing in this crate switches
algorithm; it only makes a future switch a key rotation rather than a
coordinated code migration. Source-compatible with every consumer — at
landing time, fifteen path-dependent service crates across the entity
registries, the consumer apps, and the link-graph aggregator all built
unchanged; grep `Cargo.toml` for `authentication-verifier =` across the
monorepo for the current, growing count rather than trusting a number
fixed here.

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
4. A key-set document without a key array, an Ed25519 entry missing
   `kid` / public-key material, or a **duplicate `kid`** across entries,
   yields `Keys(...)` at construction — never a last-wins merge, so the
   verifier's answer cannot depend on JSON array order.
5. A key entry naming an algorithm this build does not implement is
   **kept**, not dropped, and reported via `unsupported_key_count()` /
   `algorithms()`; a token whose `kid` selects one is rejected with
   `UnsupportedAlgorithm { kid, algorithm }`, distinct from `UnknownKid`
   (algorithm agility, §5). An empty key set constructs successfully.
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
8. **No vacuous match on an absent namespace (SEC-V2).** A `!`-negated
   `resource.`/`env.` condition must not match merely because no
   resource/environment map was supplied (e.g. the coarse blanket-guard
   path, which calls plain `evaluate`) — that would let a rule like
   `{"when":{"env.network":["!untrusted"]}}` silently grant every
   authenticated caller. An absent namespaced attribute biases to the
   **safe** outcome by the rule's effect: an `allow` rule does not match
   (no silent grant); a `deny` rule still matches (fail-closed). Subject-
   attribute (non-namespaced) negation is unaffected.
9. `from_paseto_keys_url` (feature `fetch`, SEC-V1) refuses any URL that
   is not `https://`, except `http://` to a loopback host; follows no
   redirects; times out at 10 s; and caps the response body at 64 KiB —
   each a `VerifyError::Fetch`, never a hang or an unbounded allocation.

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

**Fuzzing (SEC-I2).** `fuzz/` is a standalone `cargo-fuzz` crate (not a
workspace member; default offline features only — never affects the
stable `cargo build`/`test`/`clippy` path) with two coverage-guided
libFuzzer targets: `verify` (`Verifier::verify` over an arbitrary token,
exercising the full structural parse — header, authenticated-footer
base64url/JSON `kid` decode, key selection, Ed25519 check, with a real
key loaded so the `kid`-found branch is reachable) and `policy`
(`Policy::from_json` over arbitrary UTF-8, then `evaluate_with_context`
for every action against a fixed subject/resource/environment — parser
plus rule matching, negation, `$sub`/`$email` templates, and the
`resource.`/`env.` namespaces). Both assert the crate's golden rule #5
(no panics — every failure is a handled `Err`). Run with a nightly
toolchain: `cargo +nightly fuzz run <target>` (see `fuzz/README.md`).

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
- [x] **Algorithm agility for the verifier (v0.9.0).**
      *(2026-07-27; released 2026-08-05)* `Verifier` now dispatches on
      each key's *declared* algorithm (internal
      `VerificationKey::{Ed25519, Unsupported}` enum) instead of
      assuming Ed25519, so an unrecognised algorithm cannot silently
      verify — see §5 "Algorithm agility" and
      [authentication-sessions.md](../../../agents/share/authentication-sessions.md)
      §5.1 for the full rationale. Added `unsupported_key_count()` /
      `algorithms()`; `key_count()` now counts only usable keys; a
      duplicate `kid` is a construction error, not last-wins; a token
      selecting an unsupported key fails `UnsupportedAlgorithm`, not
      `UnknownKid`. Source-compatible — no consuming crate changed.
      Landed in `src/lib.rs`; released as `[0.9.0] - 2026-08-05` in
      `CHANGELOG.md`, alongside the two tasks below (Cargo.toml bumped
      `0.8.0` → `0.9.0`).
- [x] **cargo-fuzz harness (SEC-I2, v0.9.0).** *(2026-07-14; released
      2026-08-05)* Added `fuzz/` (two libFuzzer targets, `verify` and
      `policy`) per §11.
- [x] **SEC-V1/V2/V4 hardening (v0.9.0).** *(2026-07-13; released
      2026-08-05)* HTTPS (or loopback-only HTTP) + timeout + no-redirect
      + 64 KiB body cap on `from_paseto_keys_url` (SEC-V1, §6 FR9); no
      vacuous match for a negated `resource.`/`env.` condition on an
      absent namespace (SEC-V2, §6 FR8); cross-key-forgery, missing-`exp`,
      and never-panic-on-malformed-input tests (SEC-V4). See
      [security.md](../../../agents/share/security.md) §2 for the
      family-wide audit these came out of.
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
- [ ] **AV-1 (S) Exercise the `fetch` feature in CI.** *(verified:
      `default = []` and `fetch = ["dep:reqwest"]` in `Cargo.toml`;
      `scripts/ci-check.sh`'s `test` stage runs plain `cargo test` with
      no `--features` flag for every crate, confirmed by grepping it
      for `cargo test`/`features`)* — every `#[cfg(feature = "fetch")]`
      item in `src/lib.rs` (`from_paseto_keys_url` itself, and the
      SEC-V1 HTTPS-only / timeout / no-redirect / 64 KiB body-cap tests
      at lines ~1016/1151/1173) is **never compiled, let alone run, by
      this repo's own CI** — the same class of gap
      [`agents/share/bulk-import-export.md`](../../../agents/share/bulk-import-export.md)
      §11 step 4 documents for person's `parquet` feature ("does this
      crate's CI even build its optional features"). Add a
      `test-fetch` (or similar) stage/step that runs
      `cargo test --features fetch` for this crate specifically (either
      in `scripts/ci-check.sh` as a crate-specific override, or as a
      dedicated CI job), and note the gap's closure in `CHANGELOG.md`.
      **Acceptance:** a CI run (or a documented local
      `cargo test --features fetch`) actually compiles and passes the
      `fetch`-gated tests; `AGENTS.md`/`README.md` note how to run it.

- [ ] **AV-2 (S) Cut a dated release for the accumulated `[Unreleased]`
      changes.** *(verified: `CHANGELOG.md`'s `[Unreleased]` section
      lists the MSRV 1.96 bump and the Criterion `benches/
      verify_and_authorize.rs` addition, both undated; `Cargo.toml`
      still reads `version = "0.9.0"`, the same version §14 says
      shipped 2026-08-05 — i.e. two real changes have landed since the
      last release with no version bump)*. Per the family's "cargo
      publish authorized" convention for already-published crates once
      verified green, bump `Cargo.toml` to the next `0.10.0` (the
      bench addition is new public-facing capability, not just a patch)
      or `0.9.1`, move the `[Unreleased]` entries under a dated
      heading, and publish. Update spec §13/§14 to record the release.
      **Acceptance:** `cargo test`, `cargo clippy --all-targets -- -D
      warnings`, `cargo fmt --check` all green; `CHANGELOG.md` has no
      stale `[Unreleased]` content; `cargo publish --dry-run` succeeds.

- [ ] **AV-3 (S) Reconcile §16's stale version-bump open question.**
      *(verified: §16 currently reads "Version-bump for the unreleased
      hardening — the next release needs a number that doesn't
      collide with … `[0.8.0]` … (Lean: `0.9.0`, decided alongside the
      crates.io publish call, both explicitly deferred by H-5)", but
      §13/§14 of this same document already record that decision as
      made and shipped: "v0.9.0 shipped (2026-08-05)")*. The open
      question and the implementation-status section directly
      contradict each other, which is exactly the drift the SDD
      discipline in
      [`agents/share/index.md`](../../../agents/share/index.md) exists
      to prevent. Move the resolved entry to a `~~struck~~ — RESOLVED`
      form (matching this doc's own convention elsewhere, e.g. the
      "PASETO library choice" entry two lines below it) or delete it,
      whichever the current unreleased-changes state (AV-2) leaves
      accurate.
      **Acceptance:** §16 no longer asserts an unresolved version-bump
      decision that §13/§14 show as already made.

- [ ] **AV-4 (M) Resolve the Axum-extractor open question with real
      duplication data.** §16 asks "Should the crate offer an Axum
      extractor, or stay framework-free and let each service wrap it?
      (Currently framework-free.)" *(verified:
      `grep -rl "struct AuthUser" --include="*.rs" .` from the repo
      root, excluding `target/`, finds the same `FromRequestParts`-based
      `AuthUser` extractor hand-duplicated in 15 sibling crates —
      organization, care-pathway, workforce-planning-management,
      course, project-portfolio-management, person, patient-flow,
      thing, contact-relationship-management, case,
      content-management-system, worker, place, event, and this
      family's own authentication-service)*. Survey 2–3 of those
      implementations for how much is genuinely copy-identical
      boilerplate (parse `Authorization: Bearer …`, call
      `Verifier::verify`, map `VerifyError` → `401`) versus
      crate-specific (loco `AppContext` vs `axum::extract::State`,
      differing error bodies), then either (a) ship an optional `axum`
      Cargo feature on this crate providing a generic
      `BearerClaims<V: AsRef<Verifier>>` extractor peers can adopt on
      next touch, or (b) close the open question explicitly as "stay
      framework-free" with the survey's reasoning recorded in §16.
      Either outcome is a spec (§5/§16) + code (if (a)) + test change.
      **Acceptance:** §16's open question is replaced by a recorded
      decision; if (a), the new extractor has offline unit tests and a
      documented adoption note for peers.

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

**ABAC through v0.8.0 shipped (2026-07-05).** `Claims.attrs` + the
`abac` policy engine (§5/§6 FR6–FR7) plus its four additive follow-ons —
record-level resource attributes (v0.4.0), ownership templates +
environment attributes (v0.5.0), obligations (v0.6.0), and hot-reload
for both the policy (`ReloadablePolicy`, v0.7.0) and the verifier
(`ReloadableVerifier`, v0.8.0) — are all shipped and released.

**v0.9.0 shipped (2026-08-05).** Three further changes released
together in a dated `CHANGELOG.md` heading with a bumped `Cargo.toml`
version (see §13): SEC-V1/V2/V4 hardening (landed 2026-07-13), the
`fuzz/` cargo-fuzz harness (SEC-I2, landed 2026-07-14), and verifier
algorithm agility (landed 2026-07-27, §5). `cargo test` is green;
`cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are
both clean. Cargo.toml's `version` field reads `0.9.0`, confirming the
bump described above.

## 15. Roadmap

v0.1 (RS256-JWT, superseded): core JWKS verification + `fetch`. v0.2.0:
**PASETO v4.public pivot** — `from_paseto_keys_*`, footer-`kid`
selection, Ed25519 verification; same `Claims` role. A **BREAKING**
change (see [CHANGELOG.md](../CHANGELOG.md)). v0.3.0: **ABAC** — the
`attrs` claim + the shared `abac` policy engine; additive. v0.4.0–v0.8.0:
record-level resource attributes, ownership/environment attributes,
obligations, and hot-reload for both the policy and the verifier — all
additive (§13, §14). **Unreleased (here): SEC-V1/V2/V4 hardening,
cargo-fuzz (SEC-I2), and verifier algorithm agility** — landed in code,
awaiting a version-bump/release decision (§14). Later: refetch-on-
`UnknownKid` convenience helper, a property test over the PASETO-keys
parser itself (distinct from the `verify` fuzz target, which fuzzes
tokens, not key-set documents), removal of `scope`/`roles` in a future
major.

## 16. Open questions

- Should the crate offer an Axum extractor, or stay framework-free and
  let each service wrap it? (Currently framework-free.)
- Multiple audiences per verifier, if peers ever get distinct `aud`s.
- **Version-bump for the unreleased hardening** — the next release
  needs a number that doesn't collide with the existing `[0.8.0] -
  2026-07-05` heading; SEC-V1/V2/V4 are behavioural hardening
  (arguably a minor, not a patch, since e.g. SEC-V2 changes an ABAC
  decision in a real if narrow case) and algorithm agility is
  additive-but-security-relevant. (Lean: `0.9.0`, decided alongside the
  crates.io publish call, both explicitly deferred by H-5.)
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
- [../../agents/verification.md](../../agents/verification.md) — peer
  integration guide.
- [PASETO](https://paseto.io/) — Platform-Agnostic Security Tokens;
  v4.public = Ed25519 (RFC 8032). `rusty_paseto` crate.

## 18. Change control

Update this spec in the same PR as any behavioural change. Bump
[CHANGELOG.md](../CHANGELOG.md) under `[Unreleased]`.
