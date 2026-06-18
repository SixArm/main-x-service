## 1. Purpose and Vision

### 1.1 Purpose

The authentication entity is the **central single sign-on provider**
for the Main X Index — the federated identity index serving a
worldwide public governmental system. It authenticates human operators
and, ultimately, citizen-scale user populations with **passwordless
email magic links**. The **human session** is a server-side
Postgres-backed **cookie session** (an opaque session id in an
httpOnly cookie — never a token in browser JS); **cross-service**
authentication is preserved by exchanging that session for a
short-lived **PASETO v4 public** token that every other entity service
verifies **offline** against a published Ed25519 public-key set. There
is one place to sign in, one place to revoke, and one set of keys to
trust.

> **Pivot (2026-06-17).** This supersedes the previous **RS256 JWT +
> JWKS** access-token model. Per the family design doc
> [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md)
> (operationalising the principle in
> [`agents/share/jwt.md`](../../agents/share/jwt.md): "JWTs must not be
> used to keep users logged in — use cookie sessions"), the human
> session moves to a server-side cookie session and the cross-service
> token moves from RS256 JWT to PASETO v4.public. RS256 signing and
> `/.well-known/jwks.json` are **decommissioned** (see §9, §13, §15).

The entity comprises three subprojects:

| Subproject | Role |
|---|---|
| [authentication-service-with-loco](../authentication-service-with-loco/) | **Sessions + token issuance** — magic-link flow, server-side cookie sessions, PASETO v4.public minting (`POST /token`), Ed25519 key publication (`/.well-known/paseto-keys`). The family's reference loco.rs application. |
| [authentication-verifier-rust-crate](../authentication-verifier-rust-crate/) | Token **verification** — the dependency-light library peer services embed to verify PASETO tokens offline against the published Ed25519 keys. |
| [authentication-front-end-with-svelte](../authentication-front-end-with-svelte/) | Operator **UI / BFF** — sign up / sign in / sign out via magic link; holds the session cookie and exchanges it for a PASETO server-side. |

### 1.2 Vision

A single passwordless sign-on surface for millions of operators and
citizens across all Main X Index entity services, where:

- **Sign-in is passwordless.** No passwords to phish, leak, stuff, or
  reset at population scale — a magic link to a verified email address
  is the sole factor today, with WebAuthn / passkeys as a planned
  second passwordless factor (§15).
- **Verification is offline.** Peer services embed the verifier
  library and check PASETO v4.public signatures locally against a
  cached Ed25519 key set — zero network calls on the request hot path,
  and no runtime dependency on the auth service for request
  verification. The auth service can be down and every other service
  keeps authenticating traffic until the short-lived tokens expire.
- **Trust is auditable.** Every login is a server-side `sessions` row;
  every revocation is a `revoked_at` timestamp. The trail supports the
  strict auditability a governmental deployment demands (§12).
- **The surface is worldwide.** User-facing emails and UI localise
  across the family's supported locales
  ([`agents/share/locales.md`](../../agents/share/locales.md)) — a
  roadmap commitment (§15), not yet implemented (§14).

### 1.3 Non-goals

- **Not an identity-proofing service.** It verifies control of an
  email address, not real-world identity. Identity attributes,
  documents, and matching live in the
  [person entity](../../person/person-service-with-loco/).
- **Not the person registry.** The `users` table holds sign-in
  accounts (`pid`, email, name), not demographic records.
- **Not an OAuth2 / OIDC provider — yet.** The code implements a
  bespoke magic-link + cookie-session + PASETO protocol, not the OAuth2
  authorization-code flow or OIDC discovery / userinfo. OIDC
  compliance is roadmapped (§15) so standard clients can integrate.
- **Not an authorization service.** Tokens carry identity claims only
  (no roles / permissions); peer services authorize locally from
  claims.
