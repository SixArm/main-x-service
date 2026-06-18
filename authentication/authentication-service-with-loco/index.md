# Authentication Service — documentation index

The central single sign-on provider for the Main X Index family:
passwordless email magic-link auth establishing a server-side cookie
session; short-lived PASETO v4.public tokens (published Ed25519 key at
`/.well-known/paseto-keys`) for offline cross-service verification. Built
on loco.rs.

> **Auth model source of truth:**
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> RS256 JWT + JWKS are **decommissioned**. **Pivot in progress** — the
> code follow-up is tracked in spec §13, so the current runtime may still
> emit JWTs until those tasks land.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§18). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; API surface; env vars. |
| [README.md](./README.md) | User-facing intro + quick start. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |

## Worked flow

```text
signup/signin  ──>  POST /api/auth/signup | /api/auth/magic-link  {email, locale?}
                       │  (magic link logged to console in dev;
                       │   optional locale en|cy picks the email language)
click link     ──>  GET  /api/auth/magic-link/{token}
                       │  -> establishes session; sets __Host-mxi_session cookie
use the session──>  GET  /api/auth/me            (session cookie via the BFF)
sign out       ──>  POST /api/auth/signout       (session cookie via the BFF)

GDPR (session) ──>  GET    /api/auth/account/export   (right of access: full data)
                    GET    /api/auth/account/audit    (own audit trail)
                    DELETE /api/auth/account          (right to erasure)

peers verify   ──>  GET  /.well-known/paseto-keys (fetch once, verify PASETO offline)
operators      ──>  GET  /metrics.prom           (Prometheus scrape; no PII labels)
```

## Worked examples

```bash
# Sign up requesting the Welsh-language magic-link email.
curl -s localhost:5150/api/auth/signup -H 'content-type: application/json' \
  -d '{"email":"chi@example.com","name":"Chi","locale":"cy"}'
# -> 200 {}  (always 200, identical shape regardless of locale)

# Redeem the logged link (establishes a session; sets the cookie):
curl -s -c cookies.txt localhost:5150/api/auth/magic-link/<TOKEN>
# -> Set-Cookie: __Host-mxi_session=<sid>; HttpOnly; Secure; SameSite=Lax; Path=/
#    body { "pid": "...", "name": "Chi", "email": "chi@example.com", "is_verified": true }

# GDPR right of access — export everything held about the subject:
curl -s -b cookies.txt localhost:5150/api/auth/account/export
# -> { "user": {...}, "sessions": [...], "auth_events": [...] }  (no secrets)

# GDPR right to erasure — soft-delete + anonymise + revoke sessions:
curl -s -b cookies.txt -X DELETE localhost:5150/api/auth/account
# -> 200; afterwards /me and the export return 401 for that subject.

# Operator metrics scrape (no email/token/pid labels):
curl -s localhost:5150/metrics.prom
```

## Relationship to the family

This is the first **real** loco.rs crate in the repo. The other service
crates declare `loco-rs` but run hand-rolled Axum; they will be converted
to idiomatic loco using this crate as the reference. See the root
[AGENTS.md](../../AGENTS.md).
