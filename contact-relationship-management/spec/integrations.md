# Integrations

CRM is a consumer of the Main X Index family; it holds **EntityRef
URNs** and never duplicates upstream records.

| Service | Used for | How |
|---|---|---|
| [person-service](../../person/person-service-with-loco/) | contacts | required `person:` URN on Contact (and optional on Lead); display names resolved best-effort |
| [organization-service](../../organization/organization-service-with-loco/) | accounts | required `organization:` URN on Account |
| [worker-service](../../worker/worker-service-with-loco/) | reps & agents | `owner_ref` / `assignee_ref` / `actor_ref` URNs |
| [authentication-service](../../authentication/authentication-service-with-loco/) | SSO + ABAC attrs | offline PASETO via `authentication-verifier`; persona attributes |

Client modules follow the stub-first pattern (patient-flow
`clients.rs`): read-only, cached, best-effort display-name lookups
that never block writes.

## Dedup lives upstream

CRM performs **no** contact/account matching: creating a contact
requires an existing `person:` record, so duplicate humans are the
person-service matcher's problem. When upstream merges records and
emits `merged {pid, merged_from}`, CRM repoints its wrappers —
manual repoint endpoint in v1, event-driven on the durable bus as
roadmap.

## Events

Family envelope via the `CRM_EVENT_TRANSPORT` seam (default
`memory`; Postgres outbox rows share the mutation's transaction
under `outbox`). Event kinds are listed in
[domain-model.md](domain-model.md).
