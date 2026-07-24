# Integrations

HCM is a consumer of the Main X Index family; it holds **EntityRef
URNs** and never duplicates upstream records.

| Service | Used for | How |
|---|---|---|
| [person-service](../../person/person-service-with-loco/) | the human | `person:<pid>` on Employee and (optionally) Candidate; display names resolved best-effort |
| [worker-service](../../worker/worker-service-with-loco/) | professional identity | `worker:<pid>` on Employee; interviewers |
| [organization-service](../../organization/organization-service-with-loco/) | the employer | `organization:<pid>` on Employee, PayrollRun |
| [course-service](../../course/course-service-with-loco/) | training | `course:` / `courseinstance:` URNs on TrainingEnrollment |
| [authentication-service](../../authentication/authentication-service-with-loco/) | SSO + ABAC attrs | offline PASETO via `authentication-verifier`; persona attributes |

Client modules follow the stub-first pattern (patient-flow
`clients.rs`): display-name lookups are read-only, cached,
best-effort, and never block writes.

## The `employed_by` edge

The registry's cross-service link `employed_by` (worker →
organization, with `role` and validity dates) is the identity-level
assertion the family's link-graph aggregates. HCM's Employee record
is the operational layer above it. **Roadmap**: on hire/termination
HCM emits the corresponding `entity_links` write (or a `linked` /
`unlinked` event) so the registry edge tracks employment facts
automatically — until then the two layers are maintained
independently, and this file is the record of that gap.

## Events

Family envelope via the `HCM_EVENT_TRANSPORT` seam (default
`memory`; Postgres outbox rows share the mutation's transaction under
`outbox`). Event kinds are listed in [audit.md](audit.md).
