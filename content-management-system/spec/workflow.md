# Module 3 — Editorial workflow & governance

## The lifecycle

Per **variant** (an entry in one locale), enforced by a pure-core
state machine ([design](design.md) CMS-D4):

```
draft ──submit──▶ in_review ──approve──▶ approved ──publish──▶ published
  ▲                   │                     │                     │
  │                   └──reject(reason)─────┘                     │
  └───────────────────────────────────────── unpublish(reason)────┘
                                                                  │
published/approved/draft ──archive(reason)──▶ archived ──restore──┘
```

- An illegal transition is `422` **naming the current state** (the
  family convention), never a silent no-op.
- `reject`, `unpublish`, `archive`, and `restore` require a
  **reason**, stored in the audit snapshot
  ([audit](audit.md)).
- `archived` is terminal except a reasoned `restore` back to
  `draft`.
- **Direct-publish** (skipping review) is a *policy* question, not a
  code path: the same `publish` transition is simply permitted for
  an `access=admin`/editor persona from `draft`
  ([auth](auth.md)). One machine, many permission profiles.

## Publishing names a revision

`publish` sets `published_revision_pid` to a **specific** revision —
by default the current one, or an explicitly named earlier one.
Delivery serves that revision and nothing else
([delivery](delivery.md)). Consequences worth stating plainly:

- Editing after publishing changes **nothing** on the live site
  until the next publish. "Save" and "go live" are different verbs.
- `first_published_at` is preserved across unpublish/republish;
  `published_at` tracks the current live revision.
- Unpublish clears the pointer and (for routable types) creates a
  redirect or a `410` marker per site policy — an unpublished page
  should not become a bare 404 with no explanation
  ([delivery](delivery.md)).

## Review & approval

Submitting sets `reviewer_ref` (a `worker:` URN) and emits
`variant_submitted`. Approval records the approving actor and time
as facts; **the approver cannot be the author** when the site
declares separation of duties (`require_distinct_approver`,
default on) — an editorial control that is worth enforcing in the
machine rather than trusting to habit.

## Scheduling

`scheduled_publish_at` / `scheduled_unpublish_at` on an approved
variant. A `bg_pg` job sweeps due schedules and applies the same
transition the API would ([design](design.md) CMS-D14):

- **Idempotent per (variant, scheduled_at)** — a rerun or an
  overlapping worker never double-publishes.
- A schedule whose variant has since moved state is **skipped and
  recorded**, not force-applied.
- Scheduling emits `variant_scheduled`; the execution emits the
  ordinary `variant_published` / `variant_unpublished`, so
  consumers cannot tell (and need not care) whether a human or the
  clock did it — but the audit row records which.

## Locks and concurrency

Two layers, because neither alone is honest:

1. **Optimistic concurrency (authoritative).** Every save states its
   `base_revision_pid`; a stale base is `409` with the competing
   revision ([authoring](authoring.md)). This cannot be bypassed.
2. **Advisory locks (cooperative).** `locked_by_ref` +
   `locked_until` (auto-expiring, stealable by an editor persona
   with a reason) tell colleagues someone is typing. A lock reduces
   collisions; it never *guarantees* exclusivity, and the spec says
   so rather than implying a mutex the system does not have.

State transitions, audit rows, and outbox events share the
mutation's transaction; publish/unpublish races serialize on the
variant row (`FOR UPDATE`) so exactly one wins
([audit](audit.md)).

## Roles are policy

Author, editor, translator, admin, and the machine delivery peer
are **ABAC attribute profiles**, not a role enum in the code
([auth](auth.md)). The same endpoints serve all of them; what
differs is which transitions and which sites the policy allows,
with `resource.owner` `$sub` ownership for "my own drafts".
