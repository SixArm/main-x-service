# Regulatory, security & privacy

> Part of the [Case Tracking specification](index.md). Edition-specific
> gates: [loco regulatory](../case-folder-service-with-rust/spec/regulatory.md),
> [svelte regulatory](../case-folder-front-end-with-svelte/spec/regulatory.md).

> ⚠️ **Demo software.** The project is **not** a regulated medical
> record today. Everything below is a pre-production gate, not a claim
> of current compliance.

## Why the scope is deliberately narrow

The system tracks *where the paper is*, never *what is in it*. Keeping
clinical content out (see [scope.md](scope.md)) is what keeps the system
below **SaMD** (Software as a Medical Device) classification. Any feature
that would store or interpret clinical data must be challenged against
this boundary first.

## UK frameworks that apply before live use

| Framework            | Concern                                                              |
| -------------------- | -------------------------------------------------------------------- |
| **DCB0129 / DCB0160** | Clinical risk management (manufacturer + deploying organisation)     |
| **DSPT**              | Data Security and Protection Toolkit compliance                      |
| **UK GDPR / DPA 2018** | Lawful basis, data minimisation, retention for patient identifiers   |
| **Caldicott**         | Justify every use of patient-identifiable information                |
| **WCAG 2.2 AA**       | Accessibility (Public Sector Bodies Accessibility Regulations 2018)  |

## Security & privacy gates (pre-production checklist)

The three P0 gates are sketched as designs:
[authorization.md](authorization.md) (identity +
authorization), [audit-integrity.md](audit-integrity.md), and
[deployment.md](deployment.md).

- [ ] **Authentication + ABAC** via NHS CIS2 smartcard or OIDC/Azure AD —
      [authorization.md](authorization.md).
- [ ] **Per-user attribution** on every move (`movedBy` from the auth
      context, not free-text input) — [authorization.md](authorization.md) §4.
- [ ] **Append-only audit storage** with chained signatures for move
      events; backups + point-in-time recovery under NHS retention rules —
      [audit-integrity.md](audit-integrity.md).
- [ ] **TLS everywhere** — terminate at the ingress; HSTS; same-origin
      deployment of front-end + API — [deployment.md](deployment.md).
- [ ] **No PII beyond session lifetime in the browser** — no
      `localStorage`/IndexedDB of NHS Numbers.
- [ ] **Secrets** from a secrets manager, never committed config —
      [deployment.md](deployment.md).
- [ ] **CSP** disallowing inline scripts; reviewed external font/CDN use —
      [deployment.md](deployment.md).
- [ ] **API versioning** via `Accept` mediatype, not URL prefix.

Each gate is owned by the edition that can satisfy it; see the per-edition
regulatory files for the concrete settings (e.g. `auto_migrate: false`,
`ssr` re-enable).
