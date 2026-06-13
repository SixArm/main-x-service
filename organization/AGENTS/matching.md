# Matching Reference — Organization Entity

Entity-level summary. The matcher crate is the canonical source —
detailed guides:
[`AGENTS/matching-algorithm.md`](../organization-matcher-rust-crate/AGENTS/matching-algorithm.md)
and [`AGENTS/normalization.md`](../organization-matcher-rust-crate/AGENTS/normalization.md);
normative spec: [matcher spec](../organization-matcher-rust-crate/spec/index.md)
§5–§18.

## Public API

```rust
use organization_matcher::{MatchConfig, MatchingEngine, Organization};

let engine = MatchingEngine::new(MatchConfig::default());
let r = engine.match_organizations(&a, &b);
// r.score: f64 in [0.0, 1.0]
// r.confidence: High | Medium | Low
// r.is_match: bool (score >= config.threshold)
// r.breakdown: per-component Option<f64> + deterministic_match flag
```

`engine.rank(&query, &candidates)` ranks a candidate slice — this is
what `POST /api/organizations/match` calls.

## Deterministic short-circuits (score pinned to 1.0)

| Rule | Condition |
|---|---|
| **R-0** | Shared value on a deterministic scheme: LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT |
| **R-1** | Shared non-empty `jurisdiction` AND a shared `TaxId` value |
| **R-2** | Any case-folded `same_as` URL overlap |

Never deterministic: `TaxId` across jurisdictions, classification
codes (`Naics` / `IsicV4` / `Sic`), `Custom`.

## Probabilistic components (defaults, sum 1.0)

| Component | Weight | Algorithm |
|---|---:|---|
| Name | 0.35 | Best Jaro-Winkler over the cross-product of `legal_name`-normalised name keys (`name` + `legal_name` + `alternate_names`); Soundex +0.05 bonus capped at 0.95 |
| Address | 0.20 | Field-by-field Jaro-Winkler (street 0.30, locality 0.25, postal 0.20, region 0.15, country 0.10), renormalised over fields present on both sides |
| URL / domain | 0.15 | Registered-domain equality → 1.0, else Jaro-Winkler on domains |
| Jurisdiction | 0.10 | Case-folded exact (1.0 / 0.0) |
| Founding date | 0.10 | Same year 1.0, ±1 yr 0.5, else 0.0 |
| Keywords | 0.10 | Jaccard on `fold_set` |

The weighted average runs only over `Some` components, renormalised —
absent data never drags the score down.

## Thresholds and confidence

| Setting | Value |
|---|---|
| `is_match` threshold (default) | 0.85 |
| `MatchConfig::strict()` | 0.95 |
| `MatchConfig::lenient()` | 0.70 |
| Confidence `High` | ≥ 0.95 |
| Confidence `Medium` | ≥ 0.70 |
| Confidence `Low` | below |

## Normalisation rules (do not violate)

- `fold` — trim + NFKC + lowercase. **Diacritics preserved**
  (`Müller` ≠ `Muller`).
- `legal_name` — fold + punctuation→space + strip legal-form suffix
  tokens (`Inc`, `Ltd`, `GmbH`, …) + collapse; never empty.
- `domain` — strip scheme, `www.`, path, port, userinfo.
- `fold_set` — fold + sort + dedupe (keywords, same_as).

## How the service invokes it

Both matching endpoints construct
`MatchingEngine::new(MatchConfig::default())` per request
([`src/controllers/organizations.rs`](../organization-service-rust-crate/src/controllers/organizations.rs)):

- `POST /api/organizations/match` → `engine.rank(query, candidates)`
  — pure, no DB.
- `POST /api/organizations/check-duplicates` → load active rows (cap
  1 000) → `to_org()` each → `match_organizations` → keep
  `is_match`, sort score-desc. No blocking yet — scaling task in
  entity [spec §13 T-7](../spec/13-tasks.md).

There is **no adapter**: stored payloads deserialise directly into
the matcher type. Config is not yet operator-tunable via the service
(no threshold parameter on the endpoints).

## Changing matching behaviour

Weights, threshold, suffix list, or rule changes are matcher-spec
changes (its §7 / §15–§16) **plus** the restated table in entity
[spec §6 FR-5](../spec/06-functional-requirements.md) — one PR, three
parts (spec + code + test). See the matcher's
[`AGENTS/spec-driven-development.md`](../organization-matcher-rust-crate/AGENTS/spec-driven-development.md).
