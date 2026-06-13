## 18. Change Control

Material changes to this spec — the DTO contract (§5.3), the wire
contract (§5.4, §9), shared invariants (§5.5), composition
requirements (§6.1), compliance scope (§12) — MUST land in the same
commit as the corresponding code change.

Because this spec governs the cross-subproject contract, a change
here usually touches **more than one subproject**: e.g. a new
deterministic identifier scheme is a matcher spec + code edit, a
service bridge-test edit, and (where exposed) a front-end edit — one
change cycle, each subproject's own spec updated alongside its code.

Authority on conflict (restated from [`index.md`](index.md)): crate
spec wins on crate internals; this spec wins on the integration
contract. Either way, open a §13 task to reconcile — never silently
rewrite the loser.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in under a minute. Long-form rationale belongs in
the commit message or a §16 open question.
