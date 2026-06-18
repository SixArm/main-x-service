## 5. Domain Model

Field-by-field reference: [`AGENTS/models.md`](../AGENTS/models.md).

### 5.1 `Worker`

Material aspects:

- **Identity** — UUID `id` + `identifiers: Vec<Identifier>` + optional
  `tax_id` shortcut.
- **Names** — primary `name: HumanName` + `additional_names`
  (former names, name at credential issuance, married / maiden forms).
- **Contact** — `telecom: Vec<ContactPoint>`, `addresses: Vec<Address>`.
- **Identity / credential documents** — passport, driver's licence,
  professional credentials, certificates with type + number +
  issuing authority + issue / expiry dates + verified flag.
- **Emergency contacts** — name, relationship, telecom, address.
- **Demographics** — `gender`, `birth_date`, `marital_status`,
  `multiple_birth`, `deceased`, `photo`.
- **Organisation** — `managing_organization` reference + per-worker
  `links: Vec<WorkerLink>` (`ReplacedBy` / `Replaces` / `Refer` /
  `Seealso`).
- **Audit** — `active`, `created_at`, `updated_at`.

> **Partition rule — within-entity links vs cross-service links.** The
> within-entity `links: Vec<WorkerLink>` (and any within-entity
> `relationships`) reference **other worker records** and ARE a matcher
> signal. Cross-service `entity_links` (§5.4 — `same_identity` to a
> person, `employed_by` to an organization) are **entirely separate**:
> they are NOT stored in `links`/`relationships`, NOT routed to the
> matcher, and NOT a match signal. The matching adapter
> (`src/matching/adapter.rs`) MUST NEVER project `entity_links` into the
> matcher input. A matcher scores two records' *sameness*; "worker
> employed by org" is not sameness evidence. See
> [cross-service linking §7](../../../agents/share/cross-service-linking.md).

### 5.2 Supporting types

`Organization`, `MergeRequest` / `MergeResponse` / `MergeRecord`,
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`.

### 5.3 Invariants

The implementation MUST enforce:

- `name.family` is non-empty.
- `birth_date`, when present, is not in the future.
- An `Identifier` is unique within `(worker_id, identifier_type, system, value)`.
- `IdentityDocument.expiry_date`, when present, is on or after
  `issue_date`. Credentials with no expiry are non-expiring; an
  expiry in the past flags an expired credential but does not refuse
  the record.
- Soft delete is the only delete.

### 5.4 Cross-service entity links (write side)

Worker participates in the federated **cross-service link** graph as a
link **originator**. The full design — `EntityRef` URN format, hybrid
topology, optimistic verification lifecycle, the read-side aggregator,
and the v1 edge-kind registry — is fixed in
[`cross-service linking`](../../../agents/share/cross-service-linking.md);
this section records only what Worker owns and stores locally.

This is **distinct from** the within-entity `links: Vec<WorkerLink>`
(§5.1) and is governed by the partition rule above: cross-service edges
never touch the matcher.

**Edges Worker owns in v1** (outbound; the inverse is the far endpoint's
concern and the aggregator stores both directions):

| Kind | To | Direction | Card. | Temporal | Notes |
|---|---|---|---|---|---|
| `same_identity` | person | symmetric | 1:1 | no | identity backbone — either side (worker or person) may assert; the aggregator canonicalises on the ordered ref pair and dedupes |
| `employed_by` | organization | directed | M:N | yes (`valid_from` / `valid_to`) | carries `role` (job title); inverse `employs` |

Storage is the per-service `entity_links` table (§10.3). The far record
is named by an opaque `EntityRef` URN (`person:<uuid>`,
`organization:<uuid>`) — there is **no** foreign key across services.

**Write semantics — optimistic.** Recording an edge stores the assertion
and emits a `linked` event on the existing envelope (§8.6); it does
**not** call the target service, so write latency and availability are
independent of the person / organization services. Verification status
(`verified` / `unverified` / `dangling`) is **not** a write-side
property — it is the aggregator's view, since only the aggregator sees
both endpoints. Withdrawing an edge is a soft delete that emits
`unlinked`.

