## 22. Open Questions and Risks

Resolved questions (OQ-1..OQ-6) are archived in [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md) alongside the closing tasks (T-25 / T-11 / T-8 / T-3 / T-4 / T-13). Still open:

- **OQ-7** Should the phonetic bonus participate in `total_weight` only when applied (current behaviour) or always (skews the average down when phonetic is weak)? *Current behaviour is correct;* document explicitly.

### 22.1 Risks

Misuse as decision oracle (Med/High) → documentation + per-call `MatchBreakdown`. Diacritic-heavy false negatives (Med/Med) → NFKD + T-9.1 opt-in encoder. Spec/code drift (High/Med) → T-7 CI. Soundex over-clustering (Med/Low) → phonetic is bonus-only. `nhs-number` unmaintained (Low/Med) → pin minor + vendored fallback. Cross-scheme identifier confusion (Med/High) → scheme-local matching (FR-13 / §12.1). ES TSI lenient validation (Med/Low) — deliberate.

---

