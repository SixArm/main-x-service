# organization-matcher — Specification

> **Single source of truth.** Code conforms to this spec. A behavioural
> change is a three-part PR: spec edit + code edit + test edit. Live
> work queue is §23; open questions are §16.

## 1. Purpose

`organization-matcher` is a reusable, dependency-light Rust library for
**pairwise organization-record matching**, modelled on
[schema.org/Organization](https://schema.org/Organization). Given two
`Organization` records it returns a `MatchResult`: a score in
`[0.0, 1.0]`, a `Confidence` band, an `is_match` boolean, and a
per-component `MatchBreakdown`. It is the canonical algorithm embedded
in `organization-service`'s matching layer.

## 2. Scope

In scope: the properties of `schema.org/Organization` that carry
**identity** signal — names, registration identifiers, postal address,
website, jurisdiction, founding date, keywords, and **typed
organization-to-organization relationship references**
(`parentOrganization`/`subOrganization`, successor/predecessor) compared
as opaque sets (§14a). Out of scope: relationship **graph traversal**
(the matcher never resolves, inverts, or transitively closes a
reference — it compares the two records' relationship *sets*),
employees, brands, and anything requiring IO, a runtime, or network
access.

## 3. Glossary

- **Deterministic identifier** — globally unique by construction (LEI,
  DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT). A match pins the
  score to `1.0`.
- **Jurisdiction-scoped identifier** — `TaxId`; only unique within a
  country/register.
- **Classification code** — `Naics`/`IsicV4`/`Sic`; describes the
  *sector*, never the entity.
- **Legal name normalisation** — folding plus stripping legal-form
  suffix tokens (`Inc`, `Ltd`, `GmbH`, …) so `"Acme, Inc."` ≡ `"ACME"`.

## 4. Research basis

Entity resolution for organizations relies on (a) authoritative
registers keyed by globally-unique codes (GLEIF/LEI, D&B/DUNS, ROR,
Wikidata) for deterministic linkage, and (b) fuzzy comparison of names
(with legal-suffix and diacritic normalisation), addresses, and web
domains for probabilistic linkage. This crate combines both, mirroring
the family-wide person/place matchers.

## 5. Algorithm overview

```
Input: Organization A, Organization B, MatchConfig
  ├─ R-0 deterministic identifier match? ─yes─> 1.0
  ├─ R-1 same jurisdiction + tax id?     ─yes─> 1.0
  ├─ R-2 same_as URL overlap?            ─yes─> 1.0
  │
  ├─ name_score          (always)   legal-suffix-aware Jaro-Winkler + Soundex bonus
  ├─ address_score       (both set)  weighted field-by-field Jaro-Winkler
  ├─ url_score           (both set)  domain equality (1.0) else host Jaro-Winkler
  ├─ jurisdiction_score  (both set)  exact country (1.0/0.0)
  ├─ founding_date_score (both set)  same year 1.0, ±1yr 0.5, else 0.0
  ├─ keywords_score      (≥1 set)    Jaccard
  └─ renormalised weighted average over present components
```

**Planned, not yet implemented (§23):** `relationships_score` (typed-set
Jaccard over `(relation, organization_id)`) and `tags_score` (Jaccard
over normalised tags) are specced in §14a/§14b as future components of
the same pipeline, but neither field exists on `Organization` or
`MatchBreakdown` in code today — see the note at the head of §14a.

## 6. Domain model

`Organization`: `name` (required), `legal_name`, `alternate_names`,
`identifiers` (`OrgIdentifier { scheme, value }`), `url`, `same_as`,
`address` (`PostalAddress`), `jurisdiction` (ISO 3166), `founding_date`
(ISO-8601), `telephone`, `email`, `keywords`. This is the complete field
list in `src/organization.rs` today.

**Planned, not yet implemented (§23):** a `tags: Vec<String>` field of
short, operator-applied free-text labels (grouping / triage / workflow),
distinct from the descriptive `keywords`; defaults empty. A
**supporting** signal, meant to be scored by set Jaccard (§14b), never
identifying on its own.

Also planned: a `relationships: Vec<RelationshipRef>` field, where
`RelationshipRef { relation: RelationKind, organization_id: String }`
would hold a typed organization-to-organization reference.
`RelationKind` is designed as a `#[non_exhaustive]` enum mirroring the
service: `SubOrganizationOf` / `ParentOrganizationOf` (containment) and
`SuccessorOf` / `PredecessorOf` (mergers / renames / reorganisations).
`organization_id` would be an opaque registry id (whitespace-trimmed,
non-empty); the matcher is designed to **not** resolve, invert, or
transitively close the references — only compare the two records'
relationship **sets** (§14a). A supporting signal, not an identifying
field on its own. Neither `RelationshipRef` nor `RelationKind` exists in
`src/organization.rs` yet.

`IdentifierScheme`: deterministic — `Lei`, `Duns`, `Iso6523`, `Gln`,
`Wikidata`, `Ror`, `Isni`, `Vat`; scoped — `TaxId`; classification —
`Naics`, `IsicV4`, `Sic`; plus `Custom(String)`.

`PostalAddress`: `street_address`, `locality`, `region`, `postal_code`,
`country` — all optional; only fields present on *both* sides
contribute.

## 7. Configuration

`MatchConfig` weights (default): name 0.35, address 0.20, url 0.15,
jurisdiction 0.10, founding_date 0.10, keywords 0.10 — these six weights
are the whole of `MatchConfig` today and sum to 1.0. The renormaliser
(§17) divides by the sum of the weights whose component actually
participated, so absent data never drags the score down.
Threshold 0.85. Presets: `strict()` 0.95, `lenient()` 0.70.

**Planned, not yet implemented (§23):** a supporting `relationships_weight`
(default `0.05`, §14a) and `tags_weight` (default `0.05`, §14b) alongside
the six weights above, summing to 1.0 across eight weights once landed.
Neither field exists on `MatchConfig` yet — do not assume they are read.

## 8. Normalisation

`fold` (trim + NFKC + lowercase); `legal_name` (fold + punctuation→space
+ strip legal-form suffix tokens + collapse, never empty); `domain`
(scheme/`www.`/path/port/userinfo stripped); `fold_set` (sort + dedupe).
Diacritics are preserved (`Müller` ≠ `Muller`).

## 9. Name similarity

Best Jaro-Winkler over the cross-product of each side's name keys
(`name` + `legal_name` + `alternate_names`, each `legal_name`-normalised).
A Soundex match on the primary keys adds a +0.05 bonus capped at 0.95.

## 10. Address similarity

Weighted field-by-field Jaro-Winkler with internal weights street 0.30,
locality 0.25, postal 0.20, region 0.15, country 0.10; renormalised over
the fields present on both sides. `None` when either record lacks an
address.

## 11. URL / domain

Compared on extracted registered domain: equal → 1.0, else
Jaro-Winkler on the domain strings. `None` when either url is absent.

## 12. Jurisdiction

Exact (case-folded) country match → 1.0 else 0.0. `None` when either is
absent. Also gates the R-1 tax-id short-circuit.

## 13. Founding date

Compared by year (first four characters parsed): same 1.0, one year
apart 0.5, else 0.0. `None` when either is absent or unparseable.

## 14. Keywords

Jaccard over `fold_set(keywords)`. Skipped when both are empty; `0.0`
when exactly one side has keywords.

## 14a. Relationships — `relationships_score`

> **Planned, not yet implemented — see §23.** This section is a design
> spec for a future component; `relationships`/`RelationshipRef` do not
> exist on `Organization` (§6), `relationships_weight` does not exist on
> `MatchConfig` (§7), and `relationships_score` does not exist on
> `MatchBreakdown` today. Written in the present tense below because it
> describes the intended behaviour once §23's task lands.

Typed-set **Jaccard** over the `(relation, organization_id)` pairs:
`score = |A ∩ B| / |A ∪ B|`, where each side's set is
`{ (r.relation, r.organization_id) for r in relationships }`. So a
`SubOrganizationOf` reference only agrees with a `SubOrganizationOf`
reference to the **same** organization id — the relation kind is part of
the key; `ParentOrganizationOf` / `SuccessorOf` / `PredecessorOf` are
compared as opaque, distinct kinds (no inversion or transitive closure).
`None` (does not participate) when **either** side has no relationships;
otherwise a value in `[0.0, 1.0]`. A **supporting** signal weighted
`relationships_weight` (§7, default `0.05`); shared references never
single-handedly establish a match.

## 14b. Tags — `tags_score`

> **Planned, not yet implemented — see §23.** Same status as §14a:
> `tags` does not exist on `Organization` (§6), `tags_weight` does not
> exist on `MatchConfig` (§7), and `tags_score` does not exist on
> `MatchBreakdown` today.

Plain set **Jaccard** over `fold_set(tags)` (case-insensitively
normalised tag sets): `score = |A ∩ B| / |A ∪ B|`. Identical to the
`keywords` component (§14) in mechanism, but over the operator-applied
`tags` set rather than descriptive keywords. `None` (does not
participate) when **either** side has an empty tag set; otherwise a value
in `[0.0, 1.0]`. A **supporting** signal weighted `tags_weight` (§7,
default `0.05`); shared tags never single-handedly establish a match.

## 15. Deterministic identifier short-circuits

R-0: any shared value on a deterministic scheme → 1.0. Empty values are
ignored. `TaxId`/classification/`Custom` are excluded.

## 16. Same-jurisdiction tax id + same_as (and open questions)

R-1: both records share a non-empty `jurisdiction` AND a `TaxId` value →
1.0. R-2: any case-folded `same_as` URL overlap → 1.0.

Open questions: should an exact `url` domain match be a deterministic
pin (currently strong-but-probabilistic, since parents/subsidiaries can
share a domain)? Should `vatID` validate its national prefix?

## 17. Renormalisation

The weighted average runs only over `Some` components; the divisor is
the sum of the contributing weights. Absent data never drags the score
down.

## 18. Confidence classification

`High` ≥ 0.95, `Medium` ≥ 0.70, else `Low`. Separate from
`MatchConfig::threshold` (drives `is_match`).

## 19. Quality goals

Total functions (no `unwrap`/`expect`/`panic` in library code); no
`unsafe`; deterministic (no clocks/RNG/env); explainable (every match
carries a per-component breakdown); diacritic-correct.

## 20. Consumption

`organization-service` embeds this crate via an adapter projecting its
service-side `Organization` onto this model, then calls
`MatchingEngine::match_organizations`. A bridge test in the service pins
the contract.

## 21. Compatibility

Semantic versioning. Every re-export from `lib.rs` is part of the
contract — the actual list today: `Organization`, `OrgIdentifier`,
`IdentifierScheme`, `PostalAddress`, `MatchingEngine`, `MatchConfig`,
`MatchResult`, `MatchBreakdown`, `Confidence`, `Error`, `Result`.
`RelationshipRef`/`RelationKind` and the `relationships_score`/
`tags_score` `MatchBreakdown` fields are planned (§14a, §14b, §23), not
yet part of the public surface — do not assume a downstream consumer can
import them.

## 22. Anti-patterns

Do not short-circuit on classification codes or `Custom`. Do not score a
tax id across jurisdictions. Do not strip diacritics. Do not add IO,
async, or panics to library code.

## 23. Tasks (live work queue)

- [ ] Implement the relationships component in code: add
      `relationships: Vec<RelationshipRef>` to `Organization`, the
      `RelationshipRef { relation: RelationKind, organization_id }` +
      `#[non_exhaustive] RelationKind` (`SubOrganizationOf` /
      `ParentOrganizationOf` / `SuccessorOf` / `PredecessorOf`) types, the
      typed-set Jaccard `relationships_score` (§14a), the
      `relationships_weight` (default `0.05`, §7) in `MatchConfig`, and the
      `relationships_score: Option<f64>` field in `MatchBreakdown`; pin it
      with a `tests/public_api.rs` case.
- [ ] Implement the tags component in code: add `tags: Vec<String>`
      (default empty) to `Organization`, the plain set Jaccard
      `tags_score` over `fold_set(tags)` (§14b, `None` when either side
      empty), the `tags_weight` (default `0.05`, §7) in `MatchConfig`, and
      the `tags_score: Option<f64>` field in `MatchBreakdown`; pin it with
      a `tests/public_api.rs` case.
- [ ] Optional `phone`/`email` exact-match component.
- [ ] Consider a configurable legal-suffix list (currently a const).
- [ ] Address: postal-code exact-anchor boost.
- [ ] Split this single `spec/index.md` into the numbered §-per-file
      layout used by the sibling matcher crates.

## 24. Testing strategy

Unit tests embedded per module; an integration suite
(`tests/public_api.rs`) over the re-exported surface; a property-based
suite (`tests/property_tests.rs`, `proptest`, SEC-M6) proving the engine
and its pure helpers never panic on arbitrary UTF-8, scores stay bounded
and finite, matching is symmetric, and an identical clone self-matches;
a standalone `cargo-fuzz` harness (`fuzz/`, SEC-I2, nightly-only, not a
workspace member) coverage-fuzzing the same never-panic/bounded-score
invariants; rustdoc examples run as doctests. Run `cargo test`, `cargo
clippy --all-targets -- -D warnings`, `cargo fmt --check`.

## 25. Change control

Update this spec in the same PR as any behavioural change; bump
`CHANGELOG.md` under `[Unreleased]`.
