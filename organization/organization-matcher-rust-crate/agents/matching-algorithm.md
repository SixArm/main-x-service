# Matching algorithm — organization-matcher

## Strategies

- **Deterministic** — three short-circuit rules pin the score to 1.0.
- **Probabilistic** — weighted average over the components both records
  carry; absent fields don't penalise.

## Deterministic short-circuits

| Rule | Condition |
|---|---|
| **R-0** | Both records share a value on a *deterministic* identifier scheme. |
| **R-1** | Both share a non-empty `jurisdiction` AND a `TaxId` value. |
| **R-2** | A `same_as` URL overlaps (case-folded). |

Deterministic schemes (`IdentifierScheme::is_deterministic`): `Lei`,
`Duns`, `Iso6523`, `Gln`, `Wikidata`, `Ror`, `Isni`, `Vat`. NOT
deterministic: `TaxId` (jurisdiction-scoped, R-1 only), `Naics` /
`IsicV4` / `Sic` (classification — sector, not identity), `Custom`.

## Probabilistic components

| Component | Default weight | Algorithm |
|---|---|---|
| Name | 0.35 | Best Jaro-Winkler over the cross-product of `legal_name`-normalised name keys (`name` + `legalName` + `alternateName`), + Soundex +0.05 bonus capped at 0.95. |
| Address | 0.20 | Weighted field-by-field Jaro-Winkler (street 0.30, locality 0.25, postal 0.20, region 0.15, country 0.10), renormalised over fields present on both sides. `None` if either side lacks an address. |
| URL / domain | 0.15 | Registered domain equal → 1.0, else Jaro-Winkler on the domains. `None` if either url absent. |
| Jurisdiction | 0.10 | Case-folded country equal → 1.0 else 0.0. `None` if either absent. |
| Founding date | 0.10 | Year equal → 1.0, ±1 → 0.5, else 0.0. `None` if either absent/unparseable. |
| Keywords | 0.10 | Jaccard on `fold_set(keywords)`. Skipped if both empty; `Some(0.0)` when exactly one side has keywords. |
| Relationships | 0.05 | Typed-set Jaccard on `(relation, organization_id)` pairs. `None` (skipped) when **either** side has no relationships. |
| Tags | 0.05 | Jaccard on `fold_set(tags)`. `None` (skipped) when **either** side has no tags — unlike Keywords above. |

The original six components sum to 1.0; `relationships`/`tags` are
additive supporting weights on top (declared total 1.10) — see spec §7
for why this does not push any score outside `[0.0, 1.0]`. The weighted
average is renormalised over the `Some` components only.

## Confidence band

`High` ≥ 0.95, `Medium` ≥ 0.70, `Low` < 0.70 — separate from
`MatchConfig::threshold` (drives `is_match`, default 0.85).

## Worked example

```rust
use organization_matcher::{Organization, IdentifierScheme, OrgIdentifier, MatchingEngine};

let engine = MatchingEngine::default_config();
let mut a = Organization::new("Globex");
let mut b = Organization::new("Globex International Holdings");
a.identifiers.push(OrgIdentifier { scheme: IdentifierScheme::Lei, value: "5493001KJTIIGC8Y1R12".into() });
b.identifiers.push(OrgIdentifier { scheme: IdentifierScheme::Lei, value: "5493001KJTIIGC8Y1R12".into() });
let r = engine.match_organizations(&a, &b);
assert_eq!(r.score, 1.0);                       // R-0 fires
assert!(r.breakdown.deterministic_match);
```
