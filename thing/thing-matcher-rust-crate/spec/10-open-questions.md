## 10. Open questions

- **OQ-A — Soundex vs. Metaphone for non-English names.** Soundex was designed for English surnames and is known to be weak for many non-English orthographies. Should `MatchConfig` gain a `phonetic_encoder` enum (Soundex / Double Metaphone / NYSIIS)? Decision deferred until a multilingual evaluation corpus is available.
- **OQ-B — Cross-scheme identifier resolution.** Should the crate ship an opt-in helper that recognises `(isbn, 0-201-89683-4)` and `(isbn, 9780201896831)` as the same identifier under ISBN-10 ↔ ISBN-13 canonicalisation? Today's stance: keep canonicalisation upstream and out of this crate.
- **OQ-C — Per-scheme identifier weights.** Some `property_id` values (`"isbn"`, `"doi"`, `"gtin"`) are globally unique by construction; others (`"sku"`, `"mpn"`) are not. Should the matcher tag schemes as "globally unique" and treat shared values in that bucket as a stronger signal? Today: every shared `(property_id, value)` pair short-circuits to `deterministic_match = true` regardless.
- **OQ-D — `description` vs. `disambiguating_description` interaction.** When both fields are present on both sides, the score includes both contributions independently. Should `disambiguating_description` be promoted to a tie-breaker only? Today: both contribute via the standard weighted sum.
- ~~**OQ-E — Code follow-up for `relationships` / `tags` (spec-only).**~~ RESOLVED in v0.7.0 (2026-08-28, T-PRO-H7). §3.1, §3.3.1, §3.4, §3.7, §5.9.1, §5.9.2, §6.6, and §6.8 are now current behaviour: `crate::models` carries `relationships: Vec<RelationshipRef>` + `RelationshipRef`/`RelationKind` (the latter `#[non_exhaustive]`) and `tags: Vec<String>`, both `#[serde(default)]`; `crate::matcher` scores them as `relationships_score`/`tags_score` on `MatchBreakdown` (typed-set / plain Jaccard, `None` when either side is empty) weighted by the new `MatchConfig::relationships_weight` / `tags_weight` (default `0.05` each), folded into the renormalised weighted sum. Tests pin the Jaccard behaviour (identical / disjoint / partial-overlap / empty-either-side) and a renormalisation sanity check (absent on both sides ⇒ an otherwise-perfect match still scores `1.0`).

The following are grounded follow-up **tasks** rather than open design
questions — each has a concrete, verified gap and a clear direction, so
they are recorded here (this crate carries no `spec/13-tasks.md`
checklist — see `spec/index.md`'s table of contents) rather than left
purely as prose elsewhere.

- ~~**OQ-F (task, M, security) — Empty-value identity bypass on
  `Identifier` via direct construction/deserialization.**~~ —
  **RESOLVED (2026-09-04).** `Identifier::new` refuses an empty trimmed
  `property_id`/`value`, but `Identifier` is not `#[non_exhaustive]` and
  both fields are `pub` with `#[derive(Deserialize)]`, so a struct
  literal or `serde_json` deserialization of untrusted input bypassed
  the constructor's guard entirely. `shares_identifier`
  (`src/matcher.rs`) now skips any `Identifier` whose trimmed
  `property_id` or `value` is empty on either side — matching the
  constructor's own definition of "empty" — before comparing
  `id1 == id2`, closing the same false-identity class already fixed for
  `same_canonical_url`/`shares_same_as` in 0.7.0. Three unit tests pin
  the exact bypass shapes (empty `value`, empty `property_id`,
  whitespace-only `value`); OQ-G's property test (below) covers the
  general case. `cargo test` (106 lib tests, up from 103) + `cargo
  clippy --all-targets -- -D warnings` + `cargo doc --no-deps` all
  clean. See `CHANGELOG.md` "Security" under `[Unreleased]`.

- ~~**OQ-G (task, S, follow-up regression coverage for OQ-F) — Add a
  property test asserting no struct-literal-constructed `Identifier`
  can ever spuriously trip `deterministic_match`.**~~ — **RESOLVED
  (2026-09-04), same PR as OQ-F.** A new property,
  `deterministic_match_never_trips_on_a_shared_degenerate_identifier`
  (`tests/property_tests.rs`), generates arbitrary `Identifier`s whose
  `property_id`, `value`, or both trim to empty, attaches the same one
  to two otherwise-unrelated `Thing`s, and asserts `deterministic_match`
  is `false`. One design correction found while writing it: the first
  draft generated `p1`/`p2` independently from the existing
  `thing_strategy()` (which also randomises `url`/`same_as`) and failed
  — not from the identifier bypass, but because the strategy's small
  `same_as` alphabet let two independently-generated things coincidentally
  share a URL, a **real** `shares_same_as` match unrelated to this
  property. The test now clears `url`/`same_as` on both sides first, so
  the shared degenerate identifier is the only thing `deterministic_match`
  could possibly fire on. Confirmed the property genuinely exercises the
  OQ-F fix (not a tautology): reverting `shares_identifier`'s change
  makes it fail 1 of 13 property-test cases, immediately.

- **OQ-H (task, M) — Resolve OQ-C: tag globally-unique identifier
  schemes and weight them accordingly.** OQ-C above has stood as an
  open question describing today's behaviour (every shared
  `(property_id, value)` pair short-circuits identically, whether the
  scheme is `isbn`/`doi`/`gtin` — globally unique by construction — or
  `sku`/`mpn` — not). This is a real false-positive risk: two
  *different* products sharing a non-unique internal SKU across
  catalogues would deterministically match. *(verified: `grep -n
  "property_id" src/matcher.rs` shows `shares_identifier` compares
  every scheme identically, with no scheme-tier distinction anywhere in
  `src/`.)* **Acceptance:** a `GLOBALLY_UNIQUE_SCHEMES` const (or a
  `MatchConfig` field, consistent with the crate's existing
  configurability) lists scheme names known to be globally unique by
  construction — starting from the three OQ-C itself names (`isbn`,
  `doi`, `gtin`), extended as real usage confirms others; only a shared
  pair whose scheme is in that set short-circuits
  `deterministic_match`; a shared pair on a non-listed scheme still
  contributes to `identifiers_score` in the probabilistic sum but no
  longer forces `1.0`; unit tests cover both a globally-unique-scheme
  match (still deterministic) and a non-unique-scheme match (no longer
  deterministic, but still scores); `cargo test` + clippy pedantic
  clean; spec §5.1/§10 updated (this closes OQ-C — remove it from the
  list above once landed).

---

