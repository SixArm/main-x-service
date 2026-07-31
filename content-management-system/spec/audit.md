# Audit & events

## Audit trail

Family conventions: every mutation writes an audit row (entity
kind, pid, action, actor, snapshot). CMS-specific emphases:

- **Revision history *is* domain data**, and each save also writes
  the normal audit row. Two angles on the same fact, deliberately:
  the revision chain answers "what did it say?", the audit trail
  answers "who did what to it, and why?".
- **Reasoned actions** — reject, unpublish, archive, restore, lock
  steal, force-delete of a referenced asset, and a breaking content
  type change — carry their reason in the snapshot. An action that
  requires a reason and receives none is refused, not defaulted.
- **Publish is audited with the revision it published**, not merely
  "published" — the difference between an accountable record and a
  timestamp.
- **Sensitive reads audited**: unpublished/embargoed revision
  reads, preview-token issue and use, audit queries themselves, and
  asset force-deletes ([auth](auth.md)).
- **Scheduled executions record their trigger** (`actor = system`,
  plus the scheduling actor), so "who published this at 3am" has an
  answer.

## History is append-only

Revisions are never updated or deleted; a restore writes a **new**
revision referencing what it copied
([authoring](authoring.md)). The same posture as the family's
tamper-evident audit trail: a history that can be edited is not
history. Retention/erasure requests are handled by **redacting a
revision's body while preserving the row, its number, and its
linkage** — the family's GDPR-versus-immutable-history resolution
([compliance-for-healthcare](../../agents/share/compliance-for-healthcare.md)
§2.2), audited as a redaction with its authority.

## Events

Family envelope (kinds in [domain-model.md](domain-model.md)),
deduped by consumers on `event_id`; transactional outbox under the
`outbox` transport. Notes:

- `variant_published` carries the published revision id — the
  single most consumed event (CDN purge, static rebuild, search
  re-index), and useless without it.
- Scheduled publish emits the ordinary `variant_published`; the
  schedule sweep is idempotent per (variant, scheduled_at), so a
  rerun emits nothing new.
- **Webhooks are driven from the event record**, so no extension
  can observe a change the audit trail does not contain
  ([integrations](integrations.md)).

## Integrity

State transitions + audit + outbox share one transaction. Publish,
unpublish, and schedule execution serialize on the variant row
(`FOR UPDATE`) — exactly one winner. Revision numbers are allocated
under that lock, so the chain has no gaps and no duplicates.
Reference extraction commits with the revision that produced it, so
"where used" can never disagree with the content. Route uniqueness
and redirect loop-freedom are enforced by constraint plus a
write-time check inside the same transaction, not by a background
repair job.
