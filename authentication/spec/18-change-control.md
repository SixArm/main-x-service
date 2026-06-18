## 18. Change Control

Material changes to this spec — the session / cookie contract, the
PASETO claim set, the published-key (`/.well-known/paseto-keys`) shape
or location, the CSRF rules, the magic-link protocol, the verifier's
public API, token lifetimes, compliance scope — MUST land in the same
commit as the corresponding code change, alongside the affected
subproject's own spec edit where one exists. Three-part PRs: spec +
code + test.

### Authority boundaries (restated)

| Topic | Authority |
|---|---|
| Crate internals (module layout, model methods, controller wiring) | The subproject's own spec |
| Integration contract (session / cookie, PASETO claims, paseto-keys, CSRF, magic-link surface, verifier API) | **This spec** |
| Entity-wide goals (availability, compliance, roadmap) | **This spec** |

When the two disagree, fix the non-authoritative one via a §13 task —
never silently. The verifier crate now has its own spec
([spec/index.md](../authentication-verifier-rust-crate/spec/index.md),
§13 T-1): it is authoritative for crate internals, while this entity
spec remains authoritative for the integration contract.

### Editing discipline

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation. Avoid re-flowing surrounding
paragraphs in the same PR as a content change — keep stylistic churn
out of behavioural diffs.
