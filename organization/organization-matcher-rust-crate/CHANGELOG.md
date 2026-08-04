# Changelog

All notable changes to this crate are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> See also: [spec/index.md](./spec/index.md) — single source of truth;
> [README.md](./README.md) — user-facing intro; [AGENTS.md](./AGENTS.md) — agent guide.

## [Unreleased]

### Fixed — doc accuracy pass (DOC-3)

- **`spec/index.md` §5/§6/§7/§14a/§14b/§21 described the planned
  relationships/tags components in the present tense as if already
  implemented**, contradicting §23's own (correct) `[ ]` task entries
  for them within the same file — `relationships`/`tags` do not exist on
  `Organization`, `relationships_weight`/`tags_weight` do not exist on
  `MatchConfig`, and `relationships_score`/`tags_score` do not exist on
  `MatchBreakdown` (confirmed by reading `src/organization.rs`,
  `src/config.rs`, `src/scoring.rs`, and `src/lib.rs`'s re-export list —
  none mention them). Marked each section "planned, not yet
  implemented" with a pointer to §23, and corrected §21's compatibility
  list to the actual `lib.rs` re-exports. No code or test behaviour
  changed; this only fixes the spec's internal self-contradiction.
- **§24 Testing strategy named only the unit tests, `tests/public_api.rs`,
  and doctests** — missing the `proptest` property suite
  (`tests/property_tests.rs`, SEC-M6) and the `cargo-fuzz` harness
  (`fuzz/`, SEC-I2), both already landed earlier in this same
  `[Unreleased]` section. Added both; mirrored into
  `AGENTS/testing.md`, which had the identical gap.
- Confirmed **SEC-M5** (LEI ISO 7064 MOD 97-10 / GLN GS1 mod-10
  check-digit validation) lives entirely in
  `organization-service-with-loco`'s `src/validation.rs` — this crate
  has no check-digit validators at all (`tests/property_tests.rs`'s own
  module doc says so explicitly). Nothing to change here; noted so a
  future pass doesn't go looking for it in the matcher.

### Added — cargo-fuzz harness (SEC-I2)

- A `fuzz/` [`cargo-fuzz`](https://rust-fuzz.github.io/book/) crate adopting
  the person-matcher reference scaffolding, with two coverage-guided
  libFuzzer targets: `match_organizations` (deserialize a JSON `[organization_a, organization_b]` tuple →
  `MatchingEngine::match_organizations`; finite score in `[0,1]`, both orders) and
  `normalize` (the pure `normalize` free functions — fold / legal name / domain / fold-set — over arbitrary
  UTF-8, never-panic). Two targets rather than the reference three because
  this crate exposes its similarity primitives only through the engine (the
  `scoring` module publishes no string-similarity functions). Run on nightly:
  `cargo +nightly fuzz run <target>` (see `fuzz/README.md`). The `fuzz/` crate
  is standalone (not a workspace member), so it never affects the crate’s
  normal stable build/test/clippy. Verified: `cargo +nightly fuzz build`
  compiles both targets and short campaigns run clean (millions of execs, no
  panics).

### Added

- **Property-based tests (SEC-M6).** Added `proptest` (dev-dependency,
  `1.11`) and `tests/property_tests.rs` proving the matcher and its pure
  helpers are robust on arbitrary UTF-8: the match engine (plus
  `match_one_to_many` / `rank` / `find_matches`) and the pure helpers
  (`normalize::{fold,legal_name,domain,fold_set}`, `phonetic::{soundex,same}`,
  `IdentifierScheme::is_deterministic`) **never panic**; `score` is always
  in `[0.0, 1.0]` and never NaN; matching is **symmetric** (score,
  `is_match`, `confidence`, deterministic flag); an identical clone of a
  well-formed organization **self-matches** (score ≥ threshold); `soundex`
  returns `Some` iff an ASCII-alpha anchor is present (code always four
  chars); and `Confidence::classify` is monotonic. Tests + dev-dep only —
  no library behaviour, weight, or threshold change.

- **Relationships component (spec-only; code follow-up tracked in spec §23).**
  Specced a typed organization-to-organization relationship signal:
  `relationships: Vec<RelationshipRef>` on `Organization`,
  `RelationshipRef { relation: RelationKind, organization_id }` with a
  `#[non_exhaustive] RelationKind` (`SubOrganizationOf` /
  `ParentOrganizationOf` — schema.org subOrganization / parentOrganization
  inverses; `SuccessorOf` / `PredecessorOf` — merger / rename / reorg
  inverses), a typed-set **Jaccard** `relationships_score` over
  `(relation, organization_id)` pairs (§14a, `None` when either side is
  empty), and a **supporting** `relationships_weight` (default `0.05`, §7)
  folded into the renormalised weighted average. Supporting signal only —
  shared references never single-handedly establish a match.

### Changed

- **Doc harmonization pass.** Corrected `AGENTS/spec-driven-development.md`,
  which carried unadapted course-matcher residue: the "When To Update
  Which Section" table now maps §10 Address, §11 URL/domain, §12
  Jurisdiction, §13 Founding date, §14 Keywords (was course-matcher's
  organization-code / provider / educational-level rows); the
  CS101/`provider_id` anti-pattern was replaced with the
  organization-relevant TaxId-jurisdiction-gate (§15–§16) anti-pattern;
  and stale bare `spec.md` references were updated to `spec/index.md`.
- **index.md worked example.** Added a probabilistic multi-component
  walkthrough with a full numeric breakdown (name + address + url +
  founding date + keywords, renormalised → 0.849 / `Medium`).

### Added

- **Tests for previously-uncovered spec behaviour.** (1) Soundex bonus
  path through `name_score` (§9) — confirms the +0.05 lift on a
  Soundex-equal near-miss whose Jaro-Winkler is below the ceiling;
  (2) suffix-only names (`"The Co"` vs `"Inc"`) do not spuriously score
  ~1.0 via the `legal_name` never-empty fallback (§8); (3) one-sided
  `set_jaccard` keywords case returns `Some(0.0)` (§14).

- **Inaugural release (v0.1.0).** Pairwise organization-record matching
  modelled on schema.org/Organization, copy-adapted from the
  course-matcher template.
  - Domain model: `Organization` (name / legalName / alternateName /
    identifiers / url / sameAs / address / jurisdiction / foundingDate /
    keywords), `OrgIdentifier`, `IdentifierScheme`, `PostalAddress`.
  - **Deterministic short-circuits**: R-0 globally-unique identifiers
    (LEI, DUNS, ISO 6523, GLN, Wikidata, ROR, ISNI, VAT); R-1
    same-jurisdiction tax id; R-2 `same_as` URL overlap. Classification
    codes (NAICS/ISIC/SIC) and `Custom` never short-circuit.
  - **Probabilistic components**: name (legal-suffix-aware Jaro-Winkler
    + Soundex bonus), postal address (weighted field-by-field), url /
    domain, jurisdiction, founding date (by year), keywords (Jaccard) —
    renormalised over present components.
  - `MatchConfig` weights (name 0.35, address 0.20, url 0.15,
    jurisdiction 0.10, founding_date 0.10, keywords 0.10; threshold
    0.85) with `strict()` / `lenient()` presets.
  - Normalisation: `fold`, `legal_name` (legal-suffix stripping),
    `domain` (URL→registered domain), `fold_set`.
  - 45 embedded unit tests + a 11-test public-API integration suite +
    7 doctests. Green `cargo build`, clippy clean, rustfmt formatted.
