# Roadmap research spike outcomes

This file archives the research-spike outcomes that were previously inlined in `spec.md §21.4`. Each entry remains reference-grade documentation of a closed decision, sourced verbatim. The spec keeps a one-line pointer to this file. See [`spec.md`](../spec.md) §21 for the live roadmap.

### 21.4 Research Spike Outcomes

#### T-17 — More national identifier schemes: **survey complete; recommended next batch is the 7 phone-table-covered jurisdictions without an identifier parser**.

**Question.** Which national workeral / healthcare identifier schemes should be added next, beyond the 35 schemes already shipped (UK NHS, FR NIR, ES TSI, IE IHI, UK NI H&C, US SSN, AU IHI, DE KVNR, IT CF, NL BSN, SE Workernummer, UK Scotland CHI, BE NN, BG EGN, CZ RČ, DK CPR, EE IK, ES DNI, FI HETU, HR OIB, IS KT, LT AK, LV PK, MT ID, NO FNR, PL PESEL, RO CNP, SI EMŠO, SK RČ, UK NINO, GR DSS, LI ID, NL ID, PL NIP, PT NIF)?

**Sample size and prioritisation principle.** The crate already covers **39 phone jurisdictions** (post-T-19) and **35 national identifier schemes** across 33 distinct jurisdictions. The natural next batch is the **7 jurisdictions where the crate parses phones but not identifiers** — that closes the symmetry between the two surfaces and unlocks `deterministic_match` for worker populations the crate already invests in elsewhere.

**Gap (phone table ✅, identifier ❌).**

| Jurisdiction | Phone | Identifier candidate | Format | Check-digit algorithm |
|---|---|---|---|---|
| Brazil | `+55` | **CPF** (*Cadastro de Pessoas Físicas*) | 11 digits, often formatted `NNN.NNN.NNN-DD` | Two check digits. Digits 1–9 weighted `10..=2`, sum mod 11; if < 2 result is 0 else `11 − result`. Digits 1–10 weighted `11..=2`, same rule. |
| China | `+86` | **Resident Identity Card** (*居民身份证*) | 18 chars: 17 digits + check digit (digit or `X`) | Weighted sum of first 17 digits (weights `7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2`), mod 11; lookup `'1','0','X','9','8','7','6','5','4','3','2'`. |
| India | `+91` | **Aadhaar** | 12 digits | Verhoeff (table-driven multiplication-permutation algorithm — well-documented, ~50 LOC). Last digit is the check digit. |
| Japan | `+81` | **My Number** (*マイナンバー*) | 12 digits | Weighted sum of first 11 digits (weights `6,5,4,3,2,7,6,5,4,3,2`), mod 11; check digit = `0` if mod < 2 else `11 − mod`. |
| Mexico | `+52` | **CURP** (*Clave Única de Registro de Población*) | 18 chars: 4 letters + 6 digits (YYMMDD) + sex + state + 3 consonants + homonym + check digit | Structural validation; date-of-birth substring sanity check; check digit is a Mod-10 weighted sum over the 17-char base. |
| New Zealand | `+64` | **NHI** (National Health Index) | 7 alphanumeric (`AAANNNN` or `AAANNNC` legacy / newer 7-char form) | Letter-to-number lookup + weighted sum mod 11; the original NHI uses one check digit, the new 2019 NHI uses a different structure. Two schemes worth distinguishing. |
| South Africa | `+27` | **ID Number** | 13 digits encoding YYMMDD + sequence + citizenship + race-legacy + Luhn check | Luhn over the first 12 digits. Also encodes DOB and gender (sequence ≥ 5000 → male). |

**Decision matrix.**

| Option | Coverage gain | Build cost (incl. check-digit research + tests) | Per-scheme risk | Verdict |
|---|---|---|---|---|
| Stay at 35 | Baseline | — | — | Asymmetric vs phone table; the symmetry argument favours the +7. |
| **Add the 7-jurisdiction batch** | 35 → 42 schemes; closes phone/identifier symmetry | ~7 × the per-scheme effort previously demonstrated by T-23/T-27/T-28 (one parser + builder setter + weight + breakdown + deterministic-match branch + tests). Each parser is ~40-60 LOC. | Mid. Verhoeff (IN Aadhaar) and CURP's structural validation are the more complex parsers; the rest are weighted-Mod-N variants the codebase already supports many of. | **Recommended.** |
| Add the 7 batch + further jurisdictions (HK, SG, KR, TR, RU, AR, CA-provincial) | 42 → ~50+ | Each new scheme is a similar slice of work; some (KR RRN, CA-provincial) carry significant privacy / political sensitivity | Mixed — CA Health Card is per-province (10+ formats); KR RRN has historical use restrictions | **Deferred** — pick up incrementally based on consumer demand. The 7 batch is the highest-value tranche. |
| Use a generic "national-id" parser dependency | Universal | Heavy dep | High — vendor data quality varies | **Declined** by the same reasoning as T-14 / T-19: the worker-matcher crate is the wrong layer for a kitchen-sink dataset. |

**Per-scheme parser sketch.**

```text
parse_br_cpf("123.456.789-09") -> Option<String>
    1. Keep ASCII digits → require exactly 11.
    2. Reject all-equal sequences ("11111111111" is technically valid mod-11 but is sentinel data).
    3. Compute D1: Σ(d[i] × (10 − i)) for i ∈ 0..9; mod 11; if < 2 → 0 else 11 − mod.
    4. Compute D2: Σ(d[i] × (11 − i)) for i ∈ 0..10; mod 11; if < 2 → 0 else 11 − mod.
    5. Verify d[9] == D1 and d[10] == D2.
    6. Return Some("12345678909") (canonical 11-digit form).

parse_cn_rrn("11010519491231002X") -> Option<String>
    1. Keep ASCII alphanumeric → require exactly 18; require first 17 to be digits, 18th to be digit or 'X'/'x'.
    2. Substring validation: chars 6..14 form a valid YYYYMMDD date (chrono::NaiveDate::parse_from_str).
    3. Compute Σ(d[i] × WEIGHTS[i]) for i ∈ 0..17, mod 11; lookup CHECK_CHARS table.
    4. Verify input[17].to_uppercase() == expected.
    5. Return Some("11010519491231002X") (canonical, X uppercased).

parse_in_aadhaar("234123412346") -> Option<String>
    1. Keep ASCII digits → require exactly 12.
    2. Reject all-equal sequences and a small known-test prefix list (e.g. test vectors starting "0", "1" are blacklisted per UIDAI guidance; details: spec link below).
    3. Verhoeff: maintain `c = 0`; for i ∈ 0..12, `c = D[c][P[(i % 8)][d[11 − i]]]`; valid iff `c == 0`.
    4. Return Some("234123412346").

parse_jp_my_number("123456789018") -> Option<String>
    1. Keep ASCII digits → require exactly 12.
    2. Compute Σ(d[i] × WEIGHTS[i]) for i ∈ 0..11 with WEIGHTS = [6,5,4,3,2,7,6,5,4,3,2]; mod 11; check = 0 if < 2 else 11 − mod.
    3. Verify d[11] == check.
    4. Return Some("123456789018").

parse_mx_curp("HEGG560427MVZRRL04") -> Option<String>
    1. Strip whitespace and uppercase → require exactly 18.
    2. Structural regex: ^[A-Z][AEIOUX][A-Z]{2}\d{6}[HM][A-Z]{5}[A-Z\d]\d$
    3. Date substring chars 4..10 forms a valid YYMMDD date (chrono).
    4. Compute Mod-10 weighted check digit over chars 0..17 using A=10, B=11, …, ñ=18.
    5. Verify input[17] == check.
    6. Return Some("HEGG560427MVZRRL04").

parse_nz_nhi("ZAA0073") -> Option<String>
    1. Keep ASCII alphanumeric uppercased → require exactly 7.
    2. Detect format generation: 3 letters + 4 digits (original) vs new 2019 format.
    3. For original: convert letters to integers (A=1..Z=26 minus excluded I, O), Σ(d[i] × WEIGHTS[i]) with WEIGHTS = [7,6,5,4,3,2,1]; mod 11; check = '0'..='9' or 0 = invalid.
    4. Return Some("ZAA0073").

parse_za_id("9001015009087") -> Option<String>
    1. Keep ASCII digits → require exactly 13.
    2. Date validity: substring chars 0..6 forms a valid YYMMDD date.
    3. Luhn over the first 12 digits; check digit at position 12 must agree.
    4. (Optional secondary signal exposed to consumers, NOT used by worker-matcher: chars 6..10 < 5000 → female, ≥ 5000 → male; char 10 = citizenship.)
    5. Return Some("9001015009087").
```

**Implementation note.** Per the project's per-scheme pattern (T-16 / T-23 / T-27 / T-28 / T-30): each scheme requires a parser + `Worker` field + `WorkerBuilder` setter + `MatchConfig::<cc>_<scheme>_weight` (default `0.30`) + `MatchBreakdown::<cc>_<scheme>_score` with `#[serde(default)]` + `MatchingEngine::calculate_breakdown` / `calculate_weighted_score` / `deterministic_match` wiring + `Worker::validate` extension + per-scheme unit + integration tests. The 3 external `MatchConfig` literal sites must be updated. Demo in `examples/basic_usage.rs`.

**Risk and mitigation.**

- **Sentinel data.** Several schemes have known structurally-valid-but-policy-blocked test vectors (e.g. BR CPF `11111111111`, IN Aadhaar prefix-restricted ranges). The parsers SHOULD reject these explicitly so they cannot become production matches.
- **PII sensitivity.** CN RRN, KR RRN, ZA ID encode date-of-birth and demographic information in the number itself. The crate already treats identifiers as opaque strings at match time, so this doesn't change the matching contract — but consumers should be aware that storing these values is more disclosure than storing a UK NHS Number.
- **Format generation drift.** NZ NHI (2019 revision), CN RRN (1999 18-digit reform vs older 15-digit form), and CURP (2010 revision) all have generation-pair concerns. Each parser SHOULD accept the current generation and document the legacy form's behaviour.
- **Spec links for parser implementers.** See AGENTS/national-worker-identifiers.tsv as the canonical reference for the broader 35-scheme set; the next-batch jurisdictions will get TSV rows added when each parser ships.

**This recommendation closes T-17.** The implementation of the 7-batch is tracked as **T-17.1** in §23.2; it follows the per-scheme pattern established by T-23 / T-27 / T-28.

#### T-9 — Locale-aware phonetic encoder: **add an opt-in `PhoneticEncoder` enum; defer the default-switch decision until an empirical corpus exists**.

**Question.** Should `Normalizer::phonetic_code` be replaced or augmented with a locale-aware encoder (Double Metaphone, NYSIIS, Daitch-Mokotoff, Beider-Morse, or a custom encoder) that handles diacritic-heavy and non-English names better than American Soundex?

**Current state (baseline).**

- `Normalizer::phonetic_code` wraps `soundex::american_soundex` (the `soundex` crate, v0.2). One 4-character code per name, tuned for English-language US census names.
- `MatchingEngine::score_phonetic_names` averages the given-name and family-name Soundex equality. Three outcomes: `0.0`, `0.5`, `1.0`.
- Per FR-22/FR-23 the phonetic component is a **bonus only**: it adds a `0.05`-weighted contribution when the score exceeds `0.9`, and never lowers the overall match score. The fail-open design caps the worst-case false-positive impact of any encoder change.

**Sample size and corpus.** This spike does **not** include an empirical evaluation: a representative multinational worker corpus with ground-truth (name, alt-spelling, is-same-worker) triples is not in the repo and would need to be sourced under appropriate clinical-data governance before any default-encoder switch could be justified. The recommendation below makes the corpus requirement explicit and proposes the methodology a future empirical evaluation should use.

**Decision matrix.**

| Encoder | Strengths | Weaknesses | Rust crate | Verdict |
|---|---|---|---|---|
| **American Soundex** (current default) | Tiny, fast, well-understood; deterministic; 0 external behaviour to validate | English-tuned; loses information on digraphs (`ph`/`f`), Slavic consonant clusters, vowel-rich Romance names; 4-character cap creates short-name collisions | `soundex` (in use) | Keep as the safe default. |
| **Double Metaphone** | Handles English digraphs and many Germanic / Romance phonemes; emits a primary + secondary code so ambiguous spellings have two chances to match | More complex; secondary-code use complicates the binary "codes equal?" comparison the matcher currently performs | `rphonetic` (Apache Commons Codec port), `metaphone` | **Recommended** as the first opt-in alternative. Best general-purpose upgrade for diacritic-rich and English-variant names. |
| **NYSIIS** | Retains more vowel information than Soundex; designed for English workeral names | English-tuned; not materially better than Soundex for non-English names | `rphonetic` | Skip — incremental over Soundex, not worth a third opt-in. |
| **Daitch-Mokotoff Soundex** | Purpose-built for Slavic, Yiddish, and Germanic surnames; handles consonant clusters that defeat both Soundex and Metaphone | Returns multiple codes per name; comparison semantics need spec-level clarification | `rphonetic` | **Recommended** as the second opt-in alternative for crates serving Eastern European / Ashkenazi worker populations. |
| **Beider-Morse** | Even stronger Slavic / Sephardi / Ashkenazi support; large rule set | Heavyweight; large rule tables; meaningful binary-size cost | `rphonetic` | Defer until empirical demand exists. |
| **Locale-specific** (Kölner Phonetik for DE, Soundex-FR for FR, …) | Best per-locale recall | Each locale is its own ruleset; matrix grows with worker population diversity; no single Rust crate covers them all | Various ad-hoc | Skip — fragments the API and the maintenance burden grows linearly with worker base. |
| **Custom encoder** | Could be tuned exactly to the 35-scheme jurisdictional spread | High research cost; needs the very corpus we don't have | n/a | Skip — re-implementing what `rphonetic` already ports from Apache Commons is wasted effort. |

**Recommended action.**

1. **Stay with American Soundex as the default.** Backward-compatible; preserves the FR-22/FR-23 contract that `score_phonetic_names` returns `{0.0, 0.5, 1.0}`; no surprise behaviour change for current consumers.
2. **Add a `MatchConfig::phonetic_encoder: PhoneticEncoder` enum** with at least three variants:
   - `Soundex` (default — current behaviour)
   - `DoubleMetaphone` — opt-in via `rphonetic`; documented as the best general-purpose upgrade for diacritic-rich and English-variant names
   - `DaitchMokotoff` — opt-in via `rphonetic`; documented for Slavic / Ashkenazi worker populations
3. **Refactor `Normalizer::phonetic_code(name)` → `Normalizer::phonetic_code(name, encoder)`** (additive overload; keep the no-encoder form for backward compat by delegating to `Soundex`). Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
4. **Define the comparison semantics for multi-code encoders** in spec §14.4: Daitch-Mokotoff emits multiple codes per name. Treat `score_phonetic_names` as `1.0` when the two name's code sets intersect non-trivially, `0.0` when disjoint, with the same `0.5` mid-case for "one name matches, the other doesn't" used for Soundex today. This keeps the FR-22/FR-23 score range unchanged.
5. **Defer the default-encoder switch** until an empirical evaluation can be run. The corpus and methodology that evaluation should use are specified below.

**Empirical-evaluation methodology (for the deferred default-switch decision).**

- **Corpus.** A labelled set of (`name_a`, `name_b`, `is_same_worker`) triples sourced from at least three jurisdictions: one English-majority (UK / US / IE / AU), one Romance (FR / IT / ES / PT), one Germanic (DE / NL / AT / CH), one Slavic (PL / CZ / SK / HR / SI), one Nordic (SE / NO / DK / FI / IS). Realistic sample size: ≥ 10,000 triples per jurisdiction, balanced 50/50 between match and non-match. The corpus MUST be obtained under appropriate clinical-data governance — synthetic corpora (e.g. faker-generated with deliberate spelling drift) are acceptable for an initial pass but not sufficient for a default-switch.
- **Metrics.** For each encoder, compute: true-positive rate at the FR-23 `> 0.9` threshold, false-positive rate, AUC of the encoder-only score against the ground-truth label, per-jurisdiction breakdown. Compare against the Soundex baseline.
- **Pass criteria** for switching the default: the candidate encoder must beat Soundex on TPR by ≥ 5 percentage points in **every** jurisdiction in the corpus, with FPR no higher than baseline. A win in one jurisdiction at the cost of another is not sufficient to switch the default — opt-in is the right tool for that case.
- **Failure handling.** If no encoder dominates Soundex in every jurisdiction, leave the default unchanged and rely on the opt-in mechanism; document the per-jurisdiction wins so consumers can pick deliberately.

**Risk and mitigation.**

- The phonetic component is a `0.05`-weighted bonus that **only lifts** scores (FR-23). The worst case of a bad opt-in encoder is "more false positives lifted into Medium confidence" — bounded, recoverable, and visible in the per-field `MatchBreakdown`.
- The `rphonetic` crate is pure Rust (preserves §17 portability) but pulls in a non-trivial dep tree. Gate it behind a Cargo feature flag (`phonetic-rphonetic`) so consumers who do not opt in pay zero compile-time / binary-size cost.

**What this spike shipped.** Documentation only (this section). No code change — see "Recommended action" above for the implementation plan, which is a small additive change but should be done together with the empirical-validation work to avoid shipping an opt-in that nobody can defensibly enable.

**This recommendation closes T-9.** The follow-up implementation task is filed separately as **T-9.1** under §23.2.

#### T-19 — Broader phone country table: **tactical expansion + decline the heavyweight dependency**.

**Question.** Should the supported phone country table (currently ~26 entries) be expanded to the full ITU-T E.164 country list, and should per-country mobile/landline prefix validation be added?

**Sample size.** Before T-19 the table held 26 entries. The crate exposes **35 national identifier schemes** across 33 distinct jurisdictions. The gap between the two — **13 jurisdictions with an identifier parser but no E.164 phone metadata** — was: BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK.

**Decision matrix.**

| Option | Coverage | Build / runtime cost | Maintenance | Recall lift for worker matcher | Verdict |
|---|---|---|---|---|---|
| Stay with 26 countries | Major worker-mobility partners | Already shipped | Low | Baseline | Insufficient — leaves 13 of the 35 identifier-scheme jurisdictions on the legacy `normalize_phone` fallback. |
| **Tactical expansion to 39 (cover every identifier-scheme jurisdiction)** | Every country the crate already parses an ID for | +13 static rows (~80 LOC), one struct-field refactor (`bool` → `Option<&str>` to support Lithuania's `8` trunk prefix), 6 new unit tests | Low (ITU dial codes and NSN bounds rarely change) | Closes the gap between the identifier surface and the phone surface — phones from any 35-scheme jurisdiction now canonicalise to E.164 directly | **Recommended.** Shipped as part of T-19. |
| Expand to all ~250 ITU-T E.164 territories | Universal | ~700 LOC of static rows + per-region NSN bounds research | Medium (some countries split / merge; mobile prefixes drift) | Marginal — most worker bases concentrate in ≤ 40 jurisdictions | **Declined.** Returns are sub-linear vs the maintenance burden, and the consumer can already use the legacy fallback for the long tail. |
| Depend on `phonenumber` crate (Rust port of Google's libphonenumber) | Universal + mobile/landline distinction + region inference | Adds ~10 transitive deps, ~MB binary, measurable compile-time cost | High (track upstream metadata releases) | Same as the 250-territory table for canonicalisation; the mobile/landline distinction doesn't help matching (see below) | **Declined.** Trade-off doesn't fit a minimal-API, clinical-safety-deterministic, `< 50 µs` library. |
| Add per-country mobile / landline prefix validation | n/a | Significant per-country research | High (telecoms regulation changes) | **None for matching.** The matcher's job is canonicalisation, not validation: two normalised forms either agree (`+44…` == `+44…`) or they don't. Mobile/landline classification is a data-labelling concern that doesn't change the comparison outcome | **Declined.** This is a consumer concern; if downstream services need to label phones, they SHOULD do it in their ingest pipeline. |

**Why decline `phonenumber`.** It is a fine Rust crate (pure Rust, no C deps, so it does *not* violate §17 the way libpostal does for T-14), but:

1. The marginal recall lift for worker matcher is small. With the tactical expansion, every jurisdiction the crate already parses an identifier for now reaches E.164 directly. The remaining recall problem is the tail of jurisdictions with no identifier parser — for which `phonenumber` would help, but the consumer is already a candidate for upstream normalisation in those cases (the same pattern recommended for T-14).
2. Compile-time and binary-size cost is real; the §17 performance budget (`< 50 µs` per pair) and the project's preference for a minimal dependency surface argue against pulling in a 250-territory metadata blob to canonicalise a `0.05`-weighted field.
3. The `phonenumber` API surface (region inference, mobile/landline classification, format-as-typed) overshoots the matcher's actual need (canonicalise + compare). A heavyweight dep for a fractional consumer of its features is a poor trade.
4. The same "standardise upstream" advice given for T-14 applies cleanly: consumers with truly global worker bases can run `phonenumber` (or libphonenumber) in their ingest pipeline and write the canonical `+CC…` form into `Worker::phone`, after which the matcher reads a number that already canonicalises identically on both sides.

**What this spike shipped (concrete changes).**

- Refactored `CountryPhoneInfo::has_trunk_prefix: bool` → `trunk_prefix: Option<&'static str>` to support non-`0` trunks (Lithuania uses `8`).
- Added 13 new entries to `COUNTRY_PHONE_TABLE`: BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK. Total coverage is now **39 jurisdictions**, every one of which corresponds to a national identifier scheme the crate already parses.
- Added 6 new unit tests pinning Lithuania's `8` trunk, Greece's no-trunk form, Romania's `0` trunk, Czech canonical form, Iceland's 7-digit NSN, and Croatia/Slovenia overlapping-3-digit-code disambiguation.

**Follow-up not in scope.**
- Mobile/landline prefix validation: declined as above. If a consumer needs it, do it upstream.
- Tail-of-the-world country coverage (the remaining ~210 ITU-T territories): can be added incrementally as worker-population demand surfaces. The table format is now extensible (`Option<&str>` trunk handles every documented trunk convention).

**This recommendation closes T-19.**

#### T-14 — External postal-address standardisation: **declined**.

**Question.** Should `worker-matcher` add (behind a feature flag) an integration with an external postal-address reference for address standardisation?

**Options surveyed.**

| Option | Quality | Build cost | IO / network | Licence | Verdict |
|---|---|---|---|---|---|
| **libpostal** (statistical, OSM-trained, C library) | Best-in-class | Heavy C dep; multi-GB runtime model file; complicates cross-platform builds (Windows, WASM) | None | MIT | **Ruled out by §17 portability axiom** (pure Rust, no C deps beyond `chrono`/`strsim` defaults). |
| **`postal` Rust bindings to libpostal** | Same as libpostal | Same as libpostal + unmaintained binding crate | None | MIT | **Ruled out for the same reasons.** |
| **Rust-native parsers** (`address-parser`, etc.) | Comparable to our in-house parser | Light | None | Permissive | **No incremental value** — none rivals libpostal, and any of them is roughly what `Normalizer::parse_address_line` already does (T-20). |
| **UK Royal Mail PAF / OS AddressBase** | Authoritative, UK-only | Light Rust glue | None (file-based dataset) | Licensed (PAF is paid; AddressBase is OGL-restricted for non-government) | **Ruled out** by jurisdiction scope (35 schemes supported, no single dataset covers them) and licence cost. |
| **US USPS, FR La Poste, DE Deutsche Post APIs** | Authoritative per jurisdiction | Light glue | **Network** | Per-country terms | **Ruled out** by jurisdiction scope. |
| **Commercial APIs** (Loqate, Smarty, FullContact, Google Address Validation) | High | Light glue | **Network** | Commercial | **Ruled out** by §17 (no IO) and §20 (PII MUST NOT leave the calling process without the consumer's explicit authorisation; the entire reason this crate is a pure library is so consumers control PII egress). |
| **Status quo** — `Normalizer::parse_address_line` + `expand_street_abbreviations` (T-20) | Adequate for the line-1 contribution | Already shipped | None | n/a | **Recommended.** |

**Recommendation. Do not integrate an external postal-address reference at this layer.**

The crate's value proposition is that it is a pure-Rust, IO-free, deterministic scoring library (§17, AGENTS/security-and-privacy.md). Every external standardisation option either:

- Adds a C dependency and a multi-GB runtime model file (libpostal), violating the portability axiom; or
- Adds network IO and PII egress (commercial APIs), violating the clinical-safety axiom; or
- Ties the crate to one jurisdiction (national datasets), violating the multinational scope (§6.4, 35 supported identifier schemes).

The address-line line-1 sub-component is weighted at only **0.2** within the address sub-score (which itself defaults to **0.05** of the overall match score per §13.1) — adding a heavyweight dependency for a fractional contribution is a poor architectural trade. The dominant address signals are postcode (0.5 of the sub-score) and city (0.3); neither benefits from external street-name standardisation.

**Consumers that need higher address-matching recall SHOULD standardise upstream**: run libpostal / a commercial API in the ingest pipeline, write the normalised values into the `Address` fields, and let `worker-matcher` score the standardised forms. `Address` is a plain struct that the consumer fills however they like — the crate is already well-positioned for this pattern.

**Incremental in-house improvements** worth tracking separately if recall complaints arrive (not part of T-14 acceptance):

- Locale-aware street-type vocabulary: today the `STREET_ABBREVIATIONS` table in `src/normalizer.rs` is English-only (`St → street`, `Rd → road`, …). Could be expanded to FR (`rue`, `av.` → `avenue`, `bd` → `boulevard`), DE (`str.` → `straße`), IT (`via`, `viale`), ES (`calle`, `av.` → `avenida`), NL (`straat`), etc. Small static table, no feature flag needed.
- An optional `uprn: Option<String>` (UK Unique Property Reference Number) or equivalent national property identifier on `Address`, scored like a national identifier (deterministic, scheme-local) for consumers who *do* run an external standardiser upstream. This would be additive and not blocked by T-14's "no" verdict.

**This recommendation closes T-14.** If recall data later contradicts the assumption that line-1 is a fractional signal, re-open and re-evaluate.

