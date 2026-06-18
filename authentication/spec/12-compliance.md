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
