# AGENTS directory — Authentication entity

Entity-level reference documentation for the authentication entity:
the central single sign-on provider trio (service + verifier library +
front-end). The entity-level living spec is
[`../spec/index.md`](../spec/index.md).

> **Auth model source of truth:**
> [`../../agents/share/authentication-sessions.md`](../../agents/share/authentication-sessions.md).
> The session is a server-side httpOnly **cookie session**; cross-service
> auth is short-lived **PASETO v4.public** (published Ed25519 key at
> `/.well-known/paseto-keys`). The old **RS256 JWT + JWKS** model is
> **decommissioned**. **Pivot in progress** — the service code follow-up
> is tracked in the service spec §13; the verifier and front-end docs are
> already harmonized.

## Documents in this directory

| Document | Description |
|----------|-------------|
| [spec-driven-development.md](spec-driven-development.md) | SDD discipline at entity level — authority model, three-part PRs, section mapping |
| [subprojects.md](subprojects.md) | The three subprojects: responsibilities, dependency direction, how to run each |
| [models.md](models.md) | Domain model reference (`User`, `Session`, `Claims`, published PASETO keys) |
| [verification.md](verification.md) | How a peer service verifies tokens — verifier API, key caching, claim rules (this entity's counterpart to siblings' `matching.md`) |
| [restful.md](restful.md) | REST API + verifier library API + front-end route reference |
| [testing.md](testing.md) | Testing strategy across the three subprojects |

## Subproject docs

| Subproject | Docs |
|---|---|
| [authentication-service-with-loco](../authentication-service-with-loco/) | [spec](../authentication-service-with-loco/spec/index.md) · [AGENTS.md](../authentication-service-with-loco/AGENTS.md) · [README](../authentication-service-with-loco/README.md) |
| [authentication-verifier-rust-crate](../authentication-verifier-rust-crate/) | [spec](../authentication-verifier-rust-crate/spec/index.md) · [AGENTS.md](../authentication-verifier-rust-crate/AGENTS.md) · [README](../authentication-verifier-rust-crate/README.md) |
| [authentication-front-end-with-svelte](../authentication-front-end-with-svelte/) | [spec](../authentication-front-end-with-svelte/spec/index.md) · [AGENTS.md](../authentication-front-end-with-svelte/AGENTS.md) · [README](../authentication-front-end-with-svelte/README.md) |

## Shared documents (project root)

The shared reference docs live at the project root under
[`../../agents/share/`](../../agents/share/).

| Document | Description |
|----------|-------------|
| [overview.md](../../agents/share/overview.md) | High-level project overview |
| [architecture.md](../../agents/share/architecture.md) | Layered architecture |
| [loco.md](../../agents/share/loco.md) | Loco framework conventions |
| [rust-loco-stack.md](../../agents/share/rust-loco-stack.md) | Full dependency inventory |
| [availability.md](../../agents/share/availability.md) | Health checks, scaling |
| [auditability.md](../../agents/share/auditability.md) | Audit logging and event streaming |
| [observability.md](../../agents/share/observability.md) | Tracing + OpenTelemetry summary |
| [privacy.md](../../agents/share/privacy.md) | Masking, GDPR, consent |
| [restful.md](../../agents/share/restful.md) | REST API conventions |
| [postgresql.md](../../agents/share/postgresql.md) | PostgreSQL setup |
| [locales.md](../../agents/share/locales.md) | i18n & l10n |
| [compliance-for-technology.md](../../agents/share/compliance-for-technology.md) | ISO, GDPR, … |
| [compliance-for-healthcare.md](../../agents/share/compliance-for-healthcare.md) | HIPAA, NHS, … |

## See also

- [`../spec/index.md`](../spec/index.md) — entity-level living spec
  (authority for the cross-subproject contract).
- [`../../AGENTS.md`](../../AGENTS.md) — monorepo entry point.
