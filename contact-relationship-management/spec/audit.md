# Audit & events

## Audit trail

Family conventions: every mutation writes an audit row (entity
kind, pid, action, actor, snapshot). CRM-specific emphases:

- **Consent** is doubly recorded: the append-only `ConsentEvent`
  history *is* domain data, and each change also writes the normal
  audit row — GDPR accountability evidence from two angles.
- **Stage moves, reasoned actions** (deal reopen, lost reason,
  priority change, nurture exit, article archive) carry their
  reason in the snapshot.
- **Sensitive reads audited**: consent history views and unmasked
  deal-amount/forecast reads.

## Events

Family envelope (kinds in [domain-model.md](domain-model.md)),
deduped by consumers on `event_id`; transactional outbox under the
`outbox` transport. `sla_breached` is emitted once per breach by
the sweep job (idempotent on the breach fact).

## Integrity

State transitions + audit + outbox share one transaction; lead
conversion (lead → contact + deal) is atomic; the nurture scheduler
is idempotent per (enrolment, step); Kanban reorder races serialize
on the deal row lock (`FOR UPDATE`).
