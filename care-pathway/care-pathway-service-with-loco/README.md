# Care Pathway Service

A registry of **clinical care-pathway** records: CRUD + matching, built
on **loco.rs** and embedding the canonical
[care-pathway-matcher](../care-pathway-matcher-rust-crate).

A *care pathway* (clinical / critical / integrated care pathway) is a
structured, evidence-based, multidisciplinary plan of care for a
specific condition over a defined episode.

- Spec: [spec/index.md](./spec/index.md)
- Agent guide: [AGENTS.md](./AGENTS.md)
- Sibling UI: [care-pathway-front-end-with-svelte](../care-pathway-front-end-with-svelte)

## API

| Method | Path | Purpose |
|---|---|---|
| POST | `/api/care-pathways` | Create |
| GET | `/api/care-pathways` | List |
| GET | `/api/care-pathways/{pid}` | Fetch |
| PUT | `/api/care-pathways/{pid}` | Update |
| DELETE | `/api/care-pathways/{pid}` | Soft-delete |
| GET | `/api/care-pathways/search?q=` | Tantivy full-text search (`?fuzzy=true`, `?phonetic=true`); paginated |
| POST | `/api/care-pathways/match` | Rank `{query, candidates}` |
| POST | `/api/care-pathways/check-duplicates` | Match query vs stored pathways |
| POST | `/api/care-pathways/merge` | Merge a duplicate into a survivor |
| GET | `/api/care-pathways/merges/recent` | Merge-history records |
| GET | `/api/care-pathways/audit/recent` · `/{pid}/audit` | Audit-log query |
| GET | `/api/care-pathways/events/recent` | In-memory event stream |
| GET | `/api/care-pathways/whoami` | Verified bearer-token claims (`401` without one) |
| GET | `/api-docs/openapi.json` · `/swagger-ui` | OpenAPI 3 doc + Swagger UI |
| GET | `/metrics.prom` | Prometheus metrics (root path, public under auth enforcement) |

`GET /api/care-pathways` and the search endpoint above both take
`?limit=`/`?offset=` and report `X-Total-Count`/`X-Limit`/`X-Offset`
response headers (defaults reproduce the old hard caps of 100/50).

### Time-based analysis ([spec §6.18](./spec/index.md))

| Method | Path | Purpose |
|---|---|---|
| POST/GET | `/api/instances/{pid}/segments` | Record / list a journey's VA / NNVA / UNVA segments |
| POST | `/api/instances/{pid}/segments/{seg}/close` | Close a running segment |
| POST | `/api/instances/{pid}/clock` | Set the pathway clock `start`/`stop` (no `pause`, by design) |
| GET | `/api/instances/{pid}/time-analysis` | Per-instance TBA: lead time, value-adding ratio, coverage, per-stage anchors + adjacent delays |
| GET | `/api/instances/{pid}/timeline` | The mapped journey as an ordered wall of segments and gaps |
| GET | `/api/care-pathways/{pid}/time-analysis` | Cohort TBA: nearest-rank lead-time percentiles vs. an NHS access standard, optionally anchored (`?from_anchor=&to_anchor=`), plus censoring-aware Kaplan–Meier survival (`?discontinued=event\|censor`; `?mode=withhold\|remove` on suppression) |
| GET | `/api/care-pathways/{pid}/constraints` | Ranked constraint findings, by recoverable time (`?mode=withhold\|remove` on suppression) |
| GET | `/api/instances/flow` | Queueing-theory flow (Little's Law: λ/μ/ρ/κ/τ) |
| GET | `/api/instances/time-standards` | The NHS access-standard catalogue + segment vocabularies |
| GET | `/api/care-pathways/{pid}/export/event-log` | Bulk `event_log` export (`?format=csv\|jsonl`) — bupaR/PM4Py shape, never a `subject_ref` or a person/actor URN |
| GET | `/api/care-pathways/{pid}/export/journey-features` | Bulk `journey_features` export (`?format=csv\|jsonl`) — one row per instance, LT/VT/PT/%A/%VA/coverage/#HO/per-stage/censored |
| GET | `/api/care-pathways/{pid}/process-map` | Directly-follows process map (`?level=stage\|step&mode=withhold\|remove`) — nodes/edges with counts + median (+p90) gaps, never a discovered model |
| GET | `/api/care-pathways/{pid}/variants` | Journey variants — pathway strings, frequency/coverage Pareto, per-position duration lines (named, defaulted, echoed transform parameters) |

Every figure is derived on read — nothing is stored — and the
denominator is always elapsed calendar time, never the sum of recorded
activity. Full design:
[`agents/share/time-based-analysis.md`](../../agents/share/time-based-analysis.md),
[`../spec/time-based-analysis.md`](../spec/time-based-analysis.md).

### Seeded synthetic journey cohorts (T-14m)

```sh
cargo loco task journeys:seed pathway:<pid>                       # 10 clean instances, seed 42
cargo loco task journeys:seed pathway:<pid> n:50 seed:7 open_share:0.3
cargo loco task journeys:seed pathway:<pid> defects:all            # one instance per defect code
cargo loco task journeys:seed pathway:<pid> defects:no_segments,steps_out_of_order
```

Deterministic and **synthetic only** — never real data, never derived
from real data. Every generated `subject_ref` / care-team `member_ref`
/ `location_ref` carries a fixed, recognizably-fake byte prefix
(`facade50`/`51`/`52` in hex) rather than an ordinary-looking random
UUID, so this data can never be mistaken for a real patient's. The
same seed always produces the same cohort. Pure generator:
[`src/data/journeys.rs`](./src/data/journeys.rs); task:
[`src/tasks/journeys_seed.rs`](./src/tasks/journeys_seed.rs).

### Disclosure control (T-14k)

A shared, deployment-configurable cell-count floor
(`CARE_PATHWAY_MIN_CELL_COUNT`, default 5, **raisable only** — a lower
or garbage value falls back to the default rather than weakening
protection) governs both cohort endpoints above: `?mode=withhold`
(default: `null` + a reason) or `?mode=remove` (drop the key
entirely). Beyond that scalar case,
[`src/suppression.rs`](./src/suppression.rs) also provides a generic
stratified-table primitive (`Table`/`Partition`/`decide`/`render`)
with **secondary suppression** — when a row, a column, or any declared
group summing to a published margin is left with exactly one
suppressed cell, one more is suppressed too, so a withheld cell can
never be recovered as `margin − Σ(visible siblings)`. No 2-D
breakdown exists in this crate yet (T-14f); this primitive is ready
for it. `event_log`/`journey_features` (T-14a above) are **exempt**,
not suppressed — they are patient-level rows, gated by access control
and audited, per the family's bulk-export contract.

### Directly-follows process map (T-14b)

`?level=stage` (default) derives nodes/edges from segments in time
order; `?level=step` from completed steps in `done_on` order (a
same-day pair is a 0-day edge — `done_on` is a date). Both are
bookended with explicit `start`/`end` pseudo-nodes and keep self-loops
rather than collapsing them. The gap on an edge is measured from the
end of the first activity to the start of the second, not
start-to-start, so an activity's own duration is never counted as
transition time. Suppression (above) applies **per node/edge**: an
activity visited by fewer than the floor's worth of instances stays
withheld even inside an otherwise-large cohort. Never a discovered
model — pure graph-building lives alongside the T-14a codecs in
[`src/analytics.rs`](./src/analytics.rs); HTTP surface + suppression
rendering: [`src/controllers/tba.rs`](./src/controllers/tba.rs).

### Journey variants — pathway strings (T-14c)

Each instance's segment sequence runs through a named, defaulted,
echoed parameter chain — `min_segment_days` (drop short segments),
`collapse_gap_days` (merge small same-stage gaps), `combination_window_days`
(an overlap at least this long becomes a canonical alphabetical `a+b`
step, converging to `a+b+c` under a three-way overlap; a shorter
overlap is a handoff to the incoming stage), `min_post_combination_days`
(drop post-combination stubs — a combination era itself is exempt),
`filter` (`first`/`changes`/`all`), `max_path_length` — into a
compact pathway string (`referral-diagnostics-treatment+follow_up-…`).
The cohort is then reported as a frequency/coverage Pareto (variants
below the floor fold into `suppressed_instances`, never listed
individually; visible shares are renormalised to sum to `1.0`) plus
per-position ("line") duration quantiles with an `overall` pseudo-line.
Nothing is stored. Pure pipeline:
[`src/variants.rs`](./src/variants.rs); HTTP surface:
[`src/controllers/tba.rs`](./src/controllers/tba.rs).

### Stage anchors, delay decomposition, and anchored standards (T-14d)

Every instance's per-instance time-analysis now also carries `anchors`
(each stage's first `started_at`, `null` if never reached) and `delays`
(adjacent-pair differences, clamped at zero, each with a `reason` when
either side is unreached). Cohort compliance
(`/api/care-pathways/{pid}/time-analysis`) can score an interval
between two named stages instead of the whole clock —
`?from_anchor=&to_anchor=` — with an unreached pair counted as
`compliance.unreached`, a **third verdict**: never compliant, never a
breach, always disclosed. Precedence, most to least specific: an
explicit query pair always wins, even over a standard's own declared
anchor; naming neither falls through to the requested standard's own
anchor pair if it declares one — `cancer_fds_28_days` is the one
catalogue entry that does (referral → diagnostics; every other
standard stays whole-clock, unchanged); naming neither the query nor
finding one on the standard leaves whole-clock behaviour untouched. An
explicit pair that fails validation (only one side given, or a name
that is not a recognised stage) never silently falls back to a
standard's own anchor — it always reverts to whole-clock, with
`compliance.anchor_note` disclosing why rather than approximating.
T-14a's `journey_features` export column `anchors_delays` is wired in
this change too (a per-instance JSON cell, no cohort context needed).
Pure logic extends `src/tba.rs` itself — `StageAnchor`, `anchors()`,
`Delay`, `delays()`, `anchor_interval()`, `anchored_compliance()` —
rather than a new sibling module, since it extends the
`InstanceAnalysis`/`Standard`/`Compliance` types already there; HTTP
surface + precedence logic:
[`src/controllers/tba.rs`](./src/controllers/tba.rs).

### Censoring-aware cohort statistics (T-14e)

`?status=all` mixes a closed instance's finished lead time with an
open one's still-running lead time as if they were the same kind of
number, which understates the eventual distribution. Cohort
time-analysis now also reports `survival.time_to_close`: a
Kaplan–Meier estimate treating every open instance as right-censored
at now, with `median_ms`/`p90_ms` read off the curve (`null` with
reason `curve_did_not_reach` when the curve never drops that far —
never a fabricated figure) and the numbers of events and censored
instances disclosed alongside. `?discontinued=event` (default) or
`?discontinued=censor` selects whether a discontinued closure counts
as the event or a censoring, echoed as `survival.discontinued`.
Naming a recognised `from_anchor`/`to_anchor` pair — the same one
`compliance` uses (T-14d) — adds `survival.time_to_anchor`: a second
Kaplan–Meier curve for the interval between those two stages, treating
an instance that reached `from_anchor` but never `to_anchor` as
censored at the clock's own last-observed instant; an instance that
never reached `from_anchor` at all is excluded outright, since there
is no time zero to measure it from. The whole `survival` block is
withheld under the identical suppression decision as the percentile
detail — a curve over a handful of instances is exactly as disclosive.
A two-sample log-rank test (`tba::log_rank`, Mantel–Haenszel form) is
implemented and unit-tested but has no HTTP surface yet — there is no
cohort-splitting mechanism in this crate to hand it two sides (T-14f);
it is ready for that task to call, the same "ready for it, not wired
to it" posture [`src/suppression.rs`](./src/suppression.rs) already
documents for its own still-unused 2-D breakdown primitive. Pure logic
extends `src/tba.rs` itself: `Observation`, `close_observation()`,
`anchor_observation()`, `KaplanMeier`/`kaplan_meier()`, `LogRank`/
`log_rank()`; HTTP surface: [`src/controllers/tba.rs`](./src/controllers/tba.rs)
(`Survival`, `survival_analysis()`).

### Cross-service journey links ([spec §6.19](./spec/index.md))

| Method | Path | Purpose |
|---|---|---|
| POST/GET | `/api/instances/{pid}/links` | Assert / list a journey instance's outbound `continues_as` edges |
| DELETE | `/api/instances/{pid}/links/{id}` | Withdraw an edge |
| GET | `/api/instances/links` | Bulk reconciliation pull (privileged; `?since=` for an incremental pull) |
| GET | `/api/instances/{pid}/journey` | The whole journey, stitched across every `continues_as` link |

`continues_as` links a pathway **instance** (an enrolment, never the
template) into its next episode — another instance, a
`patient_flow_stay`, or a `case`. High-sensitivity governance alongside
`subject_of`: authorised at the read-the-journey level, every write
audited, the bulk pull gated as a privileged read. **A policy denial on
any of these endpoints is reported as `404`, not `403`** — see
[spec §6.20](./spec/index.md) for why a `403` here would itself be a
disclosure.

### Compliance ([spec §12](./spec/index.md))

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/compliance` | Posture: build provenance, IEC 62304 safety class, live controls, data-protection declarations, and per-framework **not-claimed** lines |
| GET | `/api/compliance/sbom` | CycloneDX 1.5 SBOM + SOUP register (from this binary's own `Cargo.lock`) |
| GET | `/api/compliance/audit/verify` | Verify the tamper-evident audit hash chain (HIPAA §164.312(c)) |
| GET | `/api/compliance/records/verify` | Verify row-level record integrity — detects an entity row edited outside the service |
| GET | `/api/compliance/checkpoint` | Take a MAC'd external-witness statement of the audit chain's head/anchor/row-count |
| POST | `/api/compliance/checkpoint/verify` | Verify the chain still honours a previously-taken checkpoint — catches wholesale tail deletion, which the chain alone cannot see |
| GET | `/api/care-pathways/{pid}/audit/disclosures` | Accounting of disclosures (HIPAA §164.528) |
| POST | `/api/care-pathways/{pid}/erase` | GDPR Art. 17 erasure — **irreversible**, destructive under ABAC |

The audit chain and row hashes are written by default; the **checkpoint
and MAC controls above are inert until configured** —
`CARE_PATHWAY_INTEGRITY_MAC_KEY` (or `_KEY_FILE`) turns on keyed MACs,
and a checkpoint only protects what an operator actually stores off-box.
See
[`agents/share/runbooks/integrity-activation.md`](../../agents/share/runbooks/integrity-activation.md)
for the activation order; `cargo loco task integrity_key` generates and
checks the key without ever logging it.

### FHIR R5 ([spec §12.3](./spec/index.md))

| Method | Path | Purpose |
|---|---|---|
| GET/POST/PUT/DELETE | `/fhir/PlanDefinition{,/{id}}` | Resource interactions + search |
| POST | `/fhir/PlanDefinition/$validate` | Profile + terminology validation, persists nothing |
| GET | `/fhir/metadata` | `CapabilityStatement` (public) |
| GET | `/fhir/.well-known/smart-configuration` | SMART discovery — only when a SMART authorization server is configured, else `404` (public) |
| GET | `/fhir/$export` → `/$export-status/{id}` → `/$export-file/{id}/{file}` | Bulk Data Access (NDJSON); `DELETE /$export-status/{id}` cancels |

> **Not claimed:** ONC certification (this serves FHIR R5; certification
> targets R4 + US Core, and `PlanDefinition` has no US Core profile),
> SMART App Launch itself (the credential is PASETO, not OAuth 2.0), or
> medical-device qualification. See [spec §12.5](./spec/index.md).

See [AGENTS.md](./AGENTS.md) and [spec §6](./spec/index.md) for the full
route contract.

The body for a care pathway **is** the `care_pathway_matcher::CarePathway`
shape (name, pathway code + provider, care setting, target condition
codes (ICD/SNOMED), interventions, keywords, identifiers, sameAs).

## Quick start

Requires PostgreSQL.

```bash
export DATABASE_URL=postgres://loco:loco@localhost:5432/care_pathway_service_development
cargo loco start        # migrations auto-run in development

# Create
curl -s localhost:5150/api/care-pathways -H 'content-type: application/json' \
  -d '{"name":"Acute Stroke Care Pathway","condition_codes":[{"system":"Icd10","code":"I63"}]}'

# Name search
curl -s 'localhost:5150/api/care-pathways/search?q=stroke'

# Match an explicit query against candidates (no persistence)
curl -s localhost:5150/api/care-pathways/match -H 'content-type: application/json' \
  -d '{"query":{"name":"Acute Stroke Care Pathway"},"candidates":[{"name":"Stroke Care Pathway"}]}'

# Check for duplicates of a query against stored pathways
curl -s localhost:5150/api/care-pathways/check-duplicates -H 'content-type: application/json' \
  -d '{"name":"Acute Stroke Care Pathway"}'

# Merge a duplicate into a survivor (the survivor is `main_pid`)
curl -s localhost:5150/api/care-pathways/merge -H 'content-type: application/json' \
  -d '{"main_pid":"<survivor-uuid>","duplicate_pid":"<duplicate-uuid>"}'

# Authenticated request: present a short-lived bearer PASETO minted by the
# auth-service. /whoami echoes the verified claims (401 without a valid
# token). With blanket enforcement on (CARE_PATHWAY_REQUIRE_AUTH=1) every
# /api/* route needs the same header.
curl -s localhost:5150/api/care-pathways/whoami \
  -H 'authorization: Bearer <paseto-from-authentication-service>'
```

A validation failure returns `422` with **every** problem in one body,
e.g. a blank `name` plus a malformed condition code plus a bad UUID
identifier come back together:

```bash
curl -s localhost:5150/api/care-pathways -H 'content-type: application/json' \
  -d '{"name":"  ","condition_codes":[{"system":"Icd10","code":"not-a-code"}],
       "identifiers":[{"scheme":"Uuid","value":"not-a-uuid"}]}'
# → 422 {"error":"validation","description":"name must not be blank; \
#        condition_codes[0] … ; identifiers[0] …"}
```

## Testing

```bash
cargo test                   # DB-free: matcher embedding, JSON round-trip,
                             # validation pins, the audit-chain and FHIR
                             # profile/terminology cores, and the
                             # requirement→test traceability check
cargo test -- --ignored      # request-level tests (need Postgres DATABASE_URL)
cargo clippy --all-targets
```

Two DB-free checks fail the build on purpose when the repository drifts:

- **`every_direct_dependency_is_annotated`** — adding a dependency
  without an entry in [`compliance/soup.tsv`](./compliance/soup.tsv)
  fails (IEC 62304 §8.1.2).
- **`every_named_test_exists`** — renaming or deleting a test that
  [`compliance/traceability.tsv`](./compliance/traceability.tsv) cites
  fails, so a requirement cannot silently lose its verification.

The audit chain's key property — that a digest computed before an
`INSERT` still matches after a Postgres JSONB round-trip — can only be
checked against a real database, so it lives in the `--ignored` suite
(`chain_survives_a_jsonb_round_trip`,
`tampering_with_a_row_breaks_verification`).

```bash
scripts/sbom.sh                        # CycloneDX SBOM + SOUP + cargo-deny
scripts/build-reproducible.sh --verify # build twice, compare hashes
cargo run --bin sbom                   # the SBOM alone, to stdout
```

Validation failures return `422 Unprocessable Entity` — the family
convention — for a blank `name`, a malformed `condition_codes` entry
(ICD-10 / ICD-11 / SNOMED CT Verhoeff), a malformed `identifiers` entry
(UUID / DOI shapes), or an `in_language` tag that is not valid BCP-47.
All problems are reported together in one body, on both create and
update. See [spec §6.1](./spec/index.md).

## Status

Implemented: CRUD + Tantivy full-text/fuzzy/phonetic search (with
search-blocked dedup candidates) + matching + record merge + field
masking + audited GDPR export +
audit log + all three phases of the durable event bus (in-memory
stream, transactional outbox, and the `FluvioSink` real-broker sink
behind this crate's `fluvio` feature) +
OpenAPI/Swagger + Prometheus metrics + offline PASETO v4.public
verification + blanket `/api/*` auth enforcement and ABAC authorization
(off by default, gated by `CARE_PATHWAY_REQUIRE_AUTH`) + rich payload
validation (ICD/SNOMED/UUID/DOI/BCP-47) + `?limit=`/`?offset=`
pagination on list and search + boot-time published-key fetch over HTTP
plus a background refresh loop (`CARE_PATHWAY_PASETO_KEYS_URL` /
`_REFRESH_SECS`) and ABAC-policy hot-reload
(`CARE_PATHWAY_ABAC_POLICY_FILE`), all without a restart; **time-based
analysis** (segment/clock recording, per-instance and cohort TBA,
ranked constraints, Little's-Law flow, a default-off Prometheus
flow-gauge family) and **cross-service journey links** (`continues_as`
write side + bulk reconciliation pull + the stitched
`GET /api/instances/{pid}/journey` read, each leg fetched under the
caller's own credential) — see the two tables above.

**Compliance (2026-07-25 through 2026-08-04)** — this crate is the
family's reference implementation of the four control-driving
frameworks in
[`agents/share/compliance-for-healthcare.md`](../../agents/share/compliance-for-healthcare.md)
§2: a tamper-evident audit hash chain + read/disclosure auditing +
keyed HMAC integrity MACs with per-domain HKDF subkeys + MAC'd
external-witness chain checkpoints (closing the tail-truncation gap the
hash chain alone cannot see) + row-level record content hashing
(HIPAA), Art. 17 erasure that survives that chain plus residency and
lawful-basis declarations (GDPR / EU EHDS), FHIR profile + terminology
validation with `$validate`, SMART discovery and a durable (worker +
artifact-store) Bulk Data `$export`
(ONC / HTI), and a SOUP register, CycloneDX SBOM, machine-checked
requirement→test traceability and reproducible-build tooling
(IEC 62304 / SaMD). Read-auditing is **default off**
(`CARE_PATHWAY_AUDIT_READS`), and the MAC/checkpoint controls are
**inert until a key is configured**; for a compliance deployment also
set `CARE_PATHWAY_REQUIRE_AUTH`, `CARE_PATHWAY_EVENT_TRANSPORT=outbox`,
and `CARE_PATHWAY_INTEGRITY_MAC_KEY` — see
[`agents/share/runbooks/integrity-activation.md`](../../agents/share/runbooks/integrity-activation.md)
for the order. See [spec §12](./spec/index.md) — including §12.5, which
states the limits plainly.

Deferred (see [spec §13](./spec/index.md)): instance-layer
privacy/authz for `pathway_instances.subject_ref`, terminology-server
code-existence checks, the native (non-FHIR)
bulk import/export API, and the remaining compliance follow-ups in T-15
(lifting `compliance/` to the repo root once a second crate adopts it,
an Inferno-style `/fhir` conformance run). Token issuance is
provided by the central
[authentication-service](../../authentication/authentication-service-with-loco).

Auth pivot done in this crate: the family moved from RS256 JWT + JWKS to
cookie sessions + short-lived PASETO v4.public verified offline against a
published Ed25519 key (RS256/JWKS decommissioned); the
`CARE_PATHWAY_REQUIRE_AUTH` flag and enforcement semantics are unchanged,
only the credential changed. See
[agents/share/authentication-sessions.md](../../agents/share/authentication-sessions.md)
(source of truth); `src/auth.rs` verifies PASETO via the
`authentication-verifier` crate.

## License

Dual-licensed under MIT OR Apache-2.0 OR BSD-3-Clause OR GPL-2.0-only OR
GPL-3.0-only.
