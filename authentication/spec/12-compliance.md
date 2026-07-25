## 12. Compliance

A governmental deployment makes this entity's compliance posture
load-bearing for the whole federation. Frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md);
healthcare contexts add
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

| Standard | Mechanism |
|---|---|
| GDPR / UK GDPR / UK DPA 2018 | Email addresses and names are personal data: data minimisation (the account holds only `email` + `name`), no tokens logged alongside avoidable PII in production, purpose limitation (sign-in only — identity attributes live in the person entity). **Right of access** (Art. 15) via `GET /api/auth/account/export`; **right to erasure** (Art. 17) via `DELETE /api/auth/account` — soft-delete + anonymise (§13 T-9, done). See "GDPR subject rights" below. |
| GDPR Art. 32 (security of processing) | Passwordless (no password database to breach); **server-side cookie sessions** (opaque `sid` in an `HttpOnly`/`Secure`/`__Host-` cookie — no token in browser JS, killing the `localStorage` exfiltration class); **CSRF protection** on cookie-authenticated mutating requests; Ed25519 asymmetric PASETO keys (no shared secret); short ~5-min token TTL; TLS at the edge. |
| ISO/IEC 27001 (ISMS) | Access control alignment: single sign-on chokepoint, server-side sessions with login + revocation timestamps and idle+absolute TTLs, key custody via environment injection (A.9 / A.10-style controls); operational controls are deployment-side. |
| ISO/IEC 42001:2023 (AIMS) | Not applicable today — no ML in the auth path; applies family-wide where matcher tuning is ML-driven. |
| Welsh Language (Wales) Measure 2011 / public-sector Welsh-language duty | The user-facing surfaces (magic-link email + front-end UI) ship in **Welsh (`cy`)** alongside English (T-7), so a Welsh-speaking citizen can sign in in Welsh — treating Welsh no less favourably than English. Locale catalogs (`src/i18n.rs`, `src/lib/i18n.svelte.ts`) are structured so further national languages are added by extension. |

### Audit of authentication events

Two complementary trails:

1. **`sessions`** is the login/revocation trail: every magic-link
   redemption writes a session row `(sid, user_pid, created_at,
   idle_expires_at, absolute_expires_at)`; every signout / admin revoke
   stamps `revoked_at`; `email_verified_at` records first verification.
2. **`auth_events`** (T-10, done) is the durable security/compliance
   audit trail of authentication *events*. Each row is
   `(id, event, email, user_pid, detail, created_at)`. Events:
   `signup`, `magic_link_requested`, `magic_link_redeemed`, `signout`
   (and `me` is reserved). `detail` is an outcome marker
   (`rate_limited` / `unknown_email` / `invalid_or_expired` / `issued` /
   `created` / `existing` / `ok` / `rejected`). Writes are **best-effort**
   (`Model::record_best_effort` logs on failure but never fails the
   request). The audit row may distinguish outcomes for security review,
   but the HTTP response does **not** — the anti-enumeration contract
   holds at the wire (e.g. an `unknown_email` magic-link request is
   audited as such yet still returns the same `200`). **No tokens or
   secrets are ever stored** (only event names, normalised emails,
   subject pids, and outcome markers).

The trail is queryable at `GET /api/auth/audit/recent` (newest 100,
`AuthEvent[]`). It mirrors the family `/audit/recent` pattern (see
[`agents/share/auditability.md`](../../agents/share/auditability.md)).

**Audit gating decision (T-9).** The system-wide `/audit/recent` is left
**unauthenticated** (operator-facing; consistent with the sibling
care-pathway service; rows carry no tokens or secrets). The GDPR
right-of-access requirement is met instead by a bearer-gated,
**per-subject** `GET /api/auth/account/audit` that returns only the
caller's own events — so a subject's own audit trail (and the export)
is reachable only by that subject, while the system feed stays open for
operators. Magic-link issuance also remains traced (structured tracing
with the email field).

### GDPR subject rights (T-9, done)

- **Right of access (Art. 15)** — `GET /api/auth/account/export`
  (session cookie) returns the subject's `users` row + `sessions` +
  `auth_events`. The export never includes a token, key material, the
  password hash, or the api key.
- **Right to erasure (Art. 17)** — `DELETE /api/auth/account` (session
  cookie, CSRF-protected) is **soft-delete + anonymise**, chosen over a
  hard delete so the
  `auth_events` audit trail and any referential history keep their
  integrity: a new `users.deleted_at` column is stamped, `email` becomes
  a `pid`-keyed unroutable tombstone (`deleted+<pid>@invalid`, RFC 2606
  `.invalid`, preserving `UNIQUE(email)`), `name` becomes
  `"deleted user"`, magic-link material is cleared, **all** the
  subject's sessions are revoked, and an `account_erased` audit row is
  written. Erasure is irreversible (the tombstone cannot reconstruct the
  original address). Every read path (`/me`, export) treats a
  `deleted_at` user as gone (`401`); revoking the sessions stops new
  PASETO minting, and any already-minted cross-service token expires
  within its ~5-min TTL (the same offline-revocation tradeoff as
  signout, FR-8).

### Credential handling rules

- The **session cookie** (`__Host-mxi_session`) is the human
  credential: `HttpOnly` (never reachable from browser JS), `Secure`,
  `__Host-` host-locked, `SameSite`; never logged. The browser holds
  **no token** — `localStorage` is not used for credentials.
- The **PASETO v4.public** token is the cross-service bearer credential:
  short-lived (~5 min), minted only by exchanging a session at
  `POST /token`, held server-side at the BFF, never logged, never placed
  in browser JS.
- Magic-link tokens MUST NOT appear in production logs (the dev
  console log is a development affordance only).
- **Decommissioned:** the RS256 JWT bearer credential and its
  `localStorage` storage are removed by this pivot (§1; shared §9).

### Extended frameworks

Four frameworks impose obligations beyond the table above. Regime detail:
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2; repository-wide status and the reference implementation:
[`spec/compliance` §8](../../spec/compliance/index.md). This entity sits
differently from the registries: it holds almost no data, but it **is**
the access-control and authentication mechanism every other entity's
compliance claim depends on — so the frameworks engage through the
*controls* it provides, not through the data it stores.

| Framework | Engagement here | What it drives |
|---|---|---|
| **HIPAA (US)** — §164.312(a)(1) access control, §164.312(d) person/entity authentication, §164.312(b) audit controls, §164.308(a)(1)(ii)(D) activity review | **Engaged as the mechanism.** §164.312(a)(1) and (d) are satisfied *here* on behalf of the whole family: unique user identification, verification of the entity seeking access, and — via the `sessions` idle/absolute TTLs — automatic logoff. `auth_events` is the family's authentication activity log. | **Tamper-evident history** over `auth_events` (a SHA-256 chain), because an authentication trail that can be quietly rewritten undermines every downstream registry's §164.312(b) claim; and preserving the §164.308(a)(1)(ii)(D) review path — `auth_events` must stay queryable, with the T-9 gating decision (system feed open, per-subject feed bearer-gated) revisited if a deployment's risk analysis says otherwise. |
| **GDPR / EU EHDS** — Reg. (EU) 2025/327 | GDPR fully engaged, though over a deliberately tiny surface (email + name). EHDS engages indirectly but importantly: **health-professional identity is what gates its primary-use exchange**, so this is where an EHDS deployment would attach professional-role attributes. | Reconciling **Art. 17 erasure with the tamper-evident chain**: today's erasure (soft-delete + email tombstone + session revocation, T-9) already chose redaction over hard delete for exactly the right reason — the chain makes that choice *provably* sound, since a redacted row keeps its linkage. Plus a declared **data residency** and **lawful basis**, and — for EHDS — attribute vocabulary for professional roles routed through the existing ABAC `attrs` claim rather than a new mechanism. |
| **ONC / HTI certification (US)** — 45 CFR Part 170 §170.315(g)(10) | **This is where SMART would live.** §170.315(g)(10) requires **SMART App Launch** — an OAuth 2.0 authorisation server with scoped access, launch contexts, refresh tokens, and a discoverable `/.well-known/smart-configuration`. This service is the family's authorisation server; the registries only *consume* its tokens. | An honest gap statement, not a claim. The family's credential is a **PASETO v4.public** token minted from a cookie session — deliberately **not** OAuth 2.0 and **not** SMART. A registry's `/fhir/.well-known/smart-configuration` therefore advertises the deployment's real authorisation server, and this service does **not** pretend to be one. Adding SMART App Launch would be a substantial new capability here (authorization endpoint, scopes, launch context, refresh) — a roadmap item, not a relabelling. |
| **IEC 62304 / SaMD** (with ISO 14971) | Not a device. In IEC 62304 terms this is a **supporting software item** of every clinical service that depends on it — an authentication failure is a plausible contributor to a downstream clinical hazard, which is precisely why supporting items are in scope. | A **SOUP register + CycloneDX SBOM** (this crate's dependency set — `rusty_paseto`, `ed25519-dalek`, `argon2` — is the most security-critical in the family); **machine-checked requirement→test traceability** over the token, session-TTL, CSRF, anti-enumeration, and fail-closed-seed controls (SEC-A1); and **signed, reproducible builds**, which matter more here than anywhere else because this binary holds the signing key. |

### Honest limits

- **No SMART App Launch, and none is claimed.** The gap is real and is
  stated as a gap. Any FHIR surface in the family that advertises SMART
  discovery points at a deployment's own authorisation server.
- **Not a certified health-IT module.** ONC certification targets FHIR
  **R4 + US Core** and, for authorisation, SMART App Launch — neither of
  which this service implements.
- **The chain is not yet built.** `auth_events` is append-only by
  convention (`record_best_effort` only inserts) but carries no hash
  linkage, so tampering is currently undetectable rather than merely
  unlikely. The reference implementation is the
  [care-pathway service](../../care-pathway/care-pathway-service-with-loco/).
- **Best-effort audit writes are a chain hazard.** `record_best_effort`
  deliberately never fails a request — but a dropped row leaves a gap the
  chain cannot distinguish from a deletion. Adopting the chain here means
  deciding, explicitly, whether an authentication audit write may still
  fail open.
