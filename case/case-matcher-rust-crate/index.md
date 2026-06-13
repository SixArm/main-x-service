# case-matcher — documentation index

Pairwise governmental case-management record matching, deterministic +
probabilistic.

## Start here

| Doc | Purpose |
|---|---|
| [spec/index.md](./spec/index.md) | **Single source of truth** (§1–§25). |
| [AGENTS.md](./AGENTS.md) | How to work in this crate; public API; layout. |
| [README.md](./README.md) | User-facing intro + usage. |
| [CHANGELOG.md](./CHANGELOG.md) | Release history. |
| [AGENTS/matching-algorithm.md](./AGENTS/matching-algorithm.md) | Per-component derivations + weights. |
| [AGENTS/normalization.md](./AGENTS/normalization.md) | Fold / case-number / url rules. |
| [AGENTS/testing.md](./AGENTS/testing.md) | Test layout. |

## Worked example

```text
match_cases(
  { title: "Smith v. Housing Authority", identifiers: [Docket "CV-2024-001234"] },
  { title: "Appeal of benefit denial",   identifiers: [Docket "cv-2024-001234"] },
) -> score 1.0  (R-0 deterministic docket match)

match_cases(
  { title: "Housing benefit appeal — J. Smith",   subjects: ["person:pid-42"] },
  { title: "Housing benefit appeal — John Smith", subjects: ["person:pid-42"] },
) -> high score (title + subjects corroborate)
```

Part of the Main X Index family; embedded by
[case-service](../case-service-rust-crate).
