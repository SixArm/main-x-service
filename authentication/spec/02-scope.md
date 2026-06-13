## 2. Scope

### 2.1 In scope — entity level

This spec owns the **integration contract** between the three
subprojects and the rest of the federation:

- The magic-link protocol surface: request, delivery, redemption,
  anti-enumeration behaviour.
- The JWT contract: RS256 algorithm, claim set (`sub`, `email`,
  `name`, `iss`, `aud`, `exp`, `iat`, `jti`), `kid` header, default
  token lifetime.
- The JWKS contract: location (`/.well-known/jwks.json`), document
  shape, `kid` derivation.
- The verifier-library contract: constructor inputs (JWKS + issuer +
  audience), per-request `verify`, error taxonomy.
- Entity-wide goals: availability targets, key-management posture,
  compliance, localisation, roadmap.

### 2.2 In scope — per subproject

| Subproject | Owns |
|---|---|
| [Service](../authentication-service-rust-crate/spec/index.md) | Sign up / sign in / sign out via magic link; JWT issuance; JWKS publication; session recording and revocation; the user record; mailer templates. |
| [Verifier](../authentication-verifier-rust-crate/) | Offline RS256 verification: JWKS parsing, `kid`-based key selection, signature + `iss` / `aud` / `exp` validation; optional HTTP JWKS fetch (`fetch` feature). Spec: [spec/index.md](../authentication-verifier-rust-crate/spec/index.md). |
| [Front-end](../authentication-front-end-with-svelte/spec/index.md) | The four routes (`/`, `/signup`, `/signin`, `/verify`), the API client, client-side session storage. |

### 2.3 Out of scope (today)

- Passwords, social login, multi-factor authentication.
- OAuth2 / OIDC flows (roadmap — §15).
- Roles / permissions / authorization — peer services authorize
  locally from claims.
- Organization / tenant modelling — see the
  [organization entity](../../organization/organization-service-rust-crate/).
- Account self-service beyond sign-in (profile editing, account
  recovery, deletion requests).
- Refresh tokens (open question — §16 OQ-1).
- Rate limiting / abuse resistance on magic-link issuance (roadmap —
  §15, task §13 T-6).
