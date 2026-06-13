## 12. Compliance

A governmental deployment makes this entity's compliance posture
load-bearing for the whole federation. Frameworks:
[`agents/share/compliance-for-technology.md`](../../agents/share/compliance-for-technology.md);
healthcare contexts add
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md).

| Standard | Mechanism |
|---|---|
| GDPR / UK GDPR / UK DPA 2018 | Email addresses and names are personal data: data minimisation (the account holds only `email` + `name`), no tokens logged alongside avoidable PII in production, purpose limitation (sign-in only — identity attributes live in the person entity). Right-of-access export and erasure workflow are gaps (§13 T-9). |
| GDPR Art. 32 (security of processing) | Passwordless (no password database to breach); RS256 asymmetric keys (no shared secret distribution); short token TTL; TLS at the edge. |
| ISO/IEC 27001 (ISMS) | Access control alignment: single sign-on chokepoint, per-token sessions with issuance + revocation timestamps, key custody via environment injection (A.9 / A.10-style controls); operational controls are deployment-side. |
| ISO/IEC 42001:2023 (AIMS) | Not applicable today — no ML in the auth path; applies family-wide where matcher tuning is ML-driven. |

### Audit of authentication events

The `sessions` table is the issuance/revocation trail: every token
issuance writes `(jid, user_pid, expires_at, user_agent)`; every
signout stamps `revoked_at`; `email_verified_at` records first
verification. Magic-link issuance is traced (structured tracing with
the email field).

**Gap:** unlike the sibling services there is no `audit_log` table and
no event streaming (`*Created` / `*Updated` events) for account
lifecycle and sign-in attempts — a governmental audit trail (who
attempted sign-in, from where, outcome) needs this (§13 T-10,
[`agents/share/auditability.md`](../../agents/share/auditability.md)).

### Token handling rules

- The JWT is the bearer credential for the entire family: never log
  it; the front-end keeps it in `localStorage` only, clears it on sign
  out and on `401`.
- Magic-link tokens MUST NOT appear in production logs (the dev
  console log is a development affordance only).
