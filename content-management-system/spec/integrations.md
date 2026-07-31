# Integrations

CMS is a consumer of the Main X Index family; it holds **EntityRef
URNs** and never duplicates upstream records.

| Service | Used for | How |
|---|---|---|
| [worker-service](../../worker/worker-service-with-loco/) | authors, editors, reviewers, translators | `owner_ref` / `author_ref` / `reviewer_ref` / `uploaded_by_ref` URNs |
| [organization-service](../../organization/organization-service-with-loco/) | the publishing body of a site | `site.owner_ref` (`organization:` URN) |
| [authentication-service](../../authentication/authentication-service-with-loco/) | SSO + ABAC attrs | offline PASETO via `authentication-verifier`; persona attributes |
| any entity registry | content *about* a registered thing | an `entity_ref` field on a content type (`course:`, `event:`, `place:`, `organization:`, …) — a pointer, never a copy |

Client modules follow the stub-first pattern (patient-flow / CRM
`clients.rs`): read-only, cached, best-effort display-name lookups
that **never block a write**. An author whose display name cannot
be resolved is still recorded by URN.

## Entity references are pointers

A content type may declare an `entity_ref` field scoped to one or
more entity types ([authoring](authoring.md)). The reference is
shape-validated (`EntityRef` URN grammar, from the family
[`entity-ref`](../../link/entity-ref-rust-crate/) contract) and
resolved best-effort for display; delivery emits the URN plus
whatever summary the client could resolve, and marks it explicitly
when it could not.

**CMS is not a link-graph participant in v1.** It writes no
`entity_links` rows and emits no `linked`/`unlinked` events: an
editorial mention of a course is not an assertion about that
course's identity
([cross-service-linking](../../agents/share/cross-service-linking.md)
§7's partition rule, read in the same spirit). Promoting
`entity_ref` fields to real graph edges is a roadmap decision with
a governance question attached, not a free upgrade.

## Storage

Assets use the family **`ArtifactStore`** seam
([bulk-import-export](../../agents/share/bulk-import-export.md)
§12): `local` by default (base-directory confined), `s3` behind the
same optional cargo feature and standard AWS credential chain. No
new storage abstraction, no hand-rolled signing
([assets](assets.md)).

## Outbound webhooks

The **only** extension mechanism ([design](design.md) CMS-D12):
per-site subscriptions to event kinds, delivered to an **HTTPS**
endpoint (**loopback excepted**, the family's standing rule for
server-side fetches) with **no redirects followed** (the family SSRF
rule, [security](../../agents/share/security.md) invariant 7), a
signed body, a timeout, a response-size cap, bounded retries with
backoff, and a delivery log. Typical consumers: a CDN purge on
publish, a static-site rebuild, a search re-index.

**Signing.** HMAC-SHA256 over `{timestamp}.{body}`, keyed by the
subscription's secret, carried in `x-cms-signature` with the
timestamp in `x-cms-timestamp` and the event id in `x-cms-event-id`
(so a receiver can dedupe at-least-once delivery). The timestamp is
**inside** the signed material, so a captured delivery cannot be
replayed later against a receiver that checks freshness. The secret
is returned once at registration and by no read afterwards — it is
stored recoverably, unlike a preview token, for the unavoidable
reason that the receiver must hold the same secret to verify.

**Retries are scheduled, not slept**: a failed attempt is recorded
and picked up by the next dispatch once its backoff has elapsed
(0/30/120/480/1920s, five attempts), so nothing is held in memory
and nothing is lost on restart. A 4xx other than 408/429 is not
retried — the receiver understood and refused. A subscription that
fails repeatedly is deactivated rather than hammered.

**Dispatch requires the durable transport.** Deliveries are driven
from the event record, which is durable only under
`CMS_EVENT_TRANSPORT=outbox`; under the default in-memory transport
dispatch **refuses with a `422` naming the setting** rather than
delivering a subset that disappears on restart.

## Events

Family envelope via the `CMS_EVENT_TRANSPORT` seam (default
`memory`; Postgres outbox rows share the mutation's transaction
under `outbox`). Event kinds are listed in
[domain-model.md](domain-model.md). Webhook deliveries are driven
from the same event record, so an extension cannot observe
something the audit trail does not.
