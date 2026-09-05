# case-matcher — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec edit + code edit + test edit. Live
> work queue is §23; open questions are §16.

## 1. Purpose

`case-matcher` is a reusable, dependency-light Rust library for
**pairwise governmental case-management record matching**. A *case*
(case management / case tracking) is an open or historical matter
handled by a public agency on behalf of one or more subjects — a
benefit claim, legal action, social-services referral, licensing
application, complaint, appeal, investigation, and so on. Given two
`Case` records the matcher returns a `MatchResult`: score in
`[0.0, 1.0]`, `Confidence`, `is_match`, and a per-component
`MatchBreakdown`. It is the canonical algorithm embedded in
`case-service`'s matching layer.

## 2. Scope

In scope: the attributes that distinguish one case from another —
title, involved-party subjects, agency-scoped case number, case type,
status, keywords, and document identifiers. Out of scope: the full case
content (timeline, notes, attachments, outcomes), party-level personal
data, and anything requiring IO, a runtime, or network access.

## 3. Glossary

- **Case** — an open or historical matter tracked by an agency for one
  or more subjects.
- **Deterministic identifier** — globally unique (`Docket`,
  `ExternalCaseId`, URI, UUID). A match pins the score to `1.0`.
- **Agency-scoped code** — `case_number`/`AgencyCaseNumber`/`LocalId`;
  only unique within the handling organisation.
- **Subject** — an opaque involved-party id (e.g. a person pid); shared
  subjects strongly corroborate identity.

## 4. Research basis

Governmental cases are largely identified by their **handling agency +
local case number**, by a court/tribunal **docket**, by a
**cross-system external id**, or by the **subjects** involved. Matching
therefore combines deterministic linkage on those identifiers with
fuzzy comparison of the title and overlap of the involved-party
subjects.

## 5. Algorithm overview

```
Input: Case A, Case B, MatchConfig
  ├─ R-0 deterministic identifier match?        ─yes─> 1.0
  ├─ R-1 same agency + case_number?             ─yes─> 1.0
  ├─ R-2 same_as URL overlap?                   ─yes─> 1.0
  │
  ├─ title_score        (always)      Jaro-Winkler + Soundex bonus
  ├─ subjects_score     (≥1 set)      Jaccard over folded subject ids
  ├─ case_number_score  (same agency) 1.0/0.0
  ├─ case_type_score    (both set)    exact enum (1.0/0.0)
  ├─ status_score       (both set)    exact enum (1.0/0.0)
  ├─ keywords_score     (≥1 set)      Jaccard
  ├─ tags_score         (both set)    Jaccard over folded tags  — PLANNED, §13b/§23, not yet in code
  ├─ relationships_score(≥1 set)      typed-set Jaccard over (relation, case_id)  — PLANNED, §13a/§23, not yet in code
  └─ renormalised weighted average over present components
```

`priority`, `opened_date`, and `in_language` are carried on `Case`
but never scored.

> **Implementation status.** `tags_score` and `relationships_score`
> are specified ahead of the code (per this crate's spec-first
> discipline — see `agents/spec-driven-development.md`): §23 carries
> the open task to add them. Until that task lands, `Case` has no
> `tags` or `relationships` field, `MatchConfig` has no
> `tags_weight`/`relationships_weight`, and `MatchBreakdown` has no
> `tags_score`/`relationships_score` — the live algorithm is exactly
> the six components above the line, matching `src/matcher.rs` today.

## 6. Domain model

`Case` (as implemented today): `title` (required), `alternate_titles`,
`case_number`, `agency_id`, `agency_name`, `case_type`, `status`,
`priority` (data only), `opened_date` (data only), `subjects`,
`keywords`, `identifiers` (`CaseIdentifier { scheme, value }`),
`same_as`, `in_language`.

**Planned (§23, not yet on `Case`):** `tags` and
`relationships` (`Vec<RelationshipRef>`).

`tags` (default empty, once added) would be user-applied operational
labels for grouping / filtering / triage / workflow (e.g. `"vip"`,
`"review"`, `"fast-track"`), distinct from `keywords` (subject-matter
discovery terms). The matcher would score `tags` by case-insensitive
set Jaccard (§13b); a supporting signal, not an identifying field on
its own.

`relationships`, once added, would hold typed case-to-case references —
`RelationshipRef { relation: RelationKind, case_id: String }` where
`RelationKind` is a `#[non_exhaustive]` enum mirroring the service:
`RelatedTo` and `ConsolidatedWith` (symmetric), `ParentCase` / `SubCase`
(inverses — consolidation hierarchy), and `Supersedes` / `SupersededBy`
(inverses — replacement). `case_id` is an opaque registry id
(whitespace-trimmed, non-empty); the matcher would not resolve, invert,
or transitively close the references — it would compare the two cases'
relationship **sets** (§13a). A supporting signal, not an identifying
field on its own.

`CaseType`: `Benefit`, `Legal`, `SocialServices`, `Healthcare`,
`Housing`, `Immigration`, `Licensing`, `Complaint`, `Appeal`,
`Investigation`, `Tax`, `Employment`, `Custom(String)`.
`CaseStatus`: `Open`, `InProgress`, `Pending`, `OnHold`, `Closed`,
`Resolved`, `Rejected`, `Withdrawn`, `Custom(String)`.
`Priority` (data only): `Low`, `Normal`, `High`, `Urgent`.
`IdentifierScheme`: deterministic — `Docket`, `ExternalCaseId`, `Uri`,
`Uuid`; agency-scoped — `AgencyCaseNumber`, `LocalId`; plus
`Custom(String)`.

Serialisation: struct fields are snake_case; enum unit variants
serialise as their bare PascalCase string (`"Docket"`), and `Custom`
serialises as `{"Custom":"label"}`.

## 7. Configuration

`MatchConfig` weights (default, as implemented today): title 0.30,
subjects 0.25, case_number 0.15, case_type 0.10, status 0.05,
keywords 0.15 — these six sum to exactly 1.0. **Planned (§23, not yet
on `MatchConfig`):** `tags_weight` 0.05 (a supporting signal — §13b)
and `relationships_weight` 0.05 (a supporting signal — §13a); once
added, the eight weights would be renormalised over the participating
components per match (§17), so they need not sum to exactly 1.0.
Threshold 0.85. Presets: `strict()` 0.95, `lenient()` 0.70.

Changing any default weight is a three-part change: edit this section,
the `MatchConfig` defaults, and `CHANGELOG.md`.

**Validation.** Every field is `pub` and directly settable — the plain
struct literal is still how the presets and the common case build a
config — but a caller assembling one from untrusted input (e.g.
deserialized config) can call the additive, opt-in
`MatchConfig::validated(self) -> Result<Self>`, which rejects a
negative, `NaN`, or infinite weight on any of the six fields, or a
threshold outside `[0.0, 1.0]`, returning `Error::InvalidConfig` naming
the first offending field. This exists because such a value reaching
§17's renormaliser unchecked can push the returned score outside
`[0.0, 1.0]` or produce `NaN`, breaking the bounded-and-finite
invariant §19 documents and the `Confidence` banding built on it. Same
shape as the sibling `organization-matcher`/`care-pathway-matcher`
crates' identical `MatchConfig`.

## 8. Normalisation

`fold` (trim + NFKC + lowercase, diacritic-preserving); `case_number`
(alphanumeric-only, uppercased — so `"CV-2024-001234"` ≡
`"cv 2024 001234"`); `url` (fold + drop trailing slash); `fold_set`
(sort + dedupe). Subjects and keywords compare via `fold_set` Jaccard
today; `tags` (§13b) would too, once implemented.

## 9. Title similarity

Best Jaro-Winkler over `title` + `alternate_titles` (folded), with a
Soundex +0.05 bonus on the primary titles capped at 0.95.

## 10. Subjects

Jaccard over the `fold_set` of involved-party id strings. Shared
subjects are a strong identity signal, so subjects carry the
second-highest weight. Skipped when both sides are empty.

## 11. Case number

Within the same agency key (`agency_id`, or `agency_name` fallback):
1.0 if normalised numbers equal, else 0.0. Across agencies (or missing
agency): `None` (a local number like `CV-2024-001234` is not globally
unique).

## 12. Case type & status

Exact enum match → 1.0 else 0.0. `None` when either side is unset.
Case type weight 0.10, status weight 0.05.

## 13. Keywords

Jaccard over `fold_set`. Skipped when both empty; `0.0` when exactly one
side is populated.

## 13a. Relationships (planned — see §23)

> **Not yet implemented.** `Case` carries no `relationships` field and
> `MatchBreakdown` carries no `relationships_score` today; this section
> specifies the intended design ahead of the §23 implementation task.

Typed-set **Jaccard** over the `(relation, case_id)` pairs:
`score = |A ∩ B| / |A ∪ B|`, where each side's set is
`{ (r.relation, r.case_id) for r in relationships }`. So a `ParentCase`
reference only agrees with a `ParentCase` reference to the **same**
case id — the relation kind is part of the key; `SubCase`, `RelatedTo`,
`ConsolidatedWith`, `Supersedes`, and `SupersededBy` are compared as
opaque, distinct kinds (no inversion or transitive closure). `None`
(does not participate) when **either** side has no relationships;
otherwise a value in `[0.0, 1.0]`. A **supporting** signal weighted
`relationships_weight` (§7, default `0.05`); shared references never
single-handedly establish a match.

## 13b. Tags (planned — see §23)

> **Not yet implemented.** `Case` carries no `tags` field and
> `MatchBreakdown` carries no `tags_score` today; this section
> specifies the intended design ahead of the §23 implementation task.

Plain set **Jaccard** over the case-insensitively normalised tag sets:
`tags_score = |A ∩ B| / |A ∪ B|` over each side's `fold_set` of tags —
identical to how `keywords` (§13) and `subjects` (§10) are scored.
`None` (does not participate) when **either** side has an empty tag set;
otherwise a value in `[0.0, 1.0]`. `tags` are user-applied operational
labels (grouping / triage / workflow), not subject-matter terms, so they
are a **supporting** signal weighted `tags_weight` (§7, default `0.05`);
shared tags never single-handedly establish a match.

## 14. Data-only fields

`priority`, `opened_date`, and `in_language` are carried for
downstream consumers and MUST NOT contribute to the score.

## 15. Deterministic identifier short-circuits

R-0: any shared value on a deterministic scheme → 1.0. Empty values
ignored. `AgencyCaseNumber`/`LocalId`/`Custom` are excluded.
**SEC-M2:** a *trivial* value is also ignored — one with no
alphanumeric character other than `'0'` (i.e. empty/punctuation-only,
the sentinel `"0"`, or an all-zeros UUID) — so two different cases
sharing only a placeholder identifier do not spuriously short-circuit
(`src/matcher.rs::is_trivial_identifier`).

## 16. Agency+number, same_as, and open questions

R-1: shared non-empty agency key + equal normalised `case_number` →
1.0. R-2: any normalised `same_as` URL overlap → 1.0. **SEC-M2:** R-2
also ignores a bare root `"/"` (which `normalize::url` deliberately
keeps non-empty), so two different cases sharing only `"/"` do not
short-circuit.

Open questions: should a shared subject set alone be a strong pin
(currently probabilistic — many cases per subject)? Should case-type or
status mismatch *penalise* rather than just not corroborate? Should
`opened_date` proximity become a (year-only) scored component?

## 17. Renormalisation

Weighted average over `Some` components only; divisor is the sum of
contributing weights.

## 18. Confidence classification

`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`. Separate from
`MatchConfig::threshold` (`is_match`).

## 19. Quality goals

Total functions (no `unwrap`/`expect`/`panic`); no `unsafe`;
deterministic; explainable; diacritic-correct.

**Bounded-and-finite score, and what covers it.** `MatchResult::score`
is claimed finite and in `[0.0, 1.0]` for every input the matcher is
driven with. This is proven two ways, deliberately covering different
`MatchConfig` populations: `tests/proptests.rs`'s
`score_is_finite_and_bounded` drives the engine over arbitrary `Case`
pairs under `MatchingEngine::default_config()` — the built-in presets
only. A **hand-built** `MatchConfig` (a struct literal with an
arbitrary weight or threshold, e.g. from deserialized config) is a
*different* population the presets never exercise, and is covered
separately: `validated_config_never_produces_an_unbounded_score`
generates adversarial weight/threshold vectors and asserts the
guarantee holds for any config that clears `MatchConfig::validated`
(§7) — an unvalidated hand-built config carrying a negative, `NaN`, or
infinite weight is explicitly **not** covered by this invariant, which
is exactly why `validated` exists rather than trusting every `pub`
field unconditionally.

## 20. Consumption

`case-service` embeds this crate via an adapter and calls
`MatchingEngine::match_cases`. A bridge test in the service pins the
contract.

## 21. Compatibility

Semantic versioning. Re-exports from `lib.rs` are the contract:
`Case`, `CaseIdentifier`, `IdentifierScheme`, `CaseType`, `CaseStatus`,
`Priority`, `MatchingEngine`, `MatchConfig`, `MatchResult`,
`MatchBreakdown`, `Confidence`, `Error`, `Result`. (`RelationshipRef`
and `RelationKind` are **planned** additions — §13a, §23 — not yet
present in `src/case.rs` or re-exported from `lib.rs`; add them to
this list in the same PR that lands §23's relationships task.)

## 22. Anti-patterns

Do not short-circuit on agency-scoped or `Custom` schemes. Do not score
a `case_number` across agencies. Do not score `priority`,
`opened_date`, or `in_language`. Do not strip diacritics. Do not add
IO, async, or panics to library code.

## 23. Tasks (live work queue)

- [ ] Implement the `relationships` component in code: add
      `relationships: Vec<RelationshipRef>` to `Case`, the
      `RelationshipRef { relation, case_id }` + `#[non_exhaustive]`
      `RelationKind` (`RelatedTo`, `ParentCase`, `SubCase`, `Supersedes`,
      `SupersededBy`, `ConsolidatedWith`) types, the typed-set Jaccard
      `relationships_score` (§13a), `relationships_weight` (default
      `0.05`, §7) on `MatchConfig`, and `relationships_score` on
      `MatchBreakdown`; re-export the new types (§21); update
      `CHANGELOG.md`.
- [ ] Implement the `tags` component in code: add `tags: Vec<String>`
      (default empty) to `Case`, the plain set-Jaccard `tags_score`
      (§13b, `None` when either side empty), `tags_weight` (default
      `0.05`, §7) on `MatchConfig`, and `tags_score` on `MatchBreakdown`;
      update `CHANGELOG.md`.
- [ ] Optional year-only `opened_date` proximity component.
- [ ] Optional case-type taxonomy (related types score partial).
- [ ] Split this single `spec/index.md` into the numbered §-per-file
      layout used by the sibling matcher crates.
- [x] **Bound `subjects`/`keywords` array sizes inside the library
      itself, or document that it relies entirely on the caller.**
      *(Verified: `grep -n "MAX_" src/*.rs` finds nothing — no length
      cap exists anywhere in this crate.)* The family's SEC-M1 caps
      (`MAX_ARRAY_LEN`/`MAX_ITEM_LEN`, `agents/share/security.md`
      invariant 3) live only in `case-service`'s
      `src/validation.rs`, which runs *before* the matcher is called —
      but this crate is documented as "usable standalone" (`AGENTS.md`,
      `agents/share/overview.md`), and a standalone consumer with no
      such caps can feed an arbitrarily large `subjects`/`keywords`
      array straight into the Jaccard component (`matcher.rs`'s
      set-Jaccard over `fold_set`), which is unbounded O(n·m). Either
      add an opt-in cap (a `MatchConfig` field or a documented
      `MatchingEngine::match_cases_bounded`), or add a prominent
      rustdoc note on `MatchingEngine::match_cases` and the crate root
      stating the caller must bound array sizes itself, plus a
      `CHANGELOG.md` entry. Update `spec/index.md` §19/§22 either way.
      **Acceptance:** either a cap exists and is unit-tested (an
      over-long `subjects`/`keywords` array is truncated or rejected
      deterministically, never scored in full), or the crate-root/API
      rustdoc explicitly states the caller's obligation and a doctest
      or `AGENTS.md` note points at `case-service`'s `MAX_ARRAY_LEN` as
      the reference cap a standalone consumer should copy.
      **Resolution (2026-09-05):** chose the documentation path (an
      in-library cap would need a `MatchConfig` field threaded through
      every component that reads either array, a bigger and more
      judgment-laden change than this crate's own `MatchConfig` weight
      surface warrants for a first pass; the family's existing caps all
      live at the service validation layer, not inside a matcher). Added
      a "The caller must bound array sizes" section to the crate-root
      docs (`src/lib.rs`) naming the O(n·m) Jaccard cost and pointing at
      `case-service`'s `MAX_ARRAY_LEN`/`MAX_ITEM_LEN` as the reference
      cap; a matching note on `MatchingEngine::match_cases`'s own
      rustdoc (`src/matcher.rs`), since `rank`/`match_one_to_many` both
      call it and inherit the same obligation; and a new golden rule
      (#6) in `AGENTS.md` stating the obligation plainly for an agent
      or integrator who reads that file instead. No test added — a
      documentation-only obligation has nothing to unit-test; the
      existing test suite (unchanged) still proves the crate's own
      never-panic/bounded-score behavior on whatever input it is given.
      Verified: `cargo test` (28 tests: 8 unit + 13 public-API + 7
      doctests, all green, including the newly-noted `match_cases`
      doctest), `cargo clippy --all-targets --all-features -- -D
      warnings`, `cargo fmt --check`.
- [ ] **Criterion bench group scaling `subjects`/`keywords` array size
      per `Case`, not just candidate-list length.**
      *(Verified: `grep -n "fn bench_" benches/match_pair.rs` shows
      `bench_match_pair`, `bench_deterministic`, `bench_rank` — the
      last sets `Throughput::Elements` only over the *candidate count*
      10/100/1000, §24 "Testing strategy"; none scales a single
      `Case`'s own array fields.)* Add a `bench_field_arrays` group that
      holds two records fixed and grows `subjects`/`keywords` (e.g.
      10/100/1000 entries each) with `Throughput::Elements`, so the
      O(n·m) Jaccard cost the item above is about is directly visible
      in `cargo bench` output rather than only inferred from the source.
      **Acceptance:** `cargo bench --no-run` compiles the new group; a
      local `cargo bench` run shows near-linear (or worse) scaling with
      array size, recorded in a `CHANGELOG.md` note.
- [x] **Property-test `MatchConfig` values other than the built-in
      presets.** *(resolved 2026-09-05.)* The SEC-M6 property suite
      (§24) only ever exercised `MatchingEngine::new(MatchConfig::default())`;
      the six weight fields on `MatchConfig` were all `pub` with no
      validating constructor, so a caller could build one directly with
      e.g. a negative or `NaN` weight.
  - **Resolved.** Ported organization-matcher's/care-pathway-matcher's
    identical `MatchConfig::validated(self) -> Result<Self>` fix
    (§7): rejects a negative/`NaN`/infinite weight on any of the six
    fields, or a threshold outside `[0.0, 1.0]`, via the new
    `Error::InvalidConfig(String)` variant naming the first offending
    field — the plain struct literal keeps working for the common
    case. Six new unit tests (`src/config.rs`) pin the accept/reject
    boundary. A new proptest
    (`validated_config_never_produces_an_unbounded_score`,
    `tests/proptests.rs`) generates a 7-value adversarial vector (6
    weights + threshold) and asserts an accepted config's score stays
    finite and in `[0.0, 1.0]` while a rejected one really was
    malformed — so the answer to "does the finite-score guarantee
    cover hand-built `MatchConfig` values" is: **only once validated**
    (§19 states this explicitly; no `MatchingEngine::try_new` or
    similar was added — validation stays a config-construction step,
    not an engine-construction one). Verified: `cargo test` (52 lib +
    9 proptests + 13 public-API + 7 doctests, all green, up from 46 lib
    + 8 proptests), `cargo clippy --all-targets --all-features -- -D
    warnings`, `cargo fmt --check`, `cargo doc --no-deps` all clean.

## 24. Testing strategy

Unit tests embedded per module; an integration suite
(`tests/public_api.rs`) over the re-exported surface; rustdoc examples
run as doctests; property-based tests (`tests/proptests.rs`, `proptest`)
pinning never-panic and finite-`[0.0,1.0]`-score invariants. Gate:
`cargo test`, `cargo clippy --all-targets --all-features -- -D
warnings`, `cargo fmt --check`.

**Fuzzing (SEC-I2).** A standalone `fuzz/` `cargo-fuzz` crate (not a
workspace member — never affects the stable build/test/clippy gate
above) carries two coverage-guided libFuzzer targets, `match_cases` and
`normalize`, over the same never-panic / finite-score invariants. Run
on nightly: `cargo +nightly fuzz run <target>` — see `fuzz/README.md`.

## 25. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under the latest version.
