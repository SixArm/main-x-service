## 1. Purpose and Vision

A reusable, transparent, auditable Rust library to determine whether two worker demographic records refer to the same worker. Targets identity-exchange scenarios where demographic data and national-style identifiers from disparate source systems must be reconciled. Small, dependency-light, side-effect-free: combines deterministic + probabilistic matching; explainable per-field breakdowns; configurable; handles 42 national identifier schemes (see [`agents/national-person-identifiers.md`](../agents/national-person-identifiers.md)), passport books, alphanumeric postcodes, E.164 phone numbers across 39 jurisdictions, and diacritic-rich names; trustworthy for identity-adjacent workflows (no silent fallbacks, no surprise IO).

**Non-goals.** Persistent storage / databases / indexing; network calls / telemetry; ML or trained classifiers; bulk-pipeline / blocking; cross-scheme identifier translation.

---

