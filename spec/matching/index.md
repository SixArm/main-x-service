# Matching — monorepo-wide specification

This is the family-wide reference for **record matching** across the
Main X Index. It describes the shared model, the algorithms actually in
the code, the per-entity deterministic short-circuits, the confidence
bands, and how the standalone matcher crates are embedded in the
services. It is descriptive of the current implementation, not
aspirational: where a capability is deferred it says so.

Each matcher crate's own `spec/index.md` remains the **single source of
truth** for that crate (matcher crates follow a §1–§25 SDD shape — see
e.g. [person-matcher §12 Algorithm Specifications](../../person/person-matcher-rust-crate/spec/12-algorithm-specifications.md)).
This document is the cross-cutting view that sits above them.

Briefs feeding this page:
[agents/share/match.md](../../agents/share/match.md) ·
[agents/share/match-search-merge.md](../../agents/share/match-search-merge.md).

Related monorepo docs: data conventions in
[spec/data.md](../data.md); the per-topic family briefs for
[search](../../agents/share/search.md) and
[merge](../../agents/share/merge.md); the system
[architecture](../../agents/share/architecture.md). (Sibling
`spec/search/` and `spec/merge/` umbrella specs are not yet split out;
the `agents/share/*` briefs above are the canonical short-form for
those topics today.)

---

## 1. What matching is

A **match** compares **two records of the same entity type** and returns
a **confidence score in `[0.00, 1.00]`** plus a **per-component
breakdown** explaining how the score was reached. The breakdown is a
first-class output, never discarded — clinicians, caseworkers, and
auditors must be able to see *why* a pair was flagged.

Two strategies run as a two-phase pipeline:

| Strategy | Nature | Output |
|---|---|---|
| **Deterministic** | Rule-based, binary. Short-circuit rules (e.g. a shared national identifier) pin the score to `1.0` and skip the fuzzy phase entirely. | `true` / `1.0` + a `deterministic_match` flag |
| **Probabilistic** | Weighted, fuzzy. Each component scores `0–1`; the **weight-renormalised** sum is the overall score. | `score` + `confidence` band + `breakdown` |

The canonical result shape (per matcher crate) is:

```text
MatchResult { score: f64, is_match: bool, confidence, breakdown }
```

`is_match` is the authoritative go/no-go: it compares `score` against the
configured threshold (and, in `strict_mode`, additionally requires a
deterministic match — see
[person-matcher `match_persons`](../../person/person-matcher-rust-crate/src/matcher.rs)).
`confidence` is a coarse triage band derived from `score` (§4).

### 1.1 Missing-field renormalisation (the family invariant)

A component absent on **either** side scores `None`, not `0.0`. The
weighted average divides by the sum of the weights that **actually
contributed**, so a missing field neither contributes to the numerator
nor inflates the denominator — it is skipped, not penalised. See
[`weighted_average`](../../organization/organization-matcher-rust-crate/src/scoring.rs)
(organization), mirrored in every matcher crate. Empty input → `0.0`
(no evidence either way), never a divide-by-zero.

> Known limitation: when only one weak field (e.g. gender) participates,
> renormalisation can produce a high score from thin evidence. This is
> documented, not a bug — see the person-service bridge test
> `sparse_records_do_not_panic_and_score_in_range` in
> [`tests/duplicate_detection.rs`](../../person/person-service-with-loco/tests/duplicate_detection.rs).

---

## 2. Algorithms actually used

All string scores normalise to `[0.0, 1.0]`. Two empty strings → `1.0`;
one empty, one not → `0.0`.

| Algorithm | Where used | Implementation |
|---|---|---|
| **Jaro-Winkler** (case-insensitive, prefix bonus) | Names, titles | [`Scorer::jaro_winkler_similarity`](../../person/person-matcher-rust-crate/src/scorer.rs) — wraps `strsim::jaro_winkler` |
| **Levenshtein** (normalised: `1 − distance/max_len`) | Short strings, blended into names | `Scorer::levenshtein_similarity` (same file) |
| **Combined** (`0.7·JW + 0.3·Lev`) | Default name algorithm | `Scorer::combined_similarity`; selected by `SimilarityAlgorithm::Combined` |
| **Exact match** (binary `1.0`/`0.0`) | Gender, DOB, normalised identifiers | `Scorer::exact_match` |
| **Soundex phonetic** (4-char code) | Name/title phonetic bonus | per-crate `phonetic.rs` (e.g. [organization](../../organization/organization-matcher-rust-crate/src/phonetic.rs)) |
| **Haversine** + **Gaussian decay** (`exp(−(d/s)²)`) | Geo coordinates (place) | [`Scorer::haversine_metres` + `coordinates_score`](../../place/place-matcher-rust-crate/src/scorer.rs) |
| **Jaccard** (`\|A∩B\| / \|A∪B\|`) | Keyword / code / subject sets | per-crate `set_jaccard` (e.g. [case](../../case/case-matcher-rust-crate/src/matcher.rs)) |
| **Identifier exact-match** (canonicalise → byte-equal) | National IDs, scheme IDs | per-crate `identifiers.rs` / `is_deterministic` schemes |

### 2.1 The Soundex bonus

When the two primary names share a Soundex code **and** the name score
is still below the High band, an additive bonus of **`+0.05`** lifts the
name component, capped at a **`0.95` ceiling** (`PHONETIC_CEILING`) so a
purely phonetic agreement never reaches a perfect score. See the
`PHONETIC_BONUS` / `PHONETIC_CEILING` constants in
[organization `matcher.rs`](../../organization/organization-matcher-rust-crate/src/matcher.rs)
and [case `matcher.rs`](../../case/case-matcher-rust-crate/src/matcher.rs).
The person matcher applies the same `< ~0.95` gate to its phonetic
name score.

### 2.2 Geo specifics (place only)

Place uses Haversine great-circle distance (mean Earth radius
`6 371 000 m`, cross-dateline-correct) fed into a Gaussian decay with a
default **scale of `50.0` m**: `d = 0 → 1.0`, `d = scale → 1/e ≈ 0.368`,
`d = 3·scale → ≈ 0.0001`. Negative / non-finite inputs → `0.0`.

### 2.3 Date proximity (person)

Date-of-birth (and date-of-death) use a transposition-tolerant heuristic
rather than bare equality in the probabilistic phase: exact `1.0`,
day/month swap `0.5`, otherwise `0.0`; an off-by-one day stays
review-worthy. The **deterministic** phase always demands an exact date.
See the person-service bridge tests
`off_by_one_day_dob_softly_penalised_not_dropped` and
`same_name_different_dob_and_no_shared_identifier_does_not_short_circuit`.

---

## 3. Deterministic short-circuits (per entity)

The deterministic phase runs **first**. On the first rule that fires it
returns `1.0`, sets `breakdown.deterministic_match = true`, and skips
fuzzy scoring. The family pattern is three rule classes:

- **R-0** — both records share a value on the **same globally-unique
  identifier scheme** (`is_deterministic` schemes only; never
  cross-scheme).
- **R-1** — a **scope-qualified** identifier agrees (same jurisdiction /
  agency / provider + same scoped number).
- **R-2** — their **`same_as` / `sameAs` URL** sets overlap (a shared
  canonical identity URL).

Identifiers are always **scheme-local**: a value under scheme X is only
ever compared against the same scheme X. Two ten-digit numbers that
happen to collide across a UK NHS Number and a UK NI H&C Number never
cross-match.

| Entity | R-0 schemes (short-circuit to 1.0) | R-1 (scope-qualified) | R-2 |
|---|---|---|---|
| **Person** | 42 national personal-ID schemes (UK NHS Number, FR NIR, US SSN, DE KVNR, IT CF, NL BSN, SE Personnummer, …) + shared passport `(country, number)` pair; else exact name+DOB(+gender) demographic tuple | — (schemes are person-level) | — |
| **Organization** | LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT | same-jurisdiction tax id | `same_as` URL overlap |
| **Case** | Docket, ExternalCaseId, URI, UUID | same-agency normalised case number | `same_as` URL overlap |
| **Course** | DOI, Wikidata, OER, LOM, URI, UUID | provider-scoped course code | `sameAs` URL overlap |
| **Care pathway** | DOI, Wikidata, guideline-id, URI, UUID | provider-scoped pathway code | `sameAs` URL overlap |
| **Place** | shared `(scheme, value)` `place_id` pair (Google, OSM, …) | — | — (deterministic fallback: identical normalised name + normalised postcode) |
| **Worker** | person-level national-ID schemes (as Person) | — | — |

What deliberately does **not** short-circuit:

- **Classification codes** (NAICS / ISIC / SIC / `Custom`) — they
  identify a *category*, not an *entity*; a shared code is not a match
  (organization `classification_code_does_not_short_circuit`).
- **`local_id`** — different organisations may issue colliding values, so
  it is never scored (person, place, worker).
- **Cross-jurisdiction tax id** — a tax id only short-circuits within the
  same jurisdiction (organization `tax_id_short_circuits_only_within_jurisdiction`).

Per-matcher detail: see each crate's
[person](../../person/person-matcher-rust-crate/spec/index.md) ·
[organization](../../organization/organization-matcher-rust-crate/spec/index.md) ·
[place](../../place/place-matcher-rust-crate/spec/index.md) ·
[case](../../case/case-matcher-rust-crate/spec/index.md) ·
[course](../../course/course-matcher-rust-crate/spec/index.md) ·
[care-pathway](../../care-pathway/care-pathway-matcher-rust-crate/spec/index.md) ·
[worker](../../worker/worker-matcher-rust-crate/spec/index.md) spec.

---

## 4. Confidence classification

`confidence` is a coarse band over the raw `score`, fixed across config
presets (it does **not** track the tunable threshold). The exact
boundaries differ by crate; the family shape is Certain/Probable/
Possible/Unlikely (or the three-band High/Medium/Low rendering of it).

| Band (family) | Typical range | Notes |
|---|---|---|
| **Certain** / High | ≥ 0.95 | Safe to act on with minimal review |
| **Probable** / Medium | ≥ 0.70–0.85 | Inspect the breakdown before high-stakes use |
| **Possible** | ≥ 0.50–0.60 | Candidate; needs more evidence |
| **Unlikely** / Low | below | Not a match |

Concrete bands in code:

- **Person** ([`Confidence::from_score`](../../person/person-matcher-rust-crate/src/matcher.rs)):
  High ≥ 0.90, Medium ≥ 0.75, else Low (NaN/negative → Low, >1.0 → High).
- **Organization / Case** ([`Confidence::classify`](../../organization/organization-matcher-rust-crate/src/scoring.rs)):
  High ≥ 0.95, Medium ≥ 0.70, else Low (inclusive lower bounds).

### 4.1 Default thresholds & presets

`is_match` uses a tunable threshold, separate from the confidence bands:

| Matcher | Default | strict | lenient |
|---|---|---|---|
| Person | 0.85 | 0.95 (also requires deterministic) | 0.75 |
| Organization | 0.85 | 0.95 | 0.70 |
| Case | 0.85 | 0.95 | 0.70 |
| Place | 0.80 | 0.95 | 0.65 |

Presets move **only the threshold** — component weights stay identical
(case `presets_change_only_threshold`). Strict matches are always a
**subset** of lenient matches for the same pair, because the raw score is
config-independent (person `strict_config_demands_more_evidence_than_lenient_for_same_pair`).

---

## 5. The matcher crates and the service bridge

Each entity ships a **dependency-light, standalone matcher crate** — a
pure scoring library: no IO, no clocks, no RNG, no global state, no
`unsafe`; deterministic (same inputs ⇒ same bytes out); every
probabilistic match returns a per-field breakdown. These crates are the
**canonical reference implementation** and are published to crates.io.

Two embedding patterns exist:

1. **Adapter bridge (FHIR-shaped services: person, worker, place, …).**
   The service stores a rich domain model (named `HumanName`, vector
   `identifiers` / `addresses` / `telecom` / `documents`). It embeds the
   matcher crate and projects its model onto the matcher's flat input via
   an adapter `to_matcher_<entity>` (e.g.
   [`to_matcher_person`](../../person/person-service-with-loco/src/matching/adapter.rs)).
   The projection is lossy-but-well-defined: scalars sampled from
   collections (first phone / email; first address → primary, rest →
   `previous_addresses`), field renames (FHIR `state` → matcher `county`,
   `postal_code` → `postcode`), placeholder-date guards.

2. **DTO-is-the-matcher-type (loco.rs services: organization,
   care-pathway, case).** The API DTO **is** the matcher's own type
   (stored as JSONB), so there is **no adapter** — the service hands the
   matcher type straight to `MatchingEngine`.

### 5.1 The bridge test pins both halves

For adapter-bridge services, a black-box integration test drives the
service domain model through the adapter and asserts on
`MatchingEngine` output — pinning **both** the adapter's field-routing
**and** the matcher's scoring against the service's model. See
[`tests/duplicate_detection.rs`](../../person/person-service-with-loco/tests/duplicate_detection.rs)
(18 tests): identical-clone ≥ 0.95 / High; one-letter typo fuzzy ≥ 0.85;
shared NHS-number / tax-id / passport deterministic; unrelated records <
0.70; sparse-record safety; the full national-ID routing audit; strict ⊆
lenient. A regression on either side of the contract fires a test here.

---

## 6. National-identifier coverage

The person and worker matchers carry **42 person-level national-ID
schemes** (each a dedicated builder slot, one parser per scheme in
`identifiers.rs`), plus 9 per-country passport-format validators feeding
the multi-country `PassportBook` model. Parsers canonicalise
(whitespace/case) and verify the scheme's check digit / check character
where one exists; two textual layouts of the same identifier
canonicalise to the same string for byte-equality comparison. The
reference table lives in
[person-matcher `agents/national-person-identifiers.tsv`](../../person/person-matcher-rust-crate/agents/national-person-identifiers.tsv)
(and the worker mirror).

### 6.1 Scheme routing in the adapter

Adapter-bridge services route a service `Identifier` to the right
matcher slot by inspecting the scheme **`system` URI** (most-specific
fragment first), falling back to the `IdentifierType`. The person
adapter reaches **all 26** of the matcher's exposed national-ID slots
this way (e.g. `nhs-number` → UK NHS Number; `us-ssn`/`ssa.gov` → US
SSN; `cpf` → BR CPF; `ihi` disambiguated AU vs IE by digit count;
free-form `tax_id` defaults to US SSN). The full routing table is
documented inline in
[`adapter.rs`](../../person/person-service-with-loco/src/matching/adapter.rs)
and pinned by `all_national_id_schemes_route_to_their_slot`.

### 6.2 Permanent out-of-scope decisions

- **Organisation-level identifiers are never scored by the person/worker
  matchers** — NHS ODS org codes, GLN, employer/department codes, etc.
  Two workers at the same practice share the same org code, so an
  exact-match short-circuit on it would declare colleagues to be the same
  person. Such values, if carried, belong in the unscored `local_id`
  field; embedding services drop them at the adapter. See
  [worker-matcher §2 Scope](../../worker/worker-matcher-rust-crate/spec/02-scope.md).
- **`local_id` is never scored** (collidable across issuers).
- **Cross-scheme identity resolution** and **blocking / candidate
  generation** are out of scope for the matcher libraries — they are a
  consumer (service) concern (§8).

---

## 7. Configurability

Weights and thresholds are tunable via each crate's `MatchConfig`
(weights are dimensionless and need not sum to 1.0 — renormalisation
handles that). Three presets cover most needs: `default`, `strict`,
`lenient` (§4.1). The full per-component **score breakdown** is returned
on every probabilistic match and is surfaced in service API responses
(and persisted to the review queue's `score_breakdown`), so scores are
always explainable. Representative default weights:

| Entity | Top components (default weight) |
|---|---|
| Person | family-name 0.20, DOB 0.20, given-name 0.15, national-ID slot 0.30 each (when present), email/phone/gender 0.05 |
| Organization | name 0.35, address 0.20, url 0.15, jurisdiction 0.10, founding-date 0.10, keywords 0.10 |
| Case | title 0.30, subjects (Jaccard) 0.25, keywords (Jaccard) 0.15, case-number 0.15, type 0.10, status 0.05 |

Do not change default weights or thresholds without updating the owning
crate's spec and CHANGELOG in the same change (the SDD three-part rule).

---

## 8. Duplicate-detection surface (services)

Matching powers duplicate detection in the services. The REST surface is
uniform across entities (`<plural>` = the entity's plural path segment):

| Trigger | Endpoint | Behaviour |
|---|---|---|
| **Real-time on create** | `POST /api/<plural>` | Runs duplicate detection; returns **`409 Conflict`** with candidate matches when duplicates are found |
| **Explicit check** | `POST /api/<plural>/check-duplicates` | Checks without creating |
| **Batch** | `POST /api/<plural>/deduplicate` | Scans the index; `threshold` / `max_candidates` / `auto_merge_threshold` |
| **Match against existing** | `POST /api/<plural>/match` | Scores a query record against existing records |

Detected duplicates flow into a **review queue** with status `Pending` /
`Confirmed` / `Rejected` / `AutoMerged`; high-confidence pairs (≥
`auto_merge_threshold`) may auto-merge, the rest queue for review. Each
review item carries the `match_score`, the `match_quality` band, and the
per-component `score_breakdown`.

**Candidate generation / blocking is a consumer concern, not part of the
matcher libraries.** In the services, candidate sets are sourced from
search (Tantivy full-text where implemented; ILIKE name/title search in
the loco.rs services where Tantivy is **deferred**) before the matcher
scores the survivors. The matcher crates expose `match_one_to_many` /
`rank_one_to_many` helpers for scoring a query against a candidate slice,
but do no blocking themselves.

Related workflows:
[search](../../agents/share/search.md) (how candidates are found) ·
[merge](../../agents/share/merge.md) (what happens to a confirmed
duplicate) · [architecture](../../agents/share/architecture.md) (where
the matcher sits in the request flow).

---

## 9. Implemented vs deferred

| Capability | Status |
|---|---|
| Probabilistic + deterministic matching, per-component breakdown | Implemented (all matcher crates) |
| Confidence bands + tunable thresholds/weights + presets | Implemented |
| National-ID scheme routing (person/worker), 42 schemes | Implemented |
| Adapter bridge + bridge tests (FHIR-shaped services) | Implemented (person; pattern reused) |
| DTO-is-matcher-type embedding (loco.rs services) | Implemented (organization, care-pathway, case) |
| Duplicate detection (409 on create, check, batch, review queue) | Implemented |
| Geo matching (Haversine + Gaussian decay) | Implemented (place) |
| **Tantivy full-text candidate generation / blocking** | Implemented in the original FHIR-shaped services; **deferred** in the loco.rs services (ILIKE search in the interim) |
| Population-scale Fellegi-Sunter EM weight training | Out of scope (matcher libraries) |
| Cross-scheme identity resolution; non-Latin phonetic encoders | Out of scope / opt-in deferred |
