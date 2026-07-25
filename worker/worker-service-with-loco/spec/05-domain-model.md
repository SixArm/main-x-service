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
`ReviewQueueItem`, `BatchDeduplicationRequest` / `Response`, `Consent`,
`Assessment` (§5.5).

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

### 5.5 `Assessment` — aptitude / personality / psychometric / selection

A workforce **assessment** is one administration of one instrument (a
named test) to one worker: `src/models/assessment.rs`, persisted in
`worker_assessments` (§10.5), served under
`/api/workers/{id}/assessments` (§9.2).

**Categories and the scales they measure.** `AssessmentCategory` is the
family of test; `AssessmentScale` is the dimension one result reports:

| Category | Measures | Scales |
|---|---|---|
| `aptitude` | how a person performs at tasks and reacts to situations | `numerical_reasoning`, `verbal_reasoning`, `problem_solving`, `logical_thinking` |
| `personality` | behavioural style and working qualities | `work_style`, `team_compatibility`, `introversion_extraversion` |
| `psychometric` | **spans aptitude and personality** | `behavioural_style`, `emotional_intelligence`, `cognitive_ability` — **plus** every aptitude and personality scale |
| `selection` | suitability for a role during hiring | `job_simulation`, `skills_assessment`, `judgement_test` |

`AssessmentCategory::permits` is the rule: a category always accepts its
own scales, and `psychometric` additionally accepts aptitude and
personality scales (a psychometric test covers both by definition). A
result on any other cross-category scale is a **`422`**, not a silently
mis-filed row — the profile view (§9.2) is only honest if the category
of a reading is trustworthy.

**Record shape.** `id`, `worker_id`, `category`, `instrument` (required
— results are uninterpretable without knowing which test produced
them), optional `provider`, `status`, optional `administered_on` /
`expires_on` / `administered_by` / `notes`, and
`results: Vec<AssessmentResult>`.

**`AssessmentResult`** carries `scale` plus every score field
optionally: `raw_score`, `max_score`, `percentile` (`[0, 100]`), `band`,
`narrative`. Instruments differ — some report a raw score out of a
maximum, some a norm-referenced percentile, some only a qualitative
profile with no score at all. `effective_band()` reads the explicit
`band` and otherwise derives one from the percentile.

**`ScoreBand`** is the coarse, shareable reading of a percentile, on the
conventional norm-referenced split: `low` (< 10), `below_average`
(< 30), `average` (< 70), `above_average` (< 90), `high` (≥ 90).

**Lifecycle.** `scheduled → in_progress → completed → expired`, with
`cancelled` reachable from any open state and a direct
`scheduled → completed` for a test recorded after the fact. `expired`
and `cancelled` are terminal. An illegal move is a `422` naming the
current state.

**Invariants.**

- `instrument` is required and non-blank; every text field is
  length-capped and `results` cardinality-capped (SEC-M1).
- A scale appears at most once per assessment.
- `percentile ∈ [0, 100]`; `max_score > 0`; `0 ≤ raw_score ≤ max_score`.
- `expires_on` is not before `administered_on`.
- A `completed` assessment carries an `administered_on` **and** at least
  one result — otherwise "completed" would assert a scoring that never
  happened.
- Results count as **current** on a date iff the assessment is
  `completed` and either has no expiry or has not passed it
  (`Assessment::is_valid_on`).
- Deletion is soft.

**Sensitivity.** Assessment results are sensitive personal data — they
profile cognition and behaviour. `Assessment::masked` is the redacted
projection returned under the ABAC `mask` obligation: the scale and the
interpreted band survive; raw scores, percentiles, narratives, and
operator notes do not. It is applied on **every** read path (§9.2).

> **Not a matcher signal.** Assessments are operational records about a
> worker, never evidence that two records are the same worker. The
> matching adapter MUST NEVER project `worker_assessments` into matcher
> input — the same partition rule that governs `entity_links` (§5.1).
