# Identity (CIS2/OIDC) + ABAC — production gate sketch

> Part of the [Case Tracking specification](index.md). **Design sketch for
> a P0 production gate — not yet implemented.** Builds on the demo
> [magic-link auth](auth.md). Drives roadmap item **T-G1**. The
> authorization model follows the family-wide ABAC design in
> [authorization-attributes.md](../../agents/share/authorization-attributes.md)
> (this doc supersedes the earlier RBAC sketch).

Today the app authenticates via [magic link](auth.md) against a configured
**allowlist**, and a session JWT carries a free-text `role` that the UI only
*displays*. Before any live use this must become: (1) federated **identity**
via NHS CIS2 / OIDC, and (2) enforced **attribute-based authorization**
(ABAC). When T-G1 lands, the session's display-only free-text `role` is
**replaced by an `attrs` map** (string → strings), and every decision is a
policy evaluation over those attributes.

## 1. Identity — NHS CIS2 / OIDC

Replace the allowlist + magic link with an OIDC Authorization-Code-with-PKCE
flow against **NHS CIS2** (the national care-identity broker; smartcard or
authenticator).

- The magic-link `request`/`verify` endpoints are retired (or kept only for
  a break-glass path); `/api/auth/*` becomes `/login` → CIS2 → callback.
- The session is still a short-lived HttpOnly cookie ([auth.md](auth.md)),
  but its claims now come from the **CIS2 id_token**: `sub` (a stable user
  id), name, and the user's **national role codes** + activity codes.
- `identity.role` (free string, display-only) is replaced by
  `identity.attrs` — a **string → strings map of subject attributes**
  derived from CIS2 role/activity codes via a small **config-mapped
  table**. A "role", where a deployment wants one, is just another
  attribute (e.g. `duty=records`), per the family ABAC doc §1.

```
CIS2 id_token  ──►  role codes (e.g. "R8000"/activity codes)
                         │  configured mapping
                         ▼
              subject attrs, e.g.
                { "access": ["admin"] }            — trust admin
                { "duty":   ["records"] }          — records-office staff
                { "duty":   ["clinical"] }         — clinician
                { "duty":   ["porter"] }           — porter
                { }                                — unmapped ⇒ read-only
```

Unknown/unmapped identities get **empty `attrs`** — under the default
decision (§2) that means **read-only** (least privilege).

## 2. Authorization — ABAC

Authorization is a **policy** — an ordered list of allow/deny rules
evaluated top-down over (subject attributes, derived action, resource);
**first match wins**; if no rule matches, the **default decision** applies:
**read ⇒ allow, anything else ⇒ deny**. Same policy language and semantics
as [authorization-attributes.md](../../agents/share/authorization-attributes.md)
§4–§5.

**Action attributes** (derived per request, as in the family doc §2):
`read` for GET; `write` for the mutating POSTs; `destructive`/`delete`
reserved (the API has no destructive endpoints today — future exports or
purges would derive it).

**Resource attributes** stay coarse in v1: the collection segment of the
path — `moves`, `folders`, `volumes`, `places` — plus the raw `path`. This
carries the same granularity the old permission table encoded
(`MoveRecord` vs `FolderCreate` vs `VolumeManage` vs `PlaceManage`).

The capabilities the old 5-role × 6-permission table expressed become
example policy rules over attributes:

```jsonc
{
  "rules": [
    // was Administrator: everything (incl. reserved admin surface)
    { "effect": "allow", "actions": ["*"],
      "when": { "access": ["admin"] } },

    // was Records: all folder-tracking mutations, no admin surface
    { "effect": "allow", "actions": ["write"],
      "resources": ["moves", "folders", "volumes", "places"],
      "when": { "duty": ["records"] } },

    // was Clinician / Porter: may record moves, nothing else
    { "effect": "allow", "actions": ["write"],
      "resources": ["moves"],
      "when": { "duty": ["clinical", "porter"] } }

    // was Viewer: no rule needed — empty attrs fall through to the
    // default decision (read allow, mutation deny)
  ]
}
```

- `when` is a conjunction over subject attributes; a value list means
  "has **any** of these values"; a `!`-prefixed value negates — exactly
  the family language.
- `effect: deny` rules make exceptions expressible (e.g. deny a
  contractor attribute from `places` writes) and, being first-match-wins,
  beat later allows.
- Mutating endpoints covered by `write` on their resource:
  `POST /api/moves`, `POST /api/volumes/{id}/move` → `moves`;
  `POST /api/folders` → `folders`; volume create/rename/assign/remove →
  `volumes`; `POST /api/places` → `places`.

## 3. Enforcement (where it plugs in)

ABAC layers onto the **existing session guard** ([loco auth](../case-folder-service-with-rust/spec/auth.md)):

- The guard already resolves a `Session` identity for `/api/*`. Extend it
  to derive the **action** from the method and the **resource** from the
  path, then evaluate the policy against the session's `attrs`. Explicit
  allow ⇒ proceed; explicit deny or default-deny ⇒ **`403 Forbidden`**
  (distinct from the `401` for "not signed in"), body carrying the reason
  (denying rule index, or "default deny") — the family's 401/403 split.
- Realised as a Rust module, e.g. `src/auth/abac.rs` — `Policy`, `Rule`,
  `evaluate(attrs, action, resource) -> Decision` — mirroring the shared
  engine in `authentication-verifier` 0.3's `abac` module (this consumer
  app copies the policy shape rather than depending on the PASETO
  verifier; the app's credential is its own cookie session, not a PASETO).
  Policy comes from config (inline JSON or file); parse failure warns and
  falls back to the built-in default policy so the app always boots.
- Gated behind config `auth.abac.enforce` (default `false` in dev/test so
  the open demo + existing tests are unaffected, exactly like
  `require_session`).
- The Svelte client hides actions the session's attributes can't perform
  (progressive disclosure) and treats `403` as "not permitted" in the UI;
  the API stays authoritative.

## 4. Per-user attribution

With real identity, the move audit's `moved_by` comes from the **session
identity**, not a free-text field or a worker picker — closing the
[security gate](regulatory.md) "per-user attribution on `movedBy`".

## 5. Deliberately deferred

- **Record-level / purpose-based access** (Caldicott justification,
  "legitimate relationship" checks) — coarse attribute gates first;
  per-patient access later. This is the family doc's
  [§9 open question on record-level resource attributes](../../agents/share/authorization-attributes.md)
  (guard-after-load / handler-level checks); design when a concrete
  requirement lands.
- **Break-glass** emergency access with heightened audit (an environment
  attribute + obligation, per the family doc's §9 leanings).
- **Delegated administration** of the CIS2-code → attribute mapping and
  of the policy itself.

## 6. Acceptance (when implemented)

- A session with `attrs: {}` (or any set lacking a matching allow rule)
  calling `POST /api/places` receives `403`; a session with
  `access=admin` succeeds; `duty=records` also succeeds.
- A `duty=porter` session may `POST /api/moves` but receives `403` on
  `POST /api/places`.
- Every session — including empty `attrs` — may still read (`GET`)
  everything: default allow-read.
- An unauthenticated request still receives `401` (unchanged).
- A configured `deny` rule beats a later `allow` (first-match pin).
- With `auth.abac.enforce: false` every session behaves as today (open).
