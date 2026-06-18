## 2. Scope

### 2.1 In scope — entity level

This spec owns the **integration contract** between the three
subprojects and the rest of the federation:

- The magic-link protocol surface: request, delivery, redemption,
  anti-enumeration behaviour.
- The **session contract**: server-side `sessions` table (§5.2),
  the `__Host-mxi_session` httpOnly cookie, idle + absolute TTLs,
  rotation, and revocation (per
  [`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md) §3).
- The **CSRF contract** for cookie-authenticated mutating requests
  (shared §4).
- The **cross-service token contract**: PASETO **v4.public**
  (Ed25519), claim set (`sub`, `iss`, `aud`, `iat`, `nbf`, `exp` ~5
  min, `sid`, `scope`/`roles`), `kid` in the footer, `POST /token`
  exchange (shared §5).
- The **public-key contract**: location (`/.well-known/paseto-keys`),
  document shape, `kid` derivation.
- The verifier-library contract: constructor inputs (PASETO keys +
  issuer + audience), per-request `verify`, error taxonomy.
- Entity-wide goals: availability targets, key-management posture,
  compliance, localisation, roadmap.

### 2.2 In scope — per subproject

| Subproject | Owns |
|---|---|
| [Service](../authentication-service-with-loco/spec/index.md) | Sign up / sign in / sign out via magic link; server-side cookie sessions; PASETO v4.public minting (`POST /token`); Ed25519 key publication; CSRF; session recording and revocation; the user record; mailer templates. |
| [Verifier](../authentication-verifier-rust-crate/) | Offline PASETO v4.public verification: Ed25519 key-set parsing, `kid`-based key selection, signature + `iss` / `aud` / `exp` validation; optional HTTP key fetch (`fetch` feature). Spec: [spec/index.md](../authentication-verifier-rust-crate/spec/index.md). |
| [Front-end](../authentication-front-end-with-svelte/spec/index.md) | The four routes (`/`, `/signup`, `/signin`, `/verify`); the SvelteKit-server **BFF** that holds the session cookie and exchanges it for a PASETO server-side; CSRF on mutating calls. |

### 2.3 Out of scope (today)

- Passwords, social login, multi-factor authentication.
- OAuth2 / OIDC flows (roadmap — §15).
- Roles / permissions / authorization — peer services authorize
  locally from claims.
- Organization / tenant modelling — see the
  [organization entity](../../organization/organization-service-with-loco/).
- Account self-service beyond sign-in (profile editing, account
  recovery, deletion requests).
- Refresh tokens (open question — §16 OQ-1).
- Rate limiting / abuse resistance on magic-link issuance (roadmap —
  §15, task §13 T-6).
