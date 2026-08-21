## 9. Name similarity

- `jaro_winkler(fold(a.name), fold(b.name))` is the floor.
- Then try each `(alternate_names_a × b.name)` and `(a.name ×
  alternate_names_b)` and take the max.
- **Phonetic (Soundex) bonus.** When the running best is `< 0.95` and
  the two primary names produce the same Soundex code
  (`phonetic::same`), add `+0.05`, clamped to `0.95`:

  ```text
  if best < 0.95 && phonetic::same(fold(a.name), fold(b.name)):
      best = min(best + 0.05, 0.95)
  ```

  The `0.95` ceiling is deliberate: a phonetic hit nudges a
  Medium-band score upward but never single-handedly mints a
  High-confidence classification. Soundex is initial-letter-
  preserving, so the bonus only fires when both names share a first
  letter (`Smyth` ↔ `Smith` fires; `Catherine` ↔ `Katheryn` does
  not). Encoder details: [`src/phonetic.rs`](../src/phonetic.rs) and
  [`agents/matching-algorithm.md`](../agents/matching-algorithm.md).
- Final score is in `[0.0, 1.0]`. Never `None` (every Course has a
  `name`).

