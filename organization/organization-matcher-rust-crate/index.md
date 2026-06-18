# organization-matcher — documentation index

Pairwise organization-record matching (deterministic + probabilistic)
per [schema.org/Organization](https://schema.org/Organization).

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) | Per-component derivations + weights. |
| [AGENTS/normalization.md](./AGENTS/normalization.md) | Fold / legal-name / domain rules. |
| [AGENTS/testing.md](./AGENTS/testing.md) | Test layout. |

## Worked example

```text
match_organizations(
  { name: "Globex", identifiers: [Lei "5493001KJTIIGC8Y1R12"] },
  { name: "Globex International Holdings", identifiers: [Lei "5493001KJTIIGC8Y1R12"] },
) -> score 1.0  (R-0 deterministic LEI match)

match_organizations(
  { name: "Acme, Inc." },
  { name: "ACME Corporation" },
) -> score ~1.0  (both legal-normalise to "acme")
```

### Probabilistic multi-component breakdown

No deterministic rule fires here, so the engine renormalises a weighted
average over the *present* components (§17). Absent components
(`jurisdiction` here) drop out of both numerator and denominator.

```text
match_organizations(
  { name:    "Acme Robotics",
    url:      "https://acme-robotics.com",
    address:  { locality "Boston", region "MA" },
    founding: "1998",
    keywords: ["robotics", "ai"] },
  { name:    "Acme Robotic Systems",
    url:      "https://acmerobotics.com",
    address:  { locality "Boston", region "MA" },
    founding: "1999",
    keywords: ["robotics", "automation"] },
)
-> score 0.849  Confidence::Medium  is_match=false  (threshold 0.85)

   component      score   weight   contribution
   name           0.950   0.35     0.3325   (JW ~0.90 + Soundex bonus, capped 0.95)
   address        1.000   0.20     0.2000   (locality + region agree, case-folded)
   url            0.988   0.15     0.1482   (domains differ by one hyphen → JW)
   jurisdiction    —       —        —       (absent on both → skipped)
   founding_date  0.500   0.10     0.0500   (1998 vs 1999 → one year off)
   keywords       0.333   0.10     0.0333   (1 shared of 3 distinct → Jaccard)
   ─────────────────────────────────────────
   weighted sum = 0.7641 / present-weight 0.90 = 0.849
```

Part of the Main X Index family; embedded by
[organization-service](../organization-service-with-loco).
