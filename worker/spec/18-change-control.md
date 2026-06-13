## 18. Change Control

Material changes to this spec — the DTO contract (§5.3), the REST /
envelope conventions (§9.3), shared invariants (scheme-locality,
soft-delete-only, one-persister), or compliance scope (§12) — MUST
land in the same commit as the corresponding code change in the
affected subproject(s).

Authority boundaries (restating the banner in
[`index.md`](index.md)):

- **Crate internals**: the subproject spec wins; this spec links
  down and summarises. If a summary here drifts, fix it here.
- **Integration contract**: this spec wins. If a crate change breaks
  the contract described here, either revert the change or change
  this spec in the same PR — never let the two disagree silently.

A contract change therefore typically touches **two repos' worth of
files in one PR**: this spec, the affected crate's spec section, the
code, and the seam test (§11.4). The three-part-PR rule (spec + code
+ test) applies at every level.

Bullet what changed, not how: every spec edit should be a diff a
reviewer can read in isolation.
