## 21. Roadmap and Future Work

Near- and medium-term (0.2.x / 0.3.x) all shipped (T-1 / T-2 / T-3 / T-5 / T-6 / T-10 / T-22 / T-11 / T-24 / T-25). Per-task acceptance criteria in [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md).

### 21.1 Open initiatives (0.4.x – 1.0)

T-9.1 (opt-in `MatchConfig::phonetic_encoder` enum behind a Cargo feature flag); optional `match_many_to_many` / blocking-key helpers atop the delivered batch API; optional Fellegi-Sunter weight learning (training mode); async batch evaluation with `rayon` or `tokio`; further national identifier schemes (HK, SG, KR, TR, RU, AR, CA-provincial) incremental on consumer demand; 1.0 stabilisation (ratify API surface + freeze).

### 21.2 Declined and 21.3 Research Spike Outcomes

Full rationale + per-spike outcomes in [`AGENTS/roadmap-research.md`](../AGENTS/roadmap-research.md). Headline verdicts: T-17/T-17.1 grew identifier coverage to 42 schemes (further incremental on demand); T-9 keeps Soundex default + adds opt-in encoder enum (T-9.1); T-19 ships tactical 39-jurisdiction phone table + declines ITU-T full coverage / `phonenumber` dep / mobile-landline validation; T-14 declines external postal-address standardisation at this layer.

