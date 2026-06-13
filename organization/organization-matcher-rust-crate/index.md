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

Part of the Main X Index family; embedded by
[organization-service](../organization-service-rust-crate).
