## 23. Tasks and Acceptance Criteria

Single source of truth for outstanding work; absorbs what an SDD workflow would otherwise put in a separate `tasks.md`. Tasks are tagged `T-NN`. Status legend: `[ ]` open, `[~]` in progress, `[x]` done. Delivered tasks archived in [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md) (T-1..T-16 + §23.1 changelog roll-up) and [`AGENTS/delivered-tasks-2.md`](../AGENTS/delivered-tasks-2.md) (T-17..T-32 + project-level acceptance criteria). Only open tasks below.

### 23.1 Open tasks

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as optional dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default preserves current behaviour exactly.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `Normalizer::phonetic_code(name, encoder)` (additive overload; no-encoder form retained for backward compat).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Define + test multi-code comparison semantics for Daitch-Mokotoff (FR-22a candidate): non-empty code-set intersection → `1.0`; single-name match → `0.5`; disjoint → `0.0`.
- **Acceptance:** Default-config behaviour and existing tests unchanged. New unit tests cover Double Metaphone primary/secondary equality (`Stephen`/`Steven`) and Daitch-Mokotoff Slavic-cluster equality (`Schwarz`/`Shvarts`). Documented "opt-in only" until T-9's corpus methodology is run.

**T-17.1 (residual).**
- [ ] TSV rows in `AGENTS/national-worker-identifiers.tsv` for the 7 FR-85..FR-91 schemes (parsers shipped without their TSV rows).

---

