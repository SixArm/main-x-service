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
| POST | `/api/care-pathways/import` | Native bulk import: multipart JSONL/CSV/TSV upload → `202 {job_id}`; destructive under ABAC |
| GET | `/api/care-pathways/import/{id}` | Import job status + counts + `errors_url` |
| POST | `/api/care-pathways/export` | Native bulk export: `{format, q, limit, offset, masking_profile, include_soft_deleted}` → `202 {job_id}` |
| GET | `/api/care-pathways/export/{id}` | Export job status + `download_url` |
| GET | `/api/care-pathways/bulk-jobs` | Recent bulk jobs (native + FHIR Bulk Data), newest first; filter `?kind=&status=` |
| GET | `/api/care-pathways/review-queue` | Stored duplicate-review queue (written by keyless bulk-import rows, `provenance=import`) |
| POST | `/api/care-pathways/review-queue/{id}/decision` | Decide a pending review item (`confirmed`/`rejected`) |
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
| GET | `/api/instances/{pid}/time-analysis` | Per-instance TBA: lead time, value-adding ratio, coverage, per-stage anchors + adjacent delays, plus conformance to the enrolled template (declared-step-order ratio, per-pair verdicts, skipped/after-closure positions, escalation events) |
| GET | `/api/instances/{pid}/timeline` | The mapped journey as an ordered wall of segments and gaps |
| GET | `/api/care-pathways/{pid}/time-analysis` | Cohort TBA: nearest-rank lead-time percentiles vs. an NHS access standard, optionally anchored (`?from_anchor=&to_anchor=`), plus censoring-aware Kaplan–Meier survival (`?discontinued=event\|censor`), optionally split by a rule (`?contains=&excludes=&compare=`; `?mode=withhold\|remove` on suppression), a CONSORT-style `attrition` trail, and a template-conformance fully-conformant share on every response |
| GET | `/api/care-pathways/{pid}/constraints` | Ranked constraint findings, by recoverable time — same rule-split params and `attrition` trail as time-analysis (`?mode=withhold\|remove` on suppression) |
| GET | `/api/instances/flow` | Queueing-theory flow (Little's Law: λ/μ/ρ/κ/τ) |
| GET | `/api/instances/time-standards` | The NHS access-standard catalogue + segment vocabularies |
| GET | `/api/care-pathways/{pid}/export/event-log` | Bulk `event_log` export (`?format=csv\|jsonl`) — bupaR/PM4Py shape, never a `subject_ref` or a person/actor URN |
| GET | `/api/care-pathways/{pid}/export/journey-features` | Bulk `journey_features` export (`?format=csv\|jsonl`) — one row per instance, LT/VT/PT/%A/%VA/coverage/#HO/per-stage/censored |
| GET | `/api/care-pathways/{pid}/process-map` | Directly-follows process map (`?level=stage\|step&mode=withhold\|remove`) — nodes/edges with counts + median (+p90) gaps, never a discovered model |
| GET | `/api/care-pathways/{pid}/variants` | Journey variants — pathway strings, frequency/coverage Pareto, per-position duration lines (named, defaulted, echoed transform parameters) |
| GET | `/api/care-pathways/{pid}/data-quality` | Journey data-quality and missingness report (`?status=&from_anchor=&to_anchor=`) — eight-code closed vocabulary (no segments / open segment past closure / terminal without clock stop / step done before enrolled / steps out of order / segment clipped by clock / coverage below floor / anchors unreached) plus per-stage missingness + entropy; the report is the finding, it never imputes |

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
never be recovered as `margin − Σ(visible siblings)`. Landed unused;
T-14f (below) is the first real caller.
`event_log`/`journey_features` (T-14a above) are **exempt**,
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
implemented and unit-tested but still has no HTTP surface: T-14f
(below) has since landed the cohort-splitting mechanism this
anticipated, but its own split payload compares sides via the
already-computed cohort/compliance/survival figures, not a log-rank
test between their curves — comparing the two split sides' curves
stays a further, still-open follow-up. Pure logic extends `src/tba.rs`
itself: `Observation`, `close_observation()`, `anchor_observation()`,
`KaplanMeier`/`kaplan_meier()`, `LogRank`/`log_rank()`; HTTP surface:
[`src/controllers/tba.rs`](./src/controllers/tba.rs) (`Survival`,
`survival_analysis()`).

### Rule-based cohort splits (T-14f)

`GET /api/care-pathways/{pid}/time-analysis` and `.../constraints`
gain `?contains=&excludes=&compare=`: a comma-separated list of
`type:value` predicates — `stage`/`step`/`event`/`waste`/`outcome`/
`setting`/`urgency` — every matched instance must satisfy
(`contains`, AND) and none may satisfy (`excludes`, none-of).
`compare=true` reports the complement alongside the matched side, in
the same shape the unsplit response already carries. Absent
`contains`/`excludes` adds no `split` key at all; naming a rule when
the unsplit cohort is already below the suppression floor does the
same, since splitting an already-too-small cohort would disclose
more, not less. Each side's bare `instances` count is always
published — this family never hides the count, only detail — but
`suppression::decide` over a two-cell matched/complement table decides
whether each side's *detail* renders: the first real caller of
[`src/suppression.rs`](./src/suppression.rs)'s own stratified-table
primitive. This closes a genuine gap: a suppressed side's additive
sums (per-stage, per-waste) would otherwise be exactly recoverable as
`unsplit − complement` if the complement's own sums stayed visible, so
a lone small side recruits its sibling even when the sibling alone
clears the floor. **Only `time-analysis` and `constraints` accept
these params** — `process-map` and `variants` already carry their own,
differently-shaped suppression and their own notion of "compare" would
need its own design pass, so wiring the same contract onto them stays
a documented follow-up. `setting:<s>` compares against the pathway's
*lowercased* care setting (`"Outpatient"` → `setting:outpatient`).
Pure logic: new [`src/split.rs`](./src/split.rs) (named `split`, not
`rules` — `crate::instances` is already aliased `rules` throughout the
controller layer): `Predicate`, `Features` (reusing the same
`analytics::SegmentInput`/`StepInput`/`EventInput` rows T-14a's
event-log codec already loads), `Rule`, `partition`, `split_table`;
HTTP surface: [`src/controllers/tba.rs`](./src/controllers/tba.rs)
(`load_features`, `resolve_rule`, `SplitPlan`/`resolve_split`,
`split_payload`/`split_payload_constraints`).

### Cohort attrition record (T-14g)

`time-analysis` and `constraints` both gain `attrition`: an ordered
array of `{label, operation, instances, parent}` steps —
`enrolled_on_pathway`, `status_filter`, `window`, `degenerate_clock`,
`coverage_floor`, `suppression`, plus `rule_filter`/`matched`/
`complement` when a rule-based split (T-14f) is active — explaining
the denominator inside the response rather than in a log. `parent`
(an index into the same array) lets `matched`/`complement` both fork
from the same `rule_filter` parent. `window` and `coverage_floor` are
honestly disclosed rather than invented: no date-window parameter or
coverage-based exclusion exists on these endpoints yet, so both report
zero exclusions. `degenerate_clock` discloses the count of instances
whose clock is not strictly forward **without excluding them** — they
stay in `cohort`/`compliance`/`survival` exactly as they always have,
so this feature changes no existing figure, only what is now visible
about it. Pure logic extends `src/tba.rs` itself: `AttritionStep`,
`ATTRITION_STEP_LABELS`, `ATTRITION_RULE_PARENT`, `attrition_trail()`,
`attrition_rule_branch()`; HTTP surface:
[`src/controllers/tba.rs`](./src/controllers/tba.rs) (`cohort_query()`,
`attrition_counts()`, `build_attrition()`).

### Journey data-quality and missingness report (T-14h)

`GET /api/care-pathways/{pid}/data-quality`
(`?status=&from_anchor=&to_anchor=`): per cohort, the share of
instances with no segments, an open segment past closure, a terminal
status with no clock stop, `done_on` before `enrolled_on`,
out-of-order step completion, segments clipped by the clock, coverage
below the floor, and anchors unreached — eight codes in a closed
vocabulary (BNSSG's `bad_date` 1–5, generalised) — plus per-**stage**
missingness percentage and entropy across instances (per-field is a
documented follow-up: the spec text names no field list). The report
is the finding; it never imputes. Two codes genuinely overlap by
design: an open segment on a terminal instance is also clipped once
its effective end is bounded by "as of now" rather than the clock's
own stop, so `open_segment_past_closure`'s dedicated instance
legitimately also fires `segment_clipped_by_clock`. Pure logic in new
[`src/data_quality.rs`](./src/data_quality.rs) (not a further
`src/tba.rs` extension — detectors, not an elapsed-time computation):
`DQ_CODES` (matching T-14m's `DEFECT_CODES` name-for-name), eight
`has_*` detectors, `binary_entropy_bits`, `build_report`; HTTP surface:
[`src/controllers/data_quality.rs`](./src/controllers/data_quality.rs).
This is the first task to actually exercise T-14m's generator end to
end rather than a hand-built fixture, which found and fixed two real
generator bugs in
[`src/data/journeys.rs`](./src/data/journeys.rs) — see that file's
`widen_clock_stop_past` and `terminal_without_clock_stop` match arm.

### Conformance to the enrolled template (T-14i)

Per instance, the steps copied at enrolment (`instance_steps.position`)
against their completion order (`done_on`): skipped steps, adjacent
declared pairs completed out of order, steps completed after closure,
and `escalation` events; a labelled ratio `pairs_in_order /
declared_pairs` with both numbers; a cohort share of fully-conformant
instances. Against the template only — never a discovered model — and
with no penalty for extra events: a journey may need more than its
template foresaw. `declared_pairs` is a fixed structural count (not
reduced by skips): a pair with either endpoint undone verdicts
`skipped` rather than `inverted`, but still counts against the ratio,
same as a genuine inversion would. New file, pure and DB-free:
[`src/conformance.rs`](./src/conformance.rs) (a genuinely separate
concern from `src/tba.rs` — sequence-order comparison, not an
elapsed-time computation): `StepRecord`, `PairVerdict`
(`in_order`/`inverted`/`skipped`), `conformance()`,
`CohortConformance`/`cohort_conformance()`. Wired into
`GET /api/instances/{pid}/time-analysis` (a new `conformance` key) and
`GET /api/care-pathways/{pid}/time-analysis` (a cohort `conformance`
share, withheld under the identical suppression decision
`survival`/`split` already use) — **not** `constraints`, since
template conformance is not a recoverable-time constraint finding.
Also wires T-14a's own reserved `conformance` journey-feature export
column (a compact scalar summary, not the per-pair breakdown the
dedicated endpoint carries).

### Stalled journeys — aging WIP (T-14j)

`GET /api/instances/stalled?idle_days=N` (default 60, echoed): open
instances whose last recorded activity — latest of segment start/end,
step `done_on`, or event `occurred_at` (a recorded review is an event
too) — is older than N days, sorted most-idle first, each row naming
its last-activity source. Complements `overdue-reviews` (a due date)
with an observed-silence test; retroactive, per IPPA's own
`Process.time_out`: idle-since is the activity time itself, never when
the silence was noticed. Never grouped by actor; a closed instance is
never listed. Extends [`src/instances.rs`](./src/instances.rs) (the
crate's existing pure instance-lifecycle module, not a new sibling
file): `ACTIVITY_SOURCES`, `LastActivity`, `last_activity()`
(falls back to `enrolled_on`), `is_stalled()`. `?idle_days=` falls back
to the default on zero/negative/unparseable input, the same convention
pagination already uses.

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

**Native bulk import/export** (§13 T-10, `src/bulk/`) is implemented:
async, job-based JSONL/CSV/TSV import + export on the loco `worker`
queue, reusing the same `bulk_jobs` table and `ArtifactStore`
(local/S3) the FHIR Bulk Data `$export` operation already used. Stable
key: a deterministic identifier (DOI/Wikidata/`GuidelineId`/URI/UUID) →
the provider-scoped `(provider_id, pathway_code)` pair → explicit
`pid`; a keyless row runs the same search-blocked duplicate detection
`check-duplicates` uses and queues a likely match in a newly added
`review_queue` table (`provenance="import"`), never silently dropping
the row. Export defaults to the masked view
(`crate::privacy::mask_pathway`); the unmasked `full` profile is
destructive under ABAC. Every export is audited, gating delivery.
Known, disclosed limits: `active` (soft-delete state) round-trips on
export but is never applied on import; `include_soft_deleted=true` is
rejected as not-yet-supported; the per-row upsert is not SEC-B3
advisory-lock-protected (matching organization's/case's own BLK-5 gap).

Deferred (see [spec §13](./spec/index.md)): instance-layer
privacy/authz for `pathway_instances.subject_ref`, terminology-server
code-existence checks, and the remaining compliance follow-ups in T-15
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
