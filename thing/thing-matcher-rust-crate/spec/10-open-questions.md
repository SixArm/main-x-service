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

- **OQ-F (task, M, security) — Empty-value identity bypass on
  `Identifier` via direct construction/deserialization.**
  `Identifier::new` rejects an empty trimmed `property_id`/`value`, but
  `Identifier` is **not** `#[non_exhaustive]` and both fields are `pub`
  with `#[derive(Deserialize)]` — so `Identifier { property_id:
  "sku".into(), value: String::new() }` is constructible via a struct
  literal, and is likewise reachable via `serde_json` deserialization of
  untrusted input, entirely bypassing the constructor's guard.
  `shares_identifier` (`src/matcher.rs`) only checks
  `thing1.identifiers.is_empty() || thing2.identifiers.is_empty()`
  before comparing `id1 == id2` — two things each carrying one
  `Identifier { property_id: "sku", value: "" }` (e.g. from a JSON
  payload with `"value": ""`) satisfy that equality and spuriously trip
  `deterministic_match`, exactly the SEC-M2 false-identity class this
  crate already fixed for `same_canonical_url`/`shares_same_as` in
  0.7.0 (`CHANGELOG.md` "Security" entry) — but that fix never touched
  this construction-bypass path on `Identifier`, and `shares_same_as`
  in the same file already has the right pattern to copy: it skips
  entries whose normalised form is empty. The sibling `place-matcher`
  crate has the identical bypass on its own `PlaceId` type (see its own
  `spec/10-open-questions.md` OQ-I), so the fix pattern should land
  identically in both. *(verified: `sed -n '71,78p' src/models.rs`
  shows `Identifier`'s fields are `pub` with no `#[non_exhaustive]`,
  deriving `Deserialize`; `sed -n '829,841p' src/matcher.rs` shows
  `shares_identifier`'s only empty-guard is on the outer `Vec`, not
  per-entry.)* **Acceptance:** `shares_identifier` additionally skips
  any `Identifier` whose `value` (or `property_id`) is empty (mirroring
  `shares_same_as`'s empty-skip); a unit test constructs two
  `Identifier`s with `value: String::new()` directly (bypassing `new`)
  on both sides and asserts `deterministic_match` returns `false`;
  existing `identifiers_property_scoped_no_cross_match`-style tests
  stay green; `cargo test` + `cargo clippy --all-targets -- -D
  warnings` clean; `CHANGELOG.md` entry under "Security" (same class as
  the 0.7.0 SEC-M2 fix).

- **OQ-G (task, S, follow-up regression coverage for OQ-F) — Add a
  property test asserting no struct-literal-constructed `Identifier`
  can ever spuriously trip `deterministic_match`.** Once OQ-F above is
  fixed, a `proptest` property (this crate already has a `proptest`
  dev-dependency and property-test infrastructure — `spec/index.md`
  cites the property-test suite) generating arbitrary
  `Identifier { property_id, value }` pairs — including empty-string
  and whitespace-only `value`s that bypass `Identifier::new` — over
  arbitrary `Thing` pairs guards the fix from regressing silently the
  way the pre-0.7.0 URL bug did. *(verified: `ls tests/` and
  `grep -rn "proptest" Cargo.toml` confirm the crate already has
  property-test scaffolding to extend, rather than needing a new
  harness.)* **Acceptance:** a new `proptest!` block generates
  `Identifier` pairs (including the empty/whitespace-value bypass case)
  and asserts `deterministic_match` never returns `true` from a shared
  empty-valued identifier alone; `cargo test` clean; documented in the
  same PR as OQ-F's fix (not a separate follow-up PR, since an
  unguarded fix is exactly the situation invariant 2
  (`agents/share/security.md`) warns against).

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

