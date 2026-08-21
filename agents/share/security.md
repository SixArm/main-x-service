# Security — audit summary, invariants, and the activation gate

The Main X Index family's security posture in one place: what the
**2026-07-12 repo-wide audit** found, the **cross-cutting invariants** every
crate must uphold, the **`<ENTITY>_REQUIRE_AUTH` activation gate** that
governs whether any of the authz machinery is live, the **secret-handling**
rules, and the **threat model**. This is a design/reference document; the
per-finding work items live in [tasks.md](../../tasks.md) **Phase 5
(SEC-\*)** and the fixes land as three-part changes (crate `spec §13` + code
+ test) recorded in each crate's `CHANGELOG.md`.

It builds on and cross-references the auth stack
([authentication-sessions.md](authentication-sessions.md),
[jwt-enforcement.md](jwt-enforcement.md),
[authorization-attributes.md](authorization-attributes.md)), the privacy
rules ([privacy.md](privacy.md)), the audit trail
([auditability.md](auditability.md)), the bus
([event-bus.md](event-bus.md)), cross-service linking
([cross-service-linking.md](cross-service-linking.md)), bulk
([bulk-import-export.md](bulk-import-export.md)), and compliance
([compliance-for-healthcare.md](compliance-for-healthcare.md),
[compliance-for-technology.md](compliance-for-technology.md)).

## 1. Provenance

A repo-wide security audit (2026-07-12) read the authentication service, the
offline PASETO verifier + ABAC engine, every entity service's guard /
masking / query layer, the matcher libraries + validators, and the bulk /
cross-service-linking / concurrency / secrets surfaces. Its findings are
enumerated as Phase 5 in [tasks.md](../../tasks.md); this document is the
narrative summary + the durable invariants those fixes establish.

## 2. Audit summary by theme

- **F-authn — token & session integrity.** The worst finding: a **committed
  dev signing seed** the auth service fell back to with no environment guard
  — a production deploy that forgot `TOKEN_PRIVATE_KEY_SEED` would sign
  PASETOs anyone could forge (`attrs:{access:[admin]}`). Fixed by a
  **fail-closed production guard** (SEC-A1): the dev seed is refused unless
  the environment is explicitly non-production. Also fixed: an
  unauthenticated `/api/auth/audit/recent` email leak (SEC-A2), the
  magic-link token logged unconditionally (SEC-A3), a racy single-use
  magic-link consume (SEC-A4, atomic claim), incomplete GDPR erasure
  (SEC-A7, scrub `auth_events.email` + `sessions.user_agent`), and
  stale-privilege token minting (SEC-A8, revoke sessions on attribute
  change). Remaining: constant-work signup timing (SEC-A5), rate-limit
  email canonicalization (SEC-A6), hash-at-rest of link/session tokens
  (SEC-A9), CSRF origin backstop (SEC-A10).
- **F-authz — verifier & ABAC edges.** These process attacker-controlled
  tokens, so they carry a forgery + fuzz + policy-property suite (SEC-V4,
  done). Fixed: `from_paseto_keys_url` now requires HTTPS (loopback
  excepted), a timeout, and a response-size cap (SEC-V1, MITM key
  injection); the **vacuous-negation** escalation where a `!`-negated
  `resource.`/`env.` condition matched on the coarse no-record guard path
  (SEC-V2). Malformed-key-entry load resilience (SEC-V3) was an open
  design call, not a gap — decided 2026-08-05 to keep the deliberate
  fail-fast (a malformed key document refuses to load rather than
  silently dropping the bad entry).
- **F-guard — read-path masking & guard consistency.** Fixed: the blanket
  guard is now **guard-all / deny-unless-public** rather than prefix-gated
  (SEC-G5); `derive_action` normalises a trailing slash so it can't
  downgrade a destructive POST (SEC-G6); `LIKE`-wildcard injection is
  neutralised in the repo-based searches (SEC-G4, `escape_like`); the
  governed cross-service edges (`subject_of`) are concealed behind
  read-the-case authz (SEC-G1); the person `search_persons` offset is
  bounded (SEC-G7). Remaining: record-level masking on person `list` /
  `search` / `check_duplicates` paths (SEC-G3, partial) and the
  **default-off exposure pin** (SEC-G8).
- **F-data — bulk / linking / concurrency integrity.** Fixed: the
  **critical reconcile scoping bug** (link-graph diffed the *global*
  read-model against *one* entity's edges, so each pass deleted the others'
  — the graph never converged) is now scoped per source entity (SEC-B1);
  merge TOCTOU + person self-merge (SEC-B5, lock participants + reject
  self-merge); the relay double-ship (SEC-B6, `FOR UPDATE SKIP LOCKED`);
  bulk OOM caps (SEC-B2, byte + row caps, **plus end-to-end streaming** as
  of 2026-08-05 — the import path holds neither the uploaded file nor the
  decoded rows, proved by an allocator-instrumented test, so the caps are
  now work/storage ceilings rather than the only thing preventing an
  out-of-memory kill); the SELECT-then-INSERT upsert race (SEC-B3,
  stable-key advisory lock); artifact IDOR + TTL + `file://` confinement
  (SEC-B4, plus the object-store sweep that physically deletes expired
  artifact bytes as of 2026-08-05); reconcile peer-trust (SEC-B7, token for
  remote + edge-type validation); bulk audit gaps (SEC-B8, partial);
  idempotency key wiring (SEC-B9); person merge audit in-tx (SEC-B10);
  probe SSRF-via-redirect + freshness guard (SEC-B11).
- **F-input — unverified input, false matches & fuzzing.** Fixed:
  per-field length + array-cardinality **input-size caps** family-wide
  (SEC-M1) closing the unbounded O(n·m) Jaro-Winkler / Levenshtein /
  Jaccard DoS; the systemic **false-deterministic-match** class where a
  short-circuit keyed on a post-normalisation string with no empty guard let
  two records sharing only blank/punctuation values score a spurious `1.0`
  (SEC-M2 empty guards; SEC-M3); an `i64` overflow **panic** in portfolio
  date math (SEC-M4); organization deterministic-identifier check-digit
  validation (SEC-M5, LEI/GLN/DUNS/VAT). `proptest` property harnesses cover
  every matcher (SEC-M6). Remaining: the coarse `limit_payload` body-cap
  backstop (SEC-M1 residual) and `cargo-fuzz` (SEC-I2).
- **F-assurance — supply-chain & test infrastructure.** Fixed:
  `cargo-deny` + a repo `deny.toml` (SEC-I1); `#![forbid(unsafe_code)]` on
  **every** crate root (SEC-I3). Remaining: `cargo-fuzz` targets (SEC-I2)
  and this document (SEC-I4).

## 3. Cross-cutting invariants (do not erode)

These are the load-bearing rules. A change that violates one is a
regression even if it compiles and the feature "works".

1. **Fail-closed on secrets.** A signing key / seed with a development
   default must **refuse to run** in production rather than silently using
   the default (SEC-A1). The same posture applies to any future secret: no
   usable default outside an explicitly non-production environment.
2. **Never-panic on untrusted input.** Matcher engines, validators, and
   codecs run on attacker-controlled bytes. They must return errors, never
   `panic!` / `unwrap` / overflow (SEC-M4; the `proptest` never-panic
   properties, SEC-M6). Pure functions over external input are fuzz targets.
3. **Bound every input.** Per-field text length, array cardinality, and
   per-entry length are capped **before** persist/match (`MAX_TEXT_LEN` =
   1024, `MAX_ARRAY_LEN` = 256, `MAX_ITEM_LEN` = 512; SEC-M1). Pagination
   offsets are bounded (SEC-G7). Bulk uploads have byte + row caps (SEC-B2).
   Unbounded fan-out into O(n·m) scoring is a DoS.
4. **No spurious identity.** A deterministic short-circuit to `1.0` must
   require **both** sides to carry a non-empty, well-formed value — never
   match on a shared blank / punctuation / sentinel (SEC-M2/M3), and
   validate check-digits where the scheme defines them (SEC-M5).
5. **Masking on every read path.** Record-level masking / authorization is
   not a single-record-GET feature — it must hold on `list` / `search` /
   `check-duplicates` / FHIR / bulk-export / graph paths too. A bulk or
   aggregate read must never reveal more than the equivalent single read
   (privacy.md; SEC-G1/G3; bulk export masking, SEC-B4/B8).
6. **Fail-closed authorization.** Default decision is **read-allow,
   mutation-deny**; an explicit `allow` is required for every non-read
   action; `401` = no/bad credential, `403` = valid credential + policy
   denied (authorization-attributes.md). A `!`-negated `resource.`/`env.`
   condition must **not** match on the coarse no-record path (SEC-V2).
7. **Offline-verify from trusted sources only.** Peer key fetches are
   HTTPS-only (loopback excepted), time-bounded, and size-capped (SEC-V1);
   a remote reconcile source requires a bearer token (SEC-B7). Never follow
   a redirect to an attacker-chosen host on a server-side fetch (SSRF —
   SEC-B11 non-redirecting probe client).
8. **Integrity under concurrency.** Read-then-write critical sections
   (merge, bulk upsert) lock their rows / take an advisory lock, and audit
   + event rows commit **in the same transaction** as the change (SEC-B3,
   SEC-B5, SEC-B6, SEC-B10). At-least-once bus delivery ⇒ consumers dedupe
   on `event_id`.
9. **No secret in logs.** Never log a magic-link token, session id, bearer
   token, or signing seed (SEC-A3). Audit rows record *what happened* and
   the actor, never the credential.
10. **Least-authority artifacts.** Bulk artifacts are confined to their
    store base (reject `..` / absolute `file://`), owner-scoped on read
    (IDOR), and TTL'd (SEC-B4).

## 4. The activation gate — `<ENTITY>_REQUIRE_AUTH`

**The single most important operational fact.** All authentication and
authorization in the entity services is gated by a per-service
`<ENTITY>_REQUIRE_AUTH` flag that **defaults off**. With it off there is no
token requirement and no ABAC — audit, bulk-links, and PII reads are
**open**. This is deliberate (it lets the family ship and integrate before a
deployment's policy is written), but it means:

- **The shipped default is wide open.** Turning enforcement on is a
  **tracked release gate**, not an afterthought — a deployment exposed to
  untrusted callers MUST set `<ENTITY>_REQUIRE_AUTH` (and mount an ABAC
  policy) before it is reachable. SEC-G8 pins this with an explicit
  per-service test so activation cannot be forgotten silently.
- The flag is read **once at router construction**; changing it requires a
  restart.
- Record-level checks (masking obligations, `resource.*` policy) are a
  **no-op when the flag is off**, so they, too, only protect an activated
  deployment.
- Activation is the first item on the operational checklist
  (jwt-enforcement.md; the OPS-1 runbook): set the flag, mount the policy,
  publish/point at the PASETO keys, then verify a token is required.

## 5. Secret handling

- **Signing seed** (`TOKEN_PRIVATE_KEY_SEED`): required in production; the
  dev fallback is refused outside an explicitly non-production environment
  (SEC-A1). Never commit a real seed.
- **PASETO keys**: peers hold only the **public** key set, fetched over
  HTTPS (loopback excepted) or injected via `<ENTITY>_PASETO_KEYS`; no
  shared secret, no introspection hop (authentication-sessions.md §5).
- **Sessions**: opaque, high-entropy ids in `__Host-`, `HttpOnly`,
  `Secure`, `SameSite` cookies — never a JWT, never in `localStorage`
  (jwt.md, authentication-sessions.md §3–§4).
- **No secret in logs** (invariant 9). Config secrets come from the
  environment, never code or images.
- **Dependency supply chain**: `cargo-deny` + repo `deny.toml` gate
  advisories/licenses (SEC-I1); every crate root forbids `unsafe` (SEC-I3).

## 6. Threat model (summary)

- **Assets.** Personal data (person/worker/case PII; the `case ↔ person`
  edge is the most sensitive), identity-match correctness, the audit trail's
  integrity, and the signing key.
- **Actors.** Unauthenticated network callers; authenticated-but-
  lower-privilege callers (horizontal/vertical escalation); a compromised or
  buggy **peer service** (reconcile source, probed service); a malicious
  **bulk uploader**; an insider reading beyond their authority.
- **Trust boundaries.** The HTTP edge (blanket guard + ABAC); the token
  verification boundary (offline PASETO); the peer-to-peer boundary (bus
  events, reconcile pulls, presence probes); the bulk artifact store; the
  database transaction boundary (integrity/atomicity).
- **Primary threats & the controls that answer them.** Token forgery →
  fail-closed seed + offline verify (SEC-A1/V1/V4); privilege escalation →
  fail-closed ABAC + no vacuous negation + revoke-on-change
  (authorization-attributes.md, SEC-V2, SEC-A8); PII over-disclosure →
  masking on every read + governed-edge concealment (SEC-G1/G3); DoS →
  input + pagination + bulk caps + never-panic (SEC-M1/M4, SEC-G7, SEC-B2);
  false identity → no-spurious-match + check-digits (SEC-M2/M3/M5);
  integrity → locks + in-tx audit/events + idempotency (SEC-B3/B5/B6/B9/B10);
  SSRF/MITM → HTTPS + non-redirecting fetch + peer-token (SEC-V1, SEC-B7/B11).

## 7. Status snapshot

> **Update 2026-08-20.** Two further findings, both surfaced by new
> tooling rather than by re-reading the code — which is the argument for
> the tooling. **SEC-M7** (closed): `integrity-mac`'s `decode_hex`
> sliced its input as a `&str`, so a stored MAC containing a multi-byte
> character (`k1:€a`) **panicked** instead of returning `Malformed` —
> invariant 2, reachable by anyone who can write to the database, which
> is exactly the adversary the MAC exists to defend against. Found on
> the first run of a new `cargo-fuzz` target and fixed by decoding over
> bytes. **SEC-M8** (closed everywhere): an
> over-long array was reported once for cardinality and then still
> enumerated, so ten thousand blank entries produced ten thousand
> problem strings in one `422` body — a small request buying a large
> response. Bounding the **report** is part of the same input-bounding
> rule as bounding the work (invariant 3); **rolled to organization,
> care-pathway and portfolio on 2026-08-21 (SEC-M8b)**, where each entry
> was additionally running a check-digit or terminology validation —
> care-pathway's rejection path measured 112 µs → 4.9 µs. Both are in
> [tasks.md](../../tasks.md) Phase 7; `entity-ref` and `integrity-mac`
> gained fuzz harnesses in the same pass, so SEC-I2's coverage is no
> longer matcher-only.

Phase 5 is tracked in [tasks.md](../../tasks.md). **As of 2026-08-05,
every `SEC-*` item in tasks.md is `[x]`** — the criticals, the
lower-severity authn hardening (SEC-A5/A6/A9/A10), the default-off pin
(SEC-G8), `cargo-fuzz` scaffolding (SEC-I2), and the last two residuals:
SEC-B2 (bulk import now genuinely streams, memory peak measured flat
regardless of file size) and SEC-B4 (bulk artifacts are now physically
deleted on a TTL sweep, not just gated on read) both closed 2026-08-05,
and SEC-V3 (malformed key-set load resilience) was resolved the same day
as a **decision, not a code change** — fail-fast stays, see §2. Consult
tasks.md for the authoritative, up-to-date checklist and each crate's
`CHANGELOG.md` for the landed fix.
