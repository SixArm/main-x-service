# Security policy

## Reporting a vulnerability

Email <joel@joelparkerhenderson.com> with a description of the issue,
the affected subproject(s), and reproduction steps. Please do not open
a public issue for a vulnerability. You should receive an
acknowledgement within a few business days.

## Scope

All subprojects in this monorepo: the entity-registry services, the
matcher and library crates (including `authentication-verifier` and
`integrity-mac`), the cross-cutting services, the SvelteKit front-ends,
and the consumer applications.

## What deployers must know

**The shipped default is wide open.** All authentication and
authorization in the entity services is gated by a per-service
`<ENTITY>_REQUIRE_AUTH` flag that defaults **off**. A deployment
exposed to untrusted callers MUST activate enforcement and mount an
ABAC policy before it is reachable. See
[agents/share/security.md](agents/share/security.md) for the security
posture, the cross-cutting invariants, secret-handling rules, and the
threat model, and
[agents/share/runbooks/](agents/share/index.md) for the operational
runbooks (activation, key rotation, and recovery procedures).

## Hardening history

A repo-wide security audit (2026-07-12) drove the `SEC-*` work
enumerated in [tasks.md](tasks.md) Phase 5; all items are closed.
Supply-chain gating runs in CI (`cargo deny`), every crate root
declares `#![forbid(unsafe_code)]`, and fuzz targets cover the
matchers, `entity-ref`, and `integrity-mac`.
