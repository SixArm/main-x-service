## 23. Tasks and Acceptance Criteria

Tasks tagged `T-NN`; status `[ ]` open, `[~]` in progress, `[x]` done. Delivered tasks with full acceptance criteria are archived in [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md) (summary) and [`AGENTS/delivered-tasks-detail.md`](../AGENTS/delivered-tasks-detail.md). This section keeps only currently-open tasks.

### 23.1 Done (carried over from CHANGELOG)

Full list in [`AGENTS/delivered-tasks.md`](../AGENTS/delivered-tasks.md); covers the core engine (T-1..T-8 / T-13 / T-15), 42 identifier schemes + 9 passport-format validators (T-16 / T-21 / T-23 / T-27 / T-28 / T-17.1), 39-jurisdiction phone E.164 (T-18 / T-19), address parsing + `previous_addresses` (T-20 / T-24), nickname / middle-name / DOB-transposition / email scoring (T-10 / T-25 / T-22 / T-11), passport books / blood type / multi-birth / birth+death (T-26 / T-29 / T-30 / T-31 / T-32), benchmarks / property tests / drift CI / doc harmonisation (T-5 / T-6 / T-7 / T-12), and the T-9 / T-14 / T-17 / T-19 research spike outcomes.

### 23.2 Open tasks

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as an optional dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default preserves current behaviour.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `phonetic_code(name, encoder)` (additive overload).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Test multi-code semantics for Daitch-Mokotoff: non-empty code-set intersection → `1.0`, single-name match → `0.5`, disjoint → `0.0`.
- **Acceptance:** default-config behaviour and existing tests unchanged; new unit tests cover Double Metaphone (`"Stephen"`/`"Steven"`) and Daitch-Mokotoff (`"Schwarz"`/`"Shvarts"`); documented as opt-in only until T-9's corpus methodology is run.

### 23.3 Acceptance Criteria — Project-level

"1.0-ready" when all §21.1 tasks complete; spec and code agree (T-7 enforced); `Person` / `Address` `#[non_exhaustive]` (T-8); public API unchanged for two consecutive minor releases; coverage `≥ 90%` and `cargo test` in `< 5 s`.

---

