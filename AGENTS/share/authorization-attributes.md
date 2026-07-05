# Authorization attributes (ABAC) — design

How the Main X Index family authorizes **what an authenticated caller may
do**, on top of the authentication stack in
[authentication-sessions.md](authentication-sessions.md) (cookie sessions +
PASETO v4.public) and the blanket guard in
[jwt-enforcement.md](jwt-enforcement.md). The model is **attribute-based
access control (ABAC)**: decisions are policy evaluations over
**attributes** of the subject, the action, and the resource — not
membership in a fixed role list. This is a design document: it fixes the
attribute model, the wire format, the policy language, the default policy,
the 401/403 split, the sourcing path, and the rollout, so each crate
adopts it without re-litigating. It supersedes the earlier per-crate
role/RBAC sketches in §13 tasks (place "curator", event "scheduler", …)
and any interim role-vocabulary draft.

## 1. Why ABAC

Blanket enforcement (default-off) landed family-wide on 2026-07-04: a
valid PASETO token gates `/api/*`. That is authentication only — any
valid token can do anything. Fixed role lists (RBAC) were considered and
rejected: the nine registries serve different domains (clinical,
governmental, workforce, …) whose access rules hinge on *properties* —
department, clearance, purpose-of-use, service identity — not on one
shared role enum. ABAC evaluates policies over attributes; a role, where
one is wanted, is just another attribute (`role=editor`), so ABAC
strictly generalizes RBAC without freezing a vocabulary into nine crates.

## 2. The attribute model

A decision consumes three attribute sets:

**Subject attributes** — carried by the PASETO in a new `attrs` claim
(§3): a string→strings map, e.g.

```jsonc
"attrs": {
  "access":  ["write"],            // coarse tier (default policy, §5)
  "dept":    ["cardiology"],       // deployment-specific
  "purpose": ["care"],             // e.g. purpose-of-use
  "svc":     ["true"]              // machine-peer flag
}
```

Also available to policies as pseudo-attributes: `sub` (user pid) and
`email` from the verified claims.

**Action attributes** — derived per request, fixed family-wide:

| `action` | Derivation |
|---|---|
| `read` | GET / HEAD / OPTIONS |
| `write` | POST / PUT / PATCH (except destructive named POSTs) |
| `delete` | DELETE |
| `destructive` | DELETE, plus the crate's destructive named POSTs (record **merge**, batch **deduplicate**, bulk **import**) — declared per crate in a documented const, matched on path suffix |

(`delete` implies `destructive`; a rule targeting `destructive` covers
both. `destructive` POSTs are *not* `write`.)

**Resource attributes** — the blanket guard keeps these coarse:
`entity` (the crate's entity type, e.g. `place`) and `path`, so the
default no-record-load path stays fast. **Record-level** attributes
(e.g. a case's classification or sensitivity) are **delivered**: the
shared engine's `Policy::evaluate_with_resource` (verifier 0.4) takes a
`resource.*` attribute map, and a service that has loaded the target
record derives that map at the **handler level** (after fetch) and runs
a second, finer decision. The case service is the reference: its
single-case `GET`/`PUT`/`DELETE` handlers derive
`resource.case_type`/`status`/`priority` and gate on them (§9). The
guard itself still does not load records; record-level checks are
opt-in per handler where a concrete requirement exists.

## 3. Wire format — the `attrs` claim

- The verifier's `Claims` gains `attrs: BTreeMap<String, Vec<String>>`,
  `#[serde(default)]` — absent on old tokens ⇒ empty map; no re-issue
  needed. `authentication-verifier` bumps 0.2 → 0.3 (additive).
- The existing `roles` and `scope` claims are **deprecated for
  authorization** and ignored by the ABAC guard (kept on the wire for
  compatibility; removal is a future major).
- Attribute keys and values are short lowercase strings; multi-valued
  keys mean "has each of these values"; policies match set-membership.
  Unknown attributes are inert (forward-compatible).

## 4. The policy language

A **policy** is an ordered list of rules evaluated top-down;
**first match wins**; if nothing matches, the **default decision** (§5)
applies. Pure data (JSON), pure evaluation (no I/O):

```jsonc
{
  "rules": [
    { "effect": "allow",
      "actions": ["write", "destructive"],
      "when": { "svc": ["true"] } },                    // machine peers: everything
    { "effect": "allow",
      "actions": ["destructive"],
      "when": { "access": ["admin"] } },
    { "effect": "allow",
      "actions": ["write"],
      "when": { "access": ["write", "admin"] } },
    { "effect": "deny",
      "actions": ["read"],
      "when": { "dept": ["!cardiology"] } }             // example: dept-scoped read deny
  ]
}
```

- `actions` — which derived actions the rule covers (`*` = all).
- `when` — conjunction over subject attributes: every listed key must
  match. A value list means "subject has **any** of these values"
  (`["write","admin"]` = write OR admin). A `!`-prefixed value negates
  ("does not have"). An empty `when` matches every authenticated subject.
- `effect` — `allow` or `deny`. Deny rules make exceptions expressible;
  first-match-wins keeps evaluation O(rules) and auditable.
- The engine lives in `authentication-verifier` 0.3 (`abac` module:
  `Policy`, `Rule`, `evaluate(claims, action, entity) -> Decision`) so
  all nine services share one tested implementation instead of nine
  copies.

## 5. Default decision & default policy

Checks run **inside the blanket guard**, so they apply **only when
`<ENTITY>_REQUIRE_AUTH` is on**. Flag off ⇒ no authn and no authz —
pre-ABAC behavior; shipping this is behavior-neutral until activation.

With the flag on, for protected paths (public allow-lists unchanged):

```
no / invalid / expired token   → 401                     (unchanged)
valid token                    → evaluate policy on (attrs, action, entity)
  explicit allow               → allow
  explicit deny                → 403 + reason
  no rule matched              → default: read ⇒ allow, anything else ⇒ 403
```

- **Default-allow read, default-deny mutation.** Any authenticated
  subject may read (matches the family's pre-ABAC posture); every
  non-read action needs an explicit `allow`.
- **Built-in default policy** (used when no policy is configured): the
  first three rules of the §4 example — `svc=true` ⇒ everything;
  `access=admin` ⇒ destructive+write; `access=write` ⇒ write. This gives
  deployments a working coarse tier out of the box; richer policies are
  configuration, not code.
- **Configuration**: `<ENTITY>_ABAC_POLICY` (inline JSON) or
  `<ENTITY>_ABAC_POLICY_FILE` (path). Parse failure ⇒ `tracing::warn!` +
  fall back to the built-in default policy (the service always boots,
  matching the key-fetch posture). Read once at boot; restart to change.
- **401 vs 403**: 401 = missing/bad credential; 403 = valid credential,
  policy denied (body carries the reason and the denying rule index, or
  "default deny").

## 6. Sourcing (auth-service)

- `users` gains an `attributes` column (Postgres `JSONB`, default `{}`),
  holding the string→strings map.
- Session establishment (magic-link verify) copies the user's attributes
  into the session (`sessions.data`, per authentication-sessions.md §3).
- `POST /api/auth/token` mints the session's attributes into the `attrs`
  claim.
- Attribute **assignment** is an operator action, **delivered on two
  surfaces**: the `user_attributes` CLI task
  (`src/tasks/attributes.rs`: `op:show|set|unset|clear` over one user's
  `users.attributes`, selected by `email:`/`pid:`) and the admin HTTP
  API (`GET`/`PUT /api/auth/admin/users/{pid}/attributes`,
  `src/controllers/admin.rs`, gated by an `access=admin` caller — the
  bootstrap admin is assigned via the CLI). Both validate keys/values
  as short lowercase tokens (reserved `sub`/`email`/`entity` refused)
  and write an `attributes_assigned` `auth_events` **audit row**. Until
  assigned, users have `{}` (read-only under the default policy).
  Machine peers get `svc=true` tokens from ops.

## 7. Tests (per crate, DB-free, extending the guard's matrix)

- flag off + no token ⇒ allow (unchanged pin)
- on + valid token, `attrs {}` ⇒ GET allowed, POST 403 (default deny)
- on + `access=write` ⇒ POST/PUT allowed; DELETE 403; merge-POST 403
- on + `access=admin` ⇒ DELETE + merge allowed
- on + `svc=true` ⇒ everything allowed
- a configured deny rule beats a later allow (first-match pin)
- bad policy JSON ⇒ falls back to default policy, boots, warn-logged
- 401 vs 403 distinction pinned
- verifier crate: `attrs` claim round-trips mint→verify; absent claim ⇒
  empty map; engine unit tests (matching, negation, first-match, default)

## 8. Rollout

1. This doc (the contract).
2. `authentication-verifier` 0.3: `attrs` claim + the `abac` module.
3. Enforcement side in all nine entity services (guard derives the
   action, loads the policy at boot, calls the shared engine; §13
   role/RBAC items are re-pointed to ABAC and closed).
4. Sourcing side in the auth-service (§6), including both operator
   assignment surfaces (the `user_attributes` CLI task and the
   `access=admin`-gated HTTP admin API) and the `attributes_assigned`
   audit trail.
5. Activation remains the operational decision it already was
   (jwt-enforcement.md).

## 9. Record-level resource attributes (delivered)

Beyond the coarse blanket guard, a decision can consume attributes of
the **specific target record** — the property this section originally
deferred. The shared engine (verifier 0.4) exposes
`Policy::evaluate_with_resource(claims, action, entity, resource)`,
where `resource` is a string→strings map matched by `when` keys
prefixed **`resource.`** (e.g. `resource.status`). The `resource.`
namespace is disjoint from subject attributes, so a caller cannot spoof
a resource attribute through its token; under the plain `evaluate`
(no record loaded) every `resource.*` key resolves empty, keeping the
guard's coarse path sound.

Because loading the record requires a fetch, record-level checks are
**handler-level and opt-in**, not part of the blanket middleware: a
service loads the record, derives its resource attributes, and calls
`evaluate_with_resource` (gated on the same `<ENTITY>_REQUIRE_AUTH`
flag, so it is a no-op when enforcement is off). The **case service is
the reference**: `auth::case_resource_attrs` maps a case's
classification to `resource.case_type` / `resource.status` /
`resource.priority`, and `GET`/`PUT`/`DELETE /api/cases/{pid}` run the
second decision after loading (mutations evaluate the *stored*
record). A deployment then expresses e.g. "deny write when
`resource.status=closed` unless `access=admin`" purely as policy.
Other services adopt the same pattern where a concrete requirement
lands; a dedicated per-record **sensitivity tier** column (vs deriving
from existing fields) stays an optional per-entity add.

## 10. Open questions

- **Environment attributes** — time-of-day, source network. The engine's
  input map can carry them without a language change. (Lean: defer.)
- **Policy distribution** — per-service env/file now; a central policy
  service later if policies grow. (Lean: env/file until real pain.)
- **Obligations / advice** (mask-on-allow, audit-on-allow) — the case
  service's masking posture may eventually want decision-attached
  obligations; today masking stays endpoint-level. (Lean: defer.)
