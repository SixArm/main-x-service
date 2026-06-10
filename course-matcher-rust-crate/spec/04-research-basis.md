## 4. Research basis

Approach mirrors the sibling matcher crates:

- **Name similarity:** Jaro-Winkler — proven on short titles, handles
  transpositions cheaply, prefix bonus matches catalog conventions
  where the leading discipline tag is preserved across variants
  ("Intro to CS" vs "Introduction to CS"). A capped Soundex bonus
  (§9) lifts near-homophone pairs that Jaro-Winkler alone scores just
  under the High band.
- **Set similarity:** Jaccard on the lowercased keyword / teaches
  sets — robust to ordering and exact-membership rather than
  fuzzy substring.
- **Renormalisation:** absent-component penalty was retired across
  the family on 2026-06-03 (post-Person Service fix). The matcher
  starts with the same convention.
- **Deterministic short-circuits:** DOI / Wikidata / OER / LOM / URI
  / UUID are globally unique by construction. `provider_id +
  course_code` is unique within a provider's catalogue.

Crate dependencies kept deliberately small:

- `strsim` for Jaro-Winkler.
- `unicode-normalization` for NFKC.
- `serde` + `serde_json` for round-trip.
- `thiserror` for the error enum.
- `mimalloc` is used **only** by the demo binary
  ([`src/main.rs`](../src/main.rs)) under a `musl` target gate; the
  library itself sets no global allocator.

A runnable demo binary (`cargo run`) exercises the public API with a
sequence of worked examples, mirroring the sibling matcher crates. It
is not part of the SemVer surface.

