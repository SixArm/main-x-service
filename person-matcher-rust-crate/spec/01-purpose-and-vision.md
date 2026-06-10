## 1. Purpose and Vision

A reusable, transparent, auditable Rust library to determine whether two person demographic records refer to the same person. Targets identity exchange scenarios reconciling data and national-style identifiers from disparate source systems into a single best-guess decision.

**Vision.** Small, dependency-light, side-effect-free; combines deterministic and probabilistic matching; per-field `MatchBreakdown` on every score; configurable without sacrificing safe defaults; handles 42 national-identifier schemes (§6.4 / `AGENTS/national-person-identifiers.md`), a `PassportBook` model (multi-country / multi-book / time-varying), alphanumeric postcodes, international E.164 phones across 39 jurisdictions, diacritic-rich personal names; trustworthy for identity-adjacent workflows (audit trail, no silent fallbacks, no surprise IO).

**Non-Goals.** Persistent storage / databases / indexing; network calls / telemetry / background work; ML models; bulk pipelines beyond the delivered `match_one_to_many` / `rank_one_to_many`; cross-scheme identifier translation (requires a registry the library deliberately does not consult).

---

