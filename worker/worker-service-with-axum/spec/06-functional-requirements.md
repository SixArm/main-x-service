## 6. Functional Requirements

### 6.1 Identity management

- Create / read / update / soft-delete worker records.
- Multiple professional identifiers per worker.
- Credential documents with expiry tracking.
- Multiple addresses, telecom contacts, emergency contacts.
- Automatic event publish on every CRUD.

### 6.2 Matching

Algorithm reference: [`agents/matching.md`](../agents/matching.md).

| Strategy | Output | Use |
|---|---|---|
| Probabilistic | The `worker-matcher` engine's score 0.00–1.00, renormalised over the components both workers hold | Fuzzy input; the service's matcher |
| Deterministic | Rule-based; short-circuits on identifier (NPI, DEA, employee #), tax-ID, or document exact match | Hard guarantees |

**The service scores through the matcher (since 2026-11-09, §13 T-23).**
`ProbabilisticMatcher` maps both workers through
`adapter::to_matcher_worker` and takes the engine's `score`, `is_match`,
confidence band and breakdown, at the service's `threshold_score`. A
component either worker lacks is left out of the score, not counted as
`0`: two workers identical in what they hold score `1.0`. The service's
own `ProbabilisticScorer` (the weights below) counted a missing
component as `0`, so identical workers with no shared identifier scored
`0.75`, under the threshold; it stays as a library type but scores
nothing on the service's paths. `DeterministicMatcher` keeps its own
rule-based score and takes the engine's breakdown and evidence.

**Match evidence** ([`agents/share/match-evidence.md`](../../../agents/share/match-evidence.md)).
Every scored pair also carries the engine's evidence: Fellegi–Sunter
`weight_bits`, `fields_compared`, `contradictions`, the per-term
`components`, and a `strength` — `proven` (a shared unique identifier,
or bits over the engine's `proof_bits`, default 30), `consistent` (a
match on score, short of proof), `contradicted` (two different values
under one unique scheme), or `non_match`. A score says how alike the
data is; only `proven` says the pair is one worker. Evidence holds
component and scheme names and bits, never a value, so it travels on
every path the score does, masked or not.

**Value frequencies (§7 of that document).** Given name, family name and
date of birth (the matcher's `FREQUENCY_COMPONENTS`) weigh by the agreed
value's own share of workers, so "Smith" or a placeholder birth date
proves less than a rare value. The service counts the shares
(`matching::frequencies`): each active worker through the adapter and
the matcher's `frequency_key`, divided by the population, into a
`FrequencyTable`. Below `MIN_FREQUENCY_POPULATION` (1 000 workers) it
supplies none, since a share from a small population is noise. The
batch scan counts the workers it loaded; the request paths (create-time
check, `check-duplicates`, `match`) read a cached table rebuilt in a
background task when older than ten minutes, from a scan that pages
the repository and stops at 200 000 workers (above that the table is a
sample in id order). No request waits on a rebuild. Frequencies change
the evidence only, never the score.

The service's own scorer's component weights (library only, see above):

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.30 | Jaro-Winkler + Levenshtein + Soundex bonus |
| Birth date | 0.25 | Date proximity |
| Gender | 0.10 | Exact / unknown handling |
| Address | 0.10 | Weighted postal / city / state / street |
| Identifier | 0.10 | Type + system + value match |
| Tax ID | 0.10 | Exact match (deterministic short-circuit to 1.0) |
| Document | 0.05 | Type + number match |

`ProbabilisticMatcher::classify_match` bands (configurable thresholds):

| Quality | Score |
|---|---|
| Definite | ≥ 0.95 |
| Probable | ≥ 0.85 |
| Possible | ≥ 0.50 |
| Unlikely | < 0.50 |

#### Interoperability with `worker-matcher`

The service embeds the sibling `worker-matcher` crate (declared in
`Cargo.toml`) and re-exports it from `src/matching/mod.rs` as
`matcher_lib`. The matcher crate is the **canonical reference
algorithm** — it carries 40+ national-identifier parsers (UK NHS,
FR NIR, US SSN, BR CPF, IN Aadhaar, …), passport-book matching,
blood-type signals, nickname tables, and three tuned config presets
(`strict` / `default` / `lenient`) that the in-service matcher does
not duplicate.

Bridge: [`src/matching/adapter.rs`](../src/matching/adapter.rs) exposes
`to_matcher_worker(&service::Worker) -> worker_matcher::Worker`. The
projection lifts the service's FHIR-shaped record into the matcher's
flat builder shape using the same routing rules as the person bridge
(name flattening, telecom sampling by `ContactPointSystem`, address
field renames, identifier routing by `system` URI with type-based
fallbacks, passport documents → `passport_books`). Service-only
fields (`id`, `active`, `worker_type`, `deceased_datetime`,
`managing_organization`, `links`, `created_at`, …) are dropped.

Identifier `system`-URI routing recognises a distinctive token per
national scheme and fills the matching matcher slot. The covered set
is UK NHS / US SSN / BR CPF / FR NIR / ES TSI / IN Aadhaar / JP My
Number / MX CURP / SE Personnummer / DE KVNR / NL BSN / NZ NHI / AU+IE
IHI (length-disambiguated), plus PL PESEL / PL NIP / RO CNP / UK NINO /
UK CHI / UK H&C / IT Codice Fiscale / ES DNI / PT NIF / FI HETU / DK
CPR / HR OIB / NO FNR / BG EGN / SI EMŠO / CN RRN / ZA ID / BE NN.
Tokens are chosen not to collide (e.g. `nino` never overlaps `nir`;
short abbreviations such as `chi`/`hc` require a longer qualifier).
Every covered slot carries its own weight + breakdown score +
deterministic short-circuit in the matcher (its spec §12), so a shared
value drives a match rather than silently falling through. An
unrecognised URI falls back to the `IdentifierType` enum
(`TAX`/`SSN` → US SSN; `ODS`/`MRN`/`DL`/`NPI`/`Other` unmapped).
Routing is pinned by the adapter's own unit tests and by
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs)
(`shared_pesel_drives_match_via_pl_pesel_slot`,
`shared_nhs_number_drives_match_via_uk_nhs_number_slot`).

The matcher's `uk_nhs_number` slot is the per-worker equivalent of
the person matcher's `uk_nhs_number` (both crates settled on the
shorter method name once published to crates.io).

**ODS routing decision (entity task T-7, 2026-06-13):** the service's
worker-specific `IdentifierType::ODS` (NHS Organisation Data Service
code) has **no suitable matcher slot and is deliberately left
unmapped**. Grounding:

- Every matcher identifier slot is a *person-level* national scheme
  (42 schemes: `uk_nhs_number`, `us_ssn`, `fr_nir`, …). An ODS code
  identifies an *organisation or site* (RC1/RC2 record classes), not
  the worker; every worker at the same practice shares it.
- Routing it into any person slot would make the deterministic
  exact-match short-circuit declare colleagues to be the same person
  — a catastrophic false positive.
- The matcher's `local_id` slot is deliberately never scored (matcher
  spec resolved OQ-2: organisation-issued values collide), so routing
  ODS there would be a silent no-op.

The fall-through is pinned by two bridge tests in
[`tests/duplicate_detection.rs`](../tests/duplicate_detection.rs):
`ods_organisation_code_falls_through_unmapped` (matching continues on
remaining fields) and
`shared_ods_code_does_not_make_different_workers_match` (a shared ODS
code never short-circuits two different workers to a match). Revisit
only if the matcher crate ever adds an organisation-affiliation
signal (its spec §23). See
[`agents/matching.md`](../agents/matching.md) for the in-service
algorithm and the matcher crate's
[`spec.md §12`](../../worker-matcher-rust-crate/spec/index.md) for the
canonical algorithm.

### 6.3 Search

Tantivy across 11 indexed fields (name, identifiers — including NPI
/ DEA — DOB year, addresses). Full-text + fuzzy + phonetic, boolean
syntax, pagination (`offset` + `limit`), optional sensitive-field
masking. Index stays synchronised with database writes.

### 6.4 Duplicate detection and merging

- Real-time `409 Conflict` on `POST /api/workers`.
- Explicit `POST /api/workers/check-duplicates`.
- Batch `POST /api/workers/deduplicate` with configurable thresholds.
- Every candidate on these three paths carries the matcher's `quality`
  band (`certain` ≥ 0.90, `probable` ≥ 0.75, else `possible`), its
  breakdown (the components both workers hold) and its `evidence`
  (§6.2).
- **Auto-merge only on proof.** The batch scan marks a pair `AutoMerged`
  only when its score clears `auto_merge_threshold` **and** its evidence
  is `proven`. A `consistent` or `contradicted` pair is queued `Pending`
  whatever its score (match-evidence.md §4). `AutoMerged` is a marker:
  the service performs no automatic merge itself.
- Review queue (`Pending` / `Confirmed` / `Rejected` / `AutoMerged`),
  each item storing the pair's evidence (`review_queue.evidence`, §10).
- Merge transfers identifiers (credentials!), names, addresses,
  contacts, documents, tax-ID, emergency contacts; appends the
  duplicate's primary name as a "former" alias on the survivor;
  adds a `Replaces` link; soft-deletes the duplicate; records a JSON
  snapshot; emits a `Merged` event.

### 6.5 Validation and normalisation

Required-field enforcement (family + given name), future-date guard
on birth date, tax-ID format, email regex, phone digit count, address
completeness, document number required + expiry guard,
emergency-contact name + relationship required. Phone normalised
E.164-like; addresses standardised. Failed validation → `422`.

### 6.6 Privacy

Per-field masking, GDPR Article 15 export at
`GET /api/workers/{id}/export`, masked view at
`GET /api/workers/{id}/masked`, consent model with type + status +
dates, `has_active_consent()` utility. Sensitive fields specific to
workforce data (SSN, tax ID, DEA, home address) are masked by default
in the masked view. See
[`agents/share/privacy.md`](../../../agents/share/privacy.md).

### 6.7 Audit

Every CRUD / merge / link writes to `audit_log` with old + new JSON,
user ID, IP, user agent, timestamp. Queries: per-worker, recent
system-wide, per-user.

### 6.8 FHIR R5

Bidirectional `Practitioner` resource conversion under
`/fhir/Practitioner` (handlers in `src/api/fhir/handlers.rs`; the wire
`resourceType` is `"Practitioner"` — the standard `agents/share/fhir.md`
§3 mapping, superseding the early non-standard `Worker` shape).
Search parameters: `name`, `family`, `given`,
`identifier`, `birthdate`, `gender`, `_count`.

**Status:** the FHIR handlers are implemented and **mounted** on the
axa router — `App::routes` registers `fhir_routes()` alongside the
REST and metrics route groups. The mount is
pinned by `tests/api_integration_test.rs::test_fhir_practitioner_route_is_mounted`
(un-gated, asserts the route is reachable via a `400` from the
`Path<Uuid>` extractor) and
`::test_fhir_worker_not_found_returns_operation_outcome` (DB-gated,
asserts a FHIR `OperationOutcome`). Tracked in §13 T-9 (done
2026-06-13).

### 6.9 Assessments

The service records and serves workforce **assessments** — aptitude,
personality, psychometric, and selection tests (domain model §5.5,
persistence §10.5, endpoints §9.2).

**Categories.** Four, each with its own scales:

- **Aptitude** — how a worker performs at tasks and reacts to
  situations: numerical reasoning, verbal reasoning, problem-solving,
  logical thinking.
- **Personality** — behavioural style and working qualities: how they
  work best, team compatibility, introversion/extraversion.
- **Psychometric** — covers aptitude **and** personality: behavioural
  styles, emotional intelligence, cognitive abilities.
- **Selection** — used during hiring to evaluate suitability for a
  role: job simulations, skills assessments, judgement tests.

**Required behaviour.**

1. Record an administration against a worker with its instrument,
   provider, dates, and per-scale results; read, update, and withdraw
   it (soft delete). Lookups are worker-scoped.
2. Refuse a result whose scale is outside the assessment's category
   (`422`) — psychometric is the one deliberate overlap, accepting
   aptitude and personality scales as well as its own.
3. Enforce the lifecycle `scheduled → in_progress → completed →
   expired` (with `cancelled` from any open state, and a direct
   `scheduled → completed` for a test recorded after the fact); refuse
   an illegal move with a `422` naming the current state.
4. Enforce the §5.5 invariants on create **and** on the merged record
   of an update, returning the full problem list.
5. Treat results as current only while the assessment is `completed`
   and unexpired, and derive the per-worker profile — the current
   reading per scale, what is **not** assessed, and the
   selection-suitability mean — from real scores only, never
   interpolated.
6. Honour the ABAC `mask` obligation on every read path and audit every
   assessment read and mutation (§6.6, §6.7).

**Explicitly out of scope.** Assessments are not a matcher signal
(§5.5); the service stores results, it does not administer or score
tests, and it does not decide hiring outcomes — the suitability mean is
a reported figure, not a recommendation.

