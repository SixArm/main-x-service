# Matching — Worker entity

Two matching layers coexist; know which one your change belongs to
(and see [entity spec §16 OQ-4](../spec/16-open-questions.md) on the
long-term split).

## Layer 1 — in-service matcher

`worker-service-with-loco/src/matching/` — the service's own
`ProbabilisticMatcher` / `DeterministicMatcher` behind the
`WorkerMatcher` trait. Weighted components (name 0.30, birth date
0.25, gender 0.10, address 0.10, identifier 0.10, tax ID 0.10,
document 0.05), Soundex bonus, tax-ID / document short-circuits,
quality classes Definite ≥ 0.95 / Probable ≥ 0.85 / Possible ≥ 0.50.

→ Full reference (weights, rules, per-component scoring tables):
[service `AGENTS/matching.md`](../worker-service-with-loco/AGENTS/matching.md);
normative: [service spec §6.2](../worker-service-with-loco/spec/06-functional-requirements.md).

## Layer 2 — canonical matcher (the `worker-matcher` crate)

The reference algorithm, embedded by the service and re-exported as
`matcher_lib`. Carries what the in-service matcher does not:
42 national-identifier parsers (scheme-local), 9 passport-format
validators + `PassportBook` matching, nickname tables, NFKD
diacritic-correct normalisation, E.164 phone normalisation (39
countries), config presets (`strict` 0.95 / `default` 0.85 /
`lenient` 0.75 thresholds) and `strict_mode` semantics. Returns
`MatchResult { score, is_match, confidence: High|Medium|Low,
breakdown }`.

→ Algorithm detail:
[matcher `AGENTS/matching-algorithm.md`](../worker-matcher-rust-crate/AGENTS/matching-algorithm.md)
(carries verbatim spec §12);
normalisation: [matcher `AGENTS/normalization.md`](../worker-matcher-rust-crate/AGENTS/normalization.md);
configuration: [matcher spec §13](../worker-matcher-rust-crate/spec/13-configuration-specification.md).

## The bridge between layers

```rust
use worker_service::matching::adapter::to_matcher_worker;
use worker_service::matching::matcher_lib::{MatchingEngine, MatchConfig};

let engine = MatchingEngine::new(MatchConfig::default());
let result = engine.match_workers(&to_matcher_worker(&a), &to_matcher_worker(&b));
```

Routing rules: [entity spec §5.3](../spec/05-domain-model.md).
Pinning tests:
[`tests/duplicate_detection.rs`](../worker-service-with-loco/tests/duplicate_detection.rs).

## Ground rules

- Never cross-match national identifiers across schemes.
- Never change default weights / thresholds without updating the
  owning spec section (service §6.2 or matcher §13) + CHANGELOG.
- Every probabilistic result keeps its per-component breakdown all
  the way to the front-end match UI (entity FR-22).
- Matching must stay deterministic and explainable — audit
  defensibility of merge decisions depends on it (entity spec §12).
