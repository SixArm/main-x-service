## 1. Purpose and Vision

### 1.1 Purpose

The authentication entity is the **central single sign-on provider**
for the Main X Index — the federated identity index serving a
worldwide public governmental system. It authenticates human operators
and, ultimately, citizen-scale user populations with **passwordless
email magic links**, and issues **RS256 JWT** access tokens that every
other entity service verifies **offline** against a published JWKS.
There is one place to sign in, one place to revoke, and one set of
keys to trust.

The entity comprises three subprojects:

| Subproject | Role |
|---|---|
| [authentication-service-rust-crate](../authentication-service-rust-crate/) | Token **issuance** — magic-link flow, RS256 signing, JWKS publication, sessions. The family's reference loco.rs application. |
| [authentication-verifier-rust-crate](../authentication-verifier-rust-crate/) | Token **verification** — the dependency-light library peer services embed to verify tokens offline against the JWKS. |
| [authentication-front-end-with-svelte](../authentication-front-end-with-svelte/) | Operator **UI** — sign up / sign in / sign out via magic link. |

### 1.2 Vision

A single passwordless sign-on surface for millions of operators and
citizens across all Main X Index entity services, where:

- **Sign-in is passwordless.** No passwords to phish, leak, stuff, or
  reset at population scale — a magic link to a verified email address
  is the sole factor today, with WebAuthn / passkeys as a planned
  second passwordless factor (§15).
- **Verification is offline.** Peer services embed the verifier
  library and check signatures locally against a cached JWKS — zero
  network calls on the request hot path, and no runtime dependency on
  the auth service for request verification. The auth service can be
  down and every other service keeps authenticating traffic until
  tokens expire.
- **Trust is auditable.** Every token issuance is a `sessions` row;
  every revocation is a timestamp. The trail supports the strict
  auditability a governmental deployment demands (§12).
- **The surface is worldwide.** User-facing emails and UI localise
  across the family's supported locales
  ([`agents/share/locales.md`](../../agents/share/locales.md)) — a
  roadmap commitment (§15), not yet implemented (§14).

### 1.3 Non-goals

- **Not an identity-proofing service.** It verifies control of an
  email address, not real-world identity. Identity attributes,
  documents, and matching live in the
  [person entity](../../person/person-service-rust-crate/).
- **Not the person registry.** The `users` table holds sign-in
  accounts (`pid`, email, name), not demographic records.
- **Not an OAuth2 / OIDC provider — yet.** The code implements a
  bespoke magic-link + JWT + JWKS protocol, not the OAuth2
  authorization-code flow or OIDC discovery / userinfo. OIDC
  compliance is roadmapped (§15) so standard clients can integrate.
- **Not an authorization service.** Tokens carry identity claims only
  (no roles / permissions); peer services authorize locally from
  claims.
