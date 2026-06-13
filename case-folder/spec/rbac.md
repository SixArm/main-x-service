# Identity (CIS2/OIDC) + RBAC — production gate sketch

> Part of the [Case Tracking specification](index.md). **Design sketch for
> a P0 production gate — not yet implemented.** Builds on the demo
> [magic-link auth](auth.md). Drives roadmap item **T-G1**.

Today the app authenticates via [magic link](auth.md) against a configured
**allowlist**, and a session JWT carries a free-text `role` that the UI only
*displays*. Before any live use this must become: (1) federated **identity**
via NHS CIS2 / OIDC, and (2) enforced **authorization** (RBAC).

## 1. Identity — NHS CIS2 / OIDC

Replace the allowlist + magic link with an OIDC Authorization-Code-with-PKCE
flow against **NHS CIS2** (the national care-identity broker; smartcard or
authenticator).

- The magic-link `request`/`verify` endpoints are retired (or kept only for
  a break-glass path); `/api/auth/*` becomes `/login` → CIS2 → callback.
- The session is still a short-lived HttpOnly cookie ([auth.md](auth.md)),
  but its claims now come from the **CIS2 id_token**: `sub` (a stable user
  id), name, and the user's **national RBAC role codes** + activity codes.
- `identity.role` becomes a typed set derived from CIS2 role codes, not a
  free string. A small **config-mapped table** translates the subset of
  CIS2 national role profiles the trust uses into the app roles below.

```
CIS2 id_token  ──►  role codes (e.g. "R8000"/activity codes)
                         │  configured mapping
                         ▼
                   app Role (Administrator | Records | Clinician | Porter | Viewer)
```

## 2. Authorization — RBAC

A small fixed set of **app roles** mapped to **permissions**. Every mutating
endpoint requires a permission; reads require `Read`.

| Permission       | Meaning                                             |
| ---------------- | --------------------------------------------------- |
| `Read`           | View any GET endpoint / page.                       |
| `MoveRecord`     | `POST /api/moves`, `POST /api/volumes/{id}/move`.   |
| `FolderCreate`   | `POST /api/folders`.                                |
| `VolumeManage`   | create/rename/assign/remove/move volumes.           |
| `PlaceManage`    | `POST /api/places`.                                 |
| `Admin`          | reserved (user/role administration, exports).       |

| Role            | Read | MoveRecord | FolderCreate | VolumeManage | PlaceManage | Admin |
| --------------- | :--: | :--------: | :----------: | :----------: | :---------: | :---: |
| `Administrator` |  ✓   |     ✓      |      ✓       |      ✓       |      ✓      |   ✓   |
| `Records`       |  ✓   |     ✓      |      ✓       |      ✓       |      ✓      |       |
| `Clinician`     |  ✓   |     ✓      |              |              |             |       |
| `Porter`        |  ✓   |     ✓      |              |              |             |       |
| `Viewer`        |  ✓   |            |              |              |             |       |

Unknown/unmapped identities default to `Viewer` (least privilege).

## 3. Enforcement (where it plugs in)

RBAC layers onto the **existing session guard** ([loco auth](../case-folder-service-with-rust/spec/auth.md)):

- The guard already resolves a `Session` identity for `/api/*`. Extend it so
  each route declares a required `Permission`; the guard checks
  `role.allows(permission)` and returns **`403 Forbidden`** (distinct from the
  `401` for "not signed in") when it fails.
- Realised as a Rust module, e.g. `src/auth/rbac.rs` — `enum Role`,
  `enum Permission`, `Role::from_identity(&str) -> Role`, and
  `Role::allows(Permission) -> bool` — plus a per-controller annotation of the
  permission each handler needs.
- Gated behind config `auth.rbac.enforce` (default `false` in dev/test so the
  open demo + existing tests are unaffected, exactly like `require_session`).
- The Svelte client hides actions the role can't perform (progressive
  disclosure) and treats `403` as "not permitted" in the UI; the API stays
  authoritative.

## 4. Per-user attribution

With real identity, the move audit's `moved_by` comes from the **session
identity**, not a free-text field or a worker picker — closing the
[security gate](regulatory.md) "per-user attribution on `movedBy`".

## 5. Deliberately deferred

- **Record-level / purpose-based access** (Caldicott justification, "legitimate
  relationship" checks) — coarse role gates first; per-patient access later.
- **Break-glass** emergency access with heightened audit.
- **Delegated administration** of the role→permission map.

## 6. Acceptance (when implemented)

- A `Clinician` session calling `POST /api/places` receives `403`; an
  `Administrator` succeeds.
- An unauthenticated request still receives `401` (unchanged).
- With `auth.rbac.enforce: false` every role behaves as today (open).
