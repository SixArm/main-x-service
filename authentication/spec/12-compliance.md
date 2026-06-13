## 12. Compliance

A governmental deployment makes this entity's compliance posture
load-bearing for the whole federation. Frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md);
healthcare contexts add
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

| Standard | Mechanism |
|---|---|
| GDPR / UK GDPR / UK DPA 2018 | Email addresses and names are personal data: data minimisation (the account holds only `email` + `name`), no tokens logged alongside avoidable PII in production, purpose limitation (sign-in only — identity attributes live in the person entity). **Right of access** (Art. 15) via `GET /api/auth/account/export`; **right to erasure** (Art. 17) via `DELETE /api/auth/account` — soft-delete + anonymise (§13 T-9, done). See "GDPR subject rights" below. |
| GDPR Art. 32 (security of processing) | Passwordless (no password database to breach); RS256 asymmetric keys (no shared secret distribution); short token TTL; TLS at the edge. |
| ISO/IEC 27001 (ISMS) | Access control alignment: single sign-on chokepoint, per-token sessions with issuance + revocation timestamps, key custody via environment injection (A.9 / A.10-style controls); operational controls are deployment-side. |
| ISO/IEC 42001:2023 (AIMS) | Not applicable today — no ML in the auth path; applies family-wide where matcher tuning is ML-driven. |

### Audit of authentication events

Two complementary trails:

1. **`sessions`** is the issuance/revocation trail: every token issuance
   writes `(jid, user_pid, expires_at, user_agent)`; every signout
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
  (bearer) returns the subject's `users` row + `sessions` +
  `auth_events`. The export never includes a token, key material, the
  password hash, or the api key.
- **Right to erasure (Art. 17)** — `DELETE /api/auth/account` (bearer)
  is **soft-delete + anonymise**, chosen over a hard delete so the
  `auth_events` audit trail and any referential history keep their
  integrity: a new `users.deleted_at` column is stamped, `email` becomes
  a `pid`-keyed unroutable tombstone (`deleted+<pid>@invalid`, RFC 2606
  `.invalid`, preserving `UNIQUE(email)`), `name` becomes
  `"deleted user"`, magic-link material is cleared, **all** the
  subject's sessions are revoked, and an `account_erased` audit row is
  written. Erasure is irreversible (the tombstone cannot reconstruct the
  original address). Every read path (`/me`, export) treats a
  `deleted_at` user as gone (`401`); the already-issued bearer token
  still verifies cryptographically until `exp`, bounded by the short TTL
  (the same offline-revocation tradeoff as signout, FR-8).

### Token handling rules

- The JWT is the bearer credential for the entire family: never log
  it; the front-end keeps it in `localStorage` only, clears it on sign
  out and on `401`.
- Magic-link tokens MUST NOT appear in production logs (the dev
  console log is a development affordance only).
