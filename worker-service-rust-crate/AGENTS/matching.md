# Matching Algorithm Reference

## Overview

The MPI matching system compares two worker records and produces a confidence score (0.00–1.00). Two strategies are available: probabilistic (weighted fuzzy) and deterministic (rule-based exact).

## Probabilistic Matching

Computes a weighted sum of component scores.

### Component Weights

| Component  | Weight | Algorithm                                        |
| ---------- | ------ | ------------------------------------------------ |
| Name       | 0.30   | Jaro-Winkler + Levenshtein + Soundex phonetic    |
| Birth Date | 0.25   | Date proximity with tolerance                    |
| Gender     | 0.10   | Exact match / unknown handling                   |
| Address    | 0.10   | Weighted postal/city/state/street                |
| Identifier | 0.10   | Type + system + value match                      |
| Tax ID     | 0.10   | Exact match (deterministic short-circuit to 1.0) |
| Document   | 0.05   | Type + number match                              |

### Short-Circuit Rules

- Tax ID exact match → total score = 1.0
- Document exact match → total score = 0.98

### Match Quality Classification

| Quality  | Score Range                |
| -------- | -------------------------- |
| Definite | ≥ 0.95                     |
| Probable | ≥ threshold (default 0.85) |
| Possible | ≥ 0.50                     |
| Unlikely | < 0.50                     |

## Deterministic Matching

Rule-based scoring with short-circuit rules.

### Rules

| Rule    | Condition                                   | Points   | Short-Circuit? |
| ------- | ------------------------------------------- | -------- | -------------- |
| Rule 0  | Tax ID exact match                          | —        | Yes → 1.0      |
| Rule 1  | Exact identifier match                      | —        | Yes → 1.0      |
| Rule 1b | Exact document match                        | —        | Yes → 1.0      |
| Rule 2  | Name ≥ 0.90 AND DOB ≥ 0.95 AND Gender = 1.0 | 3 points | No             |
| Rule 3  | Address ≥ 0.80                              | 1 point  | No             |

Final score = total points / available points. Match threshold: 0.75 (3/4 rules).

## Name Matching

### Family Name

- **Jaro-Winkler similarity**: Case-insensitive, prefix bonus
- **Levenshtein distance**: Normalized by max length
- **Final**: max(Jaro-Winkler, Levenshtein)
- **Soundex bonus**: +0.05 if codes match and score < 0.95

### Given Names

- Pairwise Jaro-Winkler comparison
- Common variant recognition (William/Bill, Robert/Bob, etc.)
- Average of best matches

### Weights

- Family name: 0.5
- Given names: 0.4
- Prefix/suffix: 0.1

## Birth Date Matching

| Condition               | Score |
| ----------------------- | ----- |
| Exact match             | 1.0   |
| Day off by 1–2          | 0.95  |
| Month/day transposition | 0.90  |
| Same year and month     | 0.80  |
| Year off by 1           | 0.85  |
| Same year only          | 0.50  |
| Both None               | 0.0   |

## Gender Matching

| Condition         | Score |
| ----------------- | ----- |
| Exact match       | 1.0   |
| Either is Unknown | 0.5   |
| Mismatch          | 0.0   |

## Address Matching

Best-pair matching across address lists. Per-address score:

| Component      | Weight |
| -------------- | ------ |
| Postal code    | 0.30   |
| City           | 0.20   |
| State          | 0.20   |
| Street (line1) | 0.30   |

Postal code scoring: exact = 1.0, first 5 digits = 0.95, first 3 = 0.70.
Street normalization: abbreviation expansion (St.→Street, Ave→Avenue, etc.)

## Identifier Matching

- Must match on `identifier_type` and `system`
- Exact value match: 1.0
- Formatting difference only: 0.98
- Returns max score across all pairs

## Tax ID Matching

- Uses `Worker::effective_tax_id()` (tax_id field or TAX-type identifier)
- Normalized comparison (digits only)
- Exact match: 1.0, else 0.0

## Document Matching

- Must match on `document_type`
- Exact number match: 1.0
- Same number, different country: 0.95
- Returns max score across all pairs

## Phonetic Matching (Soundex)

4-character code: first letter + 3 digits.

| Letters                | Code    |
| ---------------------- | ------- |
| B, F, P, V             | 1       |
| C, G, J, K, Q, S, X, Z | 2       |
| D, T                   | 3       |
| L                      | 4       |
| M, N                   | 5       |
| R                      | 6       |
| A, E, I, O, U, H, W, Y | ignored |

Examples: Smith → S530, Smyth → S530, Robert → R163

Applied as a bonus (+0.05) when Soundex codes match and name score < 0.95.

## Source Files

- `src/matching/mod.rs` — Matcher traits, MatchResult, ProbabilisticMatcher, DeterministicMatcher
- `src/matching/algorithms.rs` — All algorithm implementations
- `src/matching/scoring.rs` — ProbabilisticScorer, DeterministicScorer
- `src/matching/phonetic.rs` — Soundex implementation
