## 14. Implementation Status

> **Pivot pending (2026-06-17).** The "Delivered" table below records
> the **RS256 JWT + JWKS** model, which is **superseded** by the cookie
> session + PASETO v4.public design
> ([`agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md);
> §1, §5, §9, §13 T-12). Until T-12 lands, the JWT model is still the
> code reality; the rows below describe that code, and the gap table
> (§14.2) tracks the pivot.

### 14.1 Delivered

| Capability | Subproject | Notes |
|---|---|---|
| loco.rs chassis | service | Real `Hooks` / `AppContext` boot; the family's reference loco app; loco-rs 0.16 |
| Magic-link flow | service | Signup + sign-in + redeem; 32-char token, 5-min expiry, single-use; anti-enumeration always-`200` |
| RS256 signing | service | `src/auth` — claims §5.3, `kid`-stamped header, env-driven key loading, fatal-boot on bad keys |
| JWKS endpoint | service | `/.well-known/jwks.json`, pre-rendered at boot; one RSA key |
| Sessions + signout | service | `sessions` rows (`jid` = `jti`); local revocation honoured by `/me` |
| Dev ergonomics | service | Console-logged magic links (no SMTP); committed dev keypair; Postgres queue |
| Mailer templates | service | `magic_link` / `welcome` / `forgot` (English only) |
| DB-free unit tests | service | Sign/verify round-trip, JWKS shape, tamper/garbage rejection — green |
| Verifier library | verifier | `Verifier` (from value / from URL behind `fetch`), `kid` selection, `iss`/`aud`/`exp` policy, typed errors, `forbid(unsafe_code)`, 9 unit tests — green |
| Operator UI | front-end | All four routes, lean raw-JSON client, runes session in `localStorage`, SPA config; `pnpm run check` clean; build succeeds |
| Verifier doc set (T-1) | verifier | README, CHANGELOG, spec §1–§18, AGENTS.md/CLAUDE.md/index.md; `cargo package --list` green |
| Magic-link request tests (T-3) | service | Signup / magic-link / redeem (single-use, anti-enumeration) / me / signout / JWKS; Postgres-backed tests `#[ignore]`d, DB-free assertions un-gated |
| Cross-crate contract test (T-4) | service + verifier | `tests/sign_verify_contract.rs` — service signs, verifier verifies; claims round-trip; `kid` thumbprint pinned; DB-free |
| GDPR subject rights (T-9) | service | Right of access `GET /api/auth/account/export` + per-subject `GET /api/auth/account/audit` (bearer); right to erasure `DELETE /api/auth/account` — soft-delete + anonymise (`users.deleted_at`, migration `m20220101_000004`), revoke sessions, `account_erased` audit; `/me` + export `401` post-erasure; un-gated unit + DB-gated request tests; OpenAPI updated |

### 14.2 Open gaps

Open gaps drive tasks in §13. Live gap list:

| Gap | Task |
|---|---|
| Verifier absent from root `AGENTS.md` / `overview.md` (root docs; outside this entity's write scope) | T-2 |
| Single JWKS key; no rotation procedure | T-5 |
| No rate limiting on magic-link issuance | T-6 |
| Emails / UI English-only | T-7 |
| No OpenAPI / Swagger for the service | T-8 |
| No audit log or event streaming for auth events (sessions table only) | T-10 |
| No front-end unit / e2e tests | T-11 |
| **JWT-for-sessions pivot** — cookie sessions + PASETO v4.public, remove RS256/JWKS, BFF front-end, CSRF | **T-12** (supersedes the JWT model; shared design doc) |
| Refresh tokens, revocation propagation, audience model | §16 open questions |
| OIDC, WebAuthn/passkeys, multi-region, PASETO enforcement at peers | §15 roadmap |
