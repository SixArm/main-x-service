# Worker matcher — Living Specification

> **Status:** Living document. This is the canonical, spec-driven-development (SDD) specification for the `worker-matcher` Rust crate. It is the single source of truth: it consolidates what was historically split across `spec.md` (requirements), `plan.md` (design and implementation plan), and `tasks.md` (work breakdown). No separate `plan.md` or `tasks.md` exists; both are absorbed into the numbered sections below (see §9–§13 for plan content and §23 for tasks). When something changes in the codebase, this document changes first.
>
> **Version:** 0.3.0
> **Maintainer:** Joel Parker Henderson — `joel@joelparkerhenderson.com`
> **Crate:** `worker-matcher` (Cargo)
> **Edition:** Rust 2024
> **Licence:** MIT OR Apache-2.0 OR GPL-2.0 OR GPL-3.0 OR BSD-3-Clause
> **Repository:** https://github.com/sixarm/worker-matcher-rust-crate
>
> See also: [index.md](./index.md), [AGENTS.md](./AGENTS.md), [README.md](./README.md), [CHANGELOG.md](./CHANGELOG.md).

---

## Table of Contents

1. [Purpose and Vision](#1-purpose-and-vision)
2. [Scope](#2-scope)
3. [Stakeholders and Users](#3-stakeholders-and-users)
4. [Glossary](#4-glossary)
5. [Research Basis](#5-research-basis)
6. [Functional Requirements](#6-functional-requirements)
7. [Non-Functional Requirements](#7-non-functional-requirements)
8. [Domain Model](#8-domain-model)
9. [Architecture](#9-architecture)
10. [Component Specifications](#10-component-specifications)
11. [Public API Specification](#11-public-api-specification)
12. [Algorithm Specifications](#12-algorithm-specifications)
13. [Configuration Specification](#13-configuration-specification)
14. [Normalization Specification](#14-normalization-specification)
15. [Error Model](#15-error-model)
16. [Serialization Contract](#16-serialization-contract)
17. [Quality Attributes](#17-quality-attributes)
18. [Testing Strategy](#18-testing-strategy)
19. [Build, Tooling, and Release](#19-build-tooling-and-release)
20. [Security, Privacy, and Compliance](#20-security-privacy-and-compliance)
21. [Roadmap and Future Work](#21-roadmap-and-future-work)
22. [Open Questions and Risks](#22-open-questions-and-risks)
23. [Tasks and Acceptance Criteria](#23-tasks-and-acceptance-criteria)
24. [Change Control](#24-change-control)
25. [References](#25-references)

---

## 1. Purpose and Vision

### 1.1 Purpose

The `worker-matcher` crate provides a reusable, transparent, auditable Rust library to determine whether two worker demographic records refer to the same worker. It targets healthcare information exchange (HIE) scenarios where demographic data and national-style identifiers from disparate source systems must be reconciled into a single best-guess decision.

### 1.2 Vision

A small, dependency-light, side-effect-free library that:

- Combines **deterministic** (exact) and **probabilistic** (fuzzy) matching strategies.
- Produces **explainable** results — every score has a per-field breakdown.
- Is **configurable** without sacrificing safe defaults.
- Handles **multinational national identifiers** spanning 30 schemes — UK NHS Number, France NIR, España TSI, Éire IHI, UK NI H&C Number, US SSN, Australia IHI, Germany KVNR, Italy *Codice Fiscale*, Netherlands BSN, Sweden *Workernummer*, UK Scotland CHI Number, Belgium NN, Bulgaria EGN, Czech *Rodné číslo*, Denmark CPR, Estonia *Isikukood*, Spain DNI/NIE, Finland HETU, Croatia OIB, Iceland *Kennitala*, Lithuania *Asmens kodas*, Latvia *Workeras kods*, Malta National ID, Norway *Fødselsnummer*, Poland PESEL, Romania CNP, Slovenia EMŠO, Slovakia *Rodné číslo*, UK NINO — plus a `PassportBook` model for multi-country / multi-book / time-varying passport data, **alphanumeric postcodes**, **international (E.164) phone numbers** spanning the major worker-mobility jurisdictions, and **diacritic-rich workeral names** correctly.
- Is **trustworthy** for use in clinical-adjacent workflows (audit trail, no silent fallbacks, no surprise IO).

### 1.3 Non-Goals

- Persistent storage, databases, or indexing layers.
- Network calls, telemetry, or background work.
- Machine learning models or trained classifiers (this is a rule-based system).
- Batch / blocking / bulk identity resolution pipelines (single pair at a time).
- Identifier-to-identifier translation across schemes (e.g. resolving a UK NHS Number to a French NIR for the same worker — that requires a registry which this library deliberately does not consult).

---

## 2. Scope

### 2.1 In Scope

- Pairwise matching of two `Worker` records.
- Deterministic matching on **any** of the supported national identifiers (UK NHS Number, France NIR, España TSI, Éire IHI, UK Northern Ireland H&C Number) or complete demographic tuples.
- Probabilistic matching with weighted per-field similarity, including one independent score per national identifier scheme.
- String similarity (Jaro-Winkler, Levenshtein, Combined, Exact).
- Phonetic matching (Soundex) for names.
- Normalization of names (Unicode diacritics, punctuation, case, whitespace), alphanumeric postcodes, phone numbers (with country-code / trunk-prefix stripping), and per-scheme normalisation of national identifiers (whitespace, hyphens, casing, and where applicable the integral check digit).
- Address structural comparison (postcode, city, line 1).
- Serialization / deserialization via `serde` (JSON-first).
- Configurable weights, thresholds, and algorithm choice.

### 2.2 Out of Scope (Today)

- Blocking / candidate generation across large datasets.
- Persistent master worker indices.
- Probabilistic record linkage at population scale (Fellegi-Sunter EM training, etc.).
- Address parsing/standardisation against an external postal-address reference file.
- Validation of national identifier schemes beyond UK NHS, France NIR, España TSI, Éire IHI, and UK Northern Ireland H&C — additional countries are tracked in §21.
- Cross-scheme identity resolution (matching a French NIR against a UK NHS Number for the same worker); only same-scheme identifiers are ever compared.

---

## 3. Stakeholders and Users

| Stakeholder | Interest |
|---|---|
| Crate maintainer | Joel Parker Henderson — overall ownership and direction. |
| Crate consumers (Rust developers) | A stable, documented public API and predictable SemVer behaviour. |
| Healthcare integrators | A reusable matching primitive that drops into HIE pipelines without bringing IO, runtimes, or hidden state. |
| Clinical safety reviewers | Explainability and auditability of every match decision. |
| Information governance teams | Assurance that no PII leaves the process or is logged. |
| End users with diacritic names | Correctness on Unicode diacritics (e.g. `â`, `ŷ`, `é`, `ü`, `ö`, `ł`). |

---

## 4. Glossary

| Term | Definition |
|---|---|
| **HIE** | Health Information Exchange — system that shares worker data across organisations. |
| **UK NHS Number** | United Kingdom National Health Service Number (England, Wales, Isle of Man) — 10-digit healthcare identifier with a Modulus-11 check digit. Parsed via the `nhs-number` crate. |
| **France NIR** | *Numéro d'Inscription au Répertoire* — France's 15-character national social-security / healthcare identifier with a Modulus-97 check key. Also called the INSEE number or *Numéro de Sécurité Sociale*. |
| **España TSI** | *Tarjeta Sanitaria Individual* — Spain's national healthcare identifier, with regionally-varying formats. The national-level form is the *Código de Identificación Workeral del Sistema Nacional de Salud* (CIP-SNS). |
| **Éire IHI** | Individual Health Identifier — the Republic of Ireland's 7-digit national healthcare identifier, issued under the Health Identifiers Act 2014. |
| **UK NI H&C Number** | United Kingdom Northern Ireland Health and Care Number — a 10-digit Modulus-11 identifier issued by HSC, sharing the NHS Number algorithm. |
| **UK Scotland CHI Number** | United Kingdom Scotland Community Health Index Number — a 10-digit identifier issued by NHS Scotland with the same Mod-11 algorithm as the NHS Number; format `DDMMYYSSSC`. |
| **US SSN** | United States Social Security Number — a 9-digit identifier issued by the Social Security Administration. Conventionally written `"AAA-GG-SSSS"`. Has structural validity rules (`000`, `666`, and `900..=999` areas, group `00`, and serial `0000` are never issued) but no public check-digit algorithm. |
| **AU IHI** | Australia Individual Healthcare Identifier — a 16-digit identifier issued by the Healthcare Identifiers Service with a Luhn check digit (ISO/IEC 7812-1). Conventionally prefixed with `800360`. |
| **DE KVNR** | Germany *Krankenversichertennummer* — a 10-character lifelong health-insurance number (1 uppercase letter followed by 9 digits) with a Mod-10 check digit. |
| **IT CF** | Italy *Codice Fiscale* — a 16-character alphanumeric tax identifier with a Mod-26 check character. |
| **NL BSN** | Netherlands *Burgerservicenummer* — a 9-digit citizen-service number used across Dutch authorities, with the "11-test" check rule. |
| **SE Workernummer** | Sweden workeral identity number — `YYMMDD-NNNC` (10-digit) or `YYYYMMDD-NNNC` (12-digit), with a Luhn check digit. |
| **PII** | Workerally Identifiable Information. |
| **Deterministic match** | A binary same/not-same decision based on exact agreement of identifiers. |
| **Probabilistic match** | A score-based decision combining multiple weak signals. |
| **Jaro-Winkler** | String similarity favouring common prefixes; well-suited to worker names. |
| **Levenshtein** | Edit-distance metric; counts insertions/deletions/substitutions. |
| **Soundex** | Phonetic algorithm encoding consonants to digits; equates similar-sounding names. |
| **NFKD** | Unicode Normalization Form, Compatibility Decomposition. |
| **Confidence** | Qualitative bucketing of a score: High / Medium / Low. |

---

## 5. Research Basis

This crate is grounded in:

1. Grannis SJ, et al. *Worker matcher within a Health Information Exchange.* AMIA Annu Symp Proc, 2014. (PMC4696093)
2. Reisman M. *Patient Identification Techniques — Approaches, Implications, and Findings.* NCVHS, 2020. (PMC7442501)

The two PDF source documents are stored in [`help/`](./help/).

### 5.1 Findings That Shape the Design

- Real-world error rates average **~8%** and can reach **20%**.
- Even best-in-class techniques top out around **90–98% accuracy**; no algorithm achieves 100%.
- Hybrid deterministic + probabilistic strategies outperform either alone.
- **Data standardisation before matching** is essential: most gains come from normalisation, not from cleverer scoring.
- Single-identifier reliance (e.g. NHS number alone) is brittle — multi-factor matching is more robust.

### 5.2 How the Findings Are Applied

- Inputs are normalised before scoring (see §14).
- Multiple weak signals are combined via weighted average (see §12.3).
- Match results are **transparent**: every component score is returned in `MatchBreakdown`.
- Defaults are conservative (threshold 0.85) and can be tightened (`strict()`) or relaxed (`lenient()`).

---

## 6. Functional Requirements

Identifiers use **MUST**/**SHOULD**/**MAY** (RFC 2119) semantics.

### 6.1 Worker Model
- **FR-1** The library MUST expose a `Worker` struct holding the fields listed in §8.1, including thirty-five national-identifier fields plus `passport_books: Vec<PassportBook>`: `uk_nhs_number`, `fr_nir`, `es_tsi`, `ie_ihi`, `uk_hc_number`, `us_ssn`, `au_ihi`, `de_kvnr`, `it_cf`, `nl_bsn`, `se_workernummer`, `uk_chi_number`, `be_nn`, `bg_egn`, `cz_rc`, `dk_cpr`, `ee_ik`, `es_dni`, `fi_hetu`, `hr_oib`, `is_kt`, `lt_ak`, `lv_pk`, `mt_id`, `no_fnr`, `pl_pesel`, `ro_cnp`, `si_emso`, `sk_rc`, `uk_nino`, `gr_dss`, `li_id`, `nl_id`, `pl_nip`, `pt_nif`.
- **FR-2** `Worker` MUST be constructible via a fluent `WorkerBuilder` with one setter per national identifier.
- **FR-3** `Worker` MUST be cloneable, comparable for equality, debuggable, and serde-serializable.
- **FR-4** `Worker::validate()` MUST require at least one of: a name (`given_name` or `family_name`), any of the thirty national identifiers, or a non-empty `passport_books` list. Otherwise it MUST return `MatchingError::MissingField`.

### 6.2 Matching Engine
- **FR-5** The library MUST expose a `MatchingEngine` configured by a `MatchConfig`.
- **FR-6** `MatchingEngine::match_workers(&p1, &p2)` MUST return a `MatchResult { score, is_match, confidence, breakdown }`.
- **FR-7** `MatchingEngine::deterministic_match(&p1, &p2)` MUST return `bool` and MUST NOT depend on `match_threshold`.
- **FR-8** Probabilistic `score` MUST be in `[0.0, 1.0]`.
- **FR-9** `is_match` MUST be `true` iff `score >= match_threshold`.
- **FR-10** Missing fields MUST NOT throw — they MUST be omitted from the weighted average and reflected as `None` in the breakdown.
- **FR-45** The library MUST expose `MatchingEngine::match_one_to_many(query, candidates)` returning a `Vec<MatchResult>` parallel to the candidates slice; the order MUST match the slice order. Empty candidates MUST return an empty `Vec`.
- **FR-46** The library MUST expose `MatchingEngine::rank_one_to_many(query, candidates)` returning a `Vec<(usize, MatchResult)>` sorted by `score` descending. Ties MUST be broken deterministically by ascending original index so equal-score inputs produce a stable ranking.
- **FR-47** When `MatchConfig::strict_mode` is `true`, `MatchResult::is_match` MUST be `(score >= match_threshold) && deterministic_match(p1, p2)`. The probabilistic `score` and `confidence` MUST be unchanged — strict mode tightens only the binary `is_match` decision.
- **FR-48** The address sub-score MUST consider every pair drawn from one side's `address ∪ previous_addresses` against the other side's `address ∪ previous_addresses`. The reported `MatchBreakdown::address_score` is the **best (highest)** score across that cartesian product. Returns `None` only when at least one side has no address data at all (neither current nor historical).
- **FR-49** When both workers carry a `middle_name`, the given-name component score MUST blend the given-name similarity with a middle-name similarity at weights `0.95` and `0.05` respectively. The middle-name similarity uses the same `name_algorithm` and nickname-table boost as the given-name path. When either side lacks a middle name the blend MUST be skipped and the unblended given-name similarity returned.
- **FR-50** The library MUST expose a public `PassportBook` type carrying an ISO 3166-1 alpha-2 `country` code, a `number`, and optional `issued` / `expires` dates. Construction via `PassportBook::new` MUST canonicalise the country (trimmed, uppercased; exactly 2 ASCII letters) and the number (whitespace stripped, uppercased) and return `None` on invalid input. Date fields are metadata and MUST NOT participate in matching.
- **FR-51** `Worker` MUST carry a `passport_books: Vec<PassportBook>` field. `MatchBreakdown::passport_book_score` MUST be `Some(1.0)` when at least one `(country, number)` pair is shared across the two workers' books, `Some(0.0)` when both sides carry at least one book but no pair is shared, and `None` when either side has no books. `Worker::validate()` MUST accept a worker whose only identifying data is a non-empty `passport_books` list.
- **FR-52** `deterministic_match` MUST return `true` when the two workers share at least one `(country, number)` passport-book pair after canonicalisation. Cross-country values with the same `number` MUST NEVER cross-match; provenance lives on `PassportBook::country`, not on the field name.
- **FR-53** `Worker` and `Address` MUST carry `#[non_exhaustive]`. External consumers MUST construct them via the builder (`Worker::builder()`) or the constructor plus fluent setters (`Address::new().with_postcode(...)`) — direct struct-literal construction is reserved for the defining crate. This formalises the long-standing expectation that field additions are non-breaking.
- **FR-37** `MatchResult::confidence` MUST be derived from `score` via the fixed band table in §12.5 (≥0.90 = `High`, ≥0.75 = `Medium`, else `Low`). Bands MUST be independent of `match_threshold`: the same `score` always maps to the same band regardless of which preset produced it.
- **FR-38** The date-of-birth component score MUST be `1.0` for exact equality, `0.5` when swapping the day and month on one side yields the other AND the years agree AND the swapped form is itself a valid calendar date, and `0.0` otherwise. The transposition heuristic MUST apply **only** to the probabilistic component score; `deterministic_match` MUST continue to require exact `NaiveDate` equality on the demographic-tuple branch.

### 6.3 Determinism
- **FR-11** Given the same inputs and config, results MUST be byte-identical across runs (no time, no RNG, no global state).

### 6.4 National Identifier Handling

The library supports thirty-five national identifier schemes. Each is parsed by an associated function in the `identifiers` module and scored as an independent component in `MatchBreakdown`.

### 6.4a Passport Books

Passport book numbers do not fit the per-scheme `Option<String>` national-identifier pattern. Three real-world properties drive a separate model:

1. **Scheme-local provenance.** A book number is only meaningful alongside its issuing country. `"AB123456"` from the UK is a different identifier from `"AB123456"` from the US. Provenance must travel with the value.
2. **Multi-country.** A single worker may hold passports from several countries simultaneously (dual / multiple citizenship). Each passport is recorded as a separate `PassportBook` entry.
3. **Time-varying.** When a passport is renewed, the new book has a different number. Records carry both current and historical book numbers as separate `PassportBook` entries; the matcher treats any shared `(country, number)` pair as evidence the records refer to the same worker, regardless of issue date.

The `PassportBook` type (§8.6) and the `passport_books: Vec<PassportBook>` field on `Worker` (§8.1) capture this model. The matcher's deterministic and probabilistic paths consume the books as a set of `(country, number)` keys; date metadata is carried for audit but is not used in matching (FR-50 / FR-51 / FR-52).

- **FR-12** UK NHS Numbers MUST be parsed using the `nhs-number` crate via `identifiers::parse_uk_nhs_number`, accepting whitespace and other separators tolerated by that crate. The canonical form is the 10-digit compact string.
- **FR-13** Two identifiers in the **same scheme** MUST compare equal iff the scheme's parser produces `Some(canonical)` for both AND the canonical strings are equal. Identifiers from **different schemes** MUST NEVER cross-match, even when they share a textual value.
- **FR-14** A malformed identifier on either side MUST yield the corresponding `<scheme>_score = None` (not `0.0`) in the breakdown.
- **FR-25** France NIRs MUST be parsed via `identifiers::parse_fr_nir`. The parser MUST strip whitespace, uppercase letters, require exactly 15 characters, and validate the Modulus-97 check key (with Corsica department remapping `"2A" → "19"`, `"2B" → "18"`).
- **FR-26** España TSIs / CIP-SNS MUST be parsed via `identifiers::parse_es_tsi`. The parser MUST strip whitespace and ASCII hyphens, uppercase letters, require only ASCII alphanumerics, and require length in `10..=20`. No check-digit calculation is performed because Spanish regional schemes vary.
- **FR-27** Éire IHIs MUST be parsed via `identifiers::parse_ie_ihi`. The parser MUST strip non-digit characters and require exactly 7 digits.
- **FR-28** UK Northern Ireland H&C Numbers MUST be parsed via `identifiers::parse_uk_hc_number`. The algorithm is identical to UK NHS Number parsing because H&C and NHS share the 10-digit Modulus-11 scheme, but the two parsers are intentionally exposed as distinct functions tied to distinct `Worker` fields.
- **FR-32** United States Social Security Numbers MUST be parsed via `identifiers::parse_us_ssn`. The parser MUST keep only ASCII digits, require exactly 9 digits, and reject structurally-impossible values: area number `000`, area number `666`, area number in `900..=999`, group number `00`, and serial number `0000`. The canonical form is the 9-digit compact string. No geographic decoding is attempted (SSA assignment has been randomised since June 2011).
- **FR-39** Australia IHIs MUST be parsed via `identifiers::parse_au_ihi`. The parser MUST keep only ASCII digits, require exactly 16 digits, and validate a Luhn check digit (ISO/IEC 7812-1). The canonical form is the 16-digit compact string. The structural convention that real IHIs begin with `800360` is NOT enforced.
- **FR-40** Germany KVNRs MUST be parsed via `identifiers::parse_de_kvnr`. The parser MUST strip whitespace, uppercase the leading letter, require 1 ASCII letter followed by 9 ASCII digits, and validate the Mod-10 check digit where the leading letter is mapped to a two-digit ordinal (`A=01..=Z=26`) and concatenated with the next 8 digits before alternating-weight summation.
- **FR-41** Italy *Codice Fiscale* values MUST be parsed via `identifiers::parse_it_cf`. The parser MUST strip whitespace, uppercase letters, require exactly 16 ASCII alphanumerics, and validate the Mod-26 check character via the standard odd/even position tables.
- **FR-42** Netherlands BSNs MUST be parsed via `identifiers::parse_nl_bsn`. The parser MUST keep only ASCII digits, require exactly 9 digits, reject the all-zero string, and validate the "11-test" check rule (`9·d₁ + 8·d₂ + … + 2·d₈ − d₉ ≡ 0 (mod 11)`).
- **FR-43** Sweden *Workernummer* values MUST be parsed via `identifiers::parse_se_workernummer`. The parser MUST keep only ASCII digits, accept exactly 10 or 12 digits, validate the Luhn check digit computed over the 10-digit (year-truncated) form, and preserve the input length in the canonical output (10-digit and 12-digit forms therefore do NOT cross-match).
- **FR-44** UK Scotland CHI Numbers MUST be parsed via `identifiers::parse_uk_chi_number`. The parser MUST keep only ASCII digits, require exactly 10 digits, and validate the Mod-11 check digit (same algorithm as the UK NHS Number). A computed check digit of 10 MUST be rejected. The CHI Number is scheme-local and MUST NOT cross-match with the UK NHS Number or UK NI H&C Number even when the 10 digits agree.
- **FR-29** `deterministic_match` MUST return `true` if **any** of the thirty national identifier schemes produces an equal canonical-form pair, if the passport-book branch fires (FR-52), or if the demographic-tuple branch holds (see §12.1).
- **FR-54** Belgium National Numbers MUST be parsed via `identifiers::parse_be_nn`. 11 digits; check is `97 − (first-9-digits mod 97)`, with a `"2"` prefix prepended before the modulo step for births in 2000 or later; the parser MUST accept either form.
- **FR-55** Bulgaria EGNs MUST be parsed via `identifiers::parse_bg_egn`. 10 digits; weights `[2,4,8,5,10,9,7,3,6]` mod 11; mod = 10 ⇒ check = 0.
- **FR-56** Czech *Rodné číslo* MUST be parsed via `identifiers::parse_cz_rc`. Accept 9 digits (pre-1954) as-is or 10 digits where the full number is divisible by 11 (edge case: first-9 mod 11 = 10 collapses to a trailing 0).
- **FR-57** Denmark CPR MUST be parsed via `identifiers::parse_dk_cpr`. 10 digits, format-only (the historical Modulus-11 check was abandoned in 2007).
- **FR-58** Estonia *Isikukood* MUST be parsed via `identifiers::parse_ee_ik`. 11 digits with a cascading Mod-11 check (pass-1 weights `[1..9, 1]`; pass-2 weights `[3..9, 1, 2, 3]`; mod = 10 in pass-2 ⇒ check = 0).
- **FR-59** Spain DNI / NIE MUST be parsed via `identifiers::parse_es_dni`. 8 digits + control letter from `"TRWAGMYFPDXBNJZSQVHLCKE"` indexed by `number mod 23`; NIE prefixes `X`/`Y`/`Z` map to leading digits `0`/`1`/`2`.
- **FR-60** Finland HETU MUST be parsed via `identifiers::parse_fi_hetu`. 11 chars `DDMMYY` + century sign + 3 digits + Mod-31 check character from `"0123456789ABCDEFHJKLMNPRSTUVWXY"`.
- **FR-61** Croatia OIB MUST be parsed via `identifiers::parse_hr_oib`. 11 digits with ISO 7064 MOD 11,10 check.
- **FR-62** Iceland *Kennitala* MUST be parsed via `identifiers::parse_is_kt`. 10 digits with Mod-11 check (weights `[3,2,7,6,5,4,3,2]`); mod = 10 ⇒ invalid.
- **FR-63** Lithuania *Asmens kodas* MUST be parsed via `identifiers::parse_lt_ak`. 11 digits with the same cascading Mod-11 algorithm as Estonia.
- **FR-64** Latvia *Workeras kods* MUST be parsed via `identifiers::parse_lv_pk`. 11 digits; weights `[1,6,3,7,9,10,5,8,4,2]`; `check = ((1101 − Σ) mod 11) mod 10`.
- **FR-65** Malta National ID MUST be parsed via `identifiers::parse_mt_id`. 7 digits + letter in `{M, G, A, P, L, H, B, Z}` (format-only — the letter encodes geographic provenance).
- **FR-66** Norway *Fødselsnummer* MUST be parsed via `identifiers::parse_no_fnr`. 11 digits with two Mod-11 check digits; weights for check 1 = `[3,7,6,1,8,9,4,5,2]`, weights for check 2 = `[5,4,3,2,7,6,5,4,3,2]`; mod = 10 ⇒ invalid.
- **FR-67** Poland PESEL MUST be parsed via `identifiers::parse_pl_pesel`. 11 digits with weighted Mod-10 check (weights `[1,3,7,9,1,3,7,9,1,3]`).
- **FR-68** Romania CNP MUST be parsed via `identifiers::parse_ro_cnp`. 13 digits with Mod-11 check using weights `"279146358279"` (`[2,7,9,1,4,6,3,5,8,2,7,9]`); mod = 10 ⇒ check = 1.
- **FR-69** Slovenia EMŠO MUST be parsed via `identifiers::parse_si_emso`. 13 digits with Mod-11 check using weights `[7,6,5,4,3,2,7,6,5,4,3,2]`; mod = 0 ⇒ check = 0, else `11 − mod`; check = 10 ⇒ invalid.
- **FR-70** Slovakia *Rodné číslo* MUST be parsed via `identifiers::parse_sk_rc` (same algorithm as Czech RČ).
- **FR-71** UK NINO MUST be parsed via `identifiers::parse_uk_nino`. Format `AA999999A`; banned first prefix letters `D F I Q U V`; banned second prefix letters `D F I O Q U V`; banned admin prefixes `OO CR FY MW NC PP PZ TN`; suffix MUST be one of `A B C D`. Format-only — no checksum.
- **FR-72** Greece DSS investor-share codes MUST be parsed via `identifiers::parse_gr_dss`. Exactly 10 ASCII digits; format-only.
- **FR-73** Liechtenstein National Identity Card Numbers MUST be parsed via `identifiers::parse_li_id`. 2 ASCII letters followed by 8 or 9 ASCII digits (accepting both the spec's textual description and its example). Format-only; the number changes on each card renewal, so for cross-renewal matching consumers SHOULD prefer `PassportBook` with `country = "LI"`.
- **FR-74** Netherlands National Identity Card Numbers MUST be parsed via `identifiers::parse_nl_id`. 9 characters: positions 1–2 are uppercase letters except `O`; positions 3–8 are alphanumeric except `O`; position 9 is a digit.
- **FR-75** Poland NIP MUST be parsed via `identifiers::parse_pl_nip`. 10 digits; weights `[6,5,7,2,3,4,5,6,7]` mod 11; mod = 10 ⇒ invalid; else 10th digit MUST equal the remainder.
- **FR-76** Portugal NIF MUST be parsed via `identifiers::parse_pt_nif`. 9 digits; weights `[9,8,7,6,5,4,3,2]` over the first 8; `r = Σ mod 11`; check = `0` if `r < 2`, else `11 − r`.
- **FR-77** The library MUST expose per-country **passport-number format validators** in the `identifiers` module: `parse_cy_passport` (`E` + 6 digits or `K` + 8 digits), `parse_cz_passport` (8 to 12 digits), `parse_li_passport` (1 letter + 5 digits), `parse_lt_passport` (8 digits), `parse_mt_passport` (7 digits), `parse_nl_passport` (same shape as the NL ID card), `parse_pt_passport` (1 letter + 6 digits), `parse_ro_passport` (2 letters + 6 digits), `parse_sk_passport` (2 letters + 7 digits). These are pure format validators; they have NO corresponding `Worker` field — passport-book data flows through `Worker::passport_books: Vec<PassportBook>` per FR-50/51/52.
- **FR-78** The library MUST expose a public `BloodType` enum with the 8 ABO+RhD variants (`APositive`, `ANegative`, `BPositive`, `BNegative`, `ABPositive`, `ABNegative`, `OPositive`, `ONegative`), serialised as their canonical short forms (`"A+"`, `"A-"`, `"B+"`, `"B-"`, `"AB+"`, `"AB-"`, `"O+"`, `"O-"`).
- **FR-79** `BloodType::parse(s)` MUST accept canonical short forms, lowercase forms, word forms (`"A positive"`, `"A pos"`, `"A negative"`, `"A neg"`), `+VE`/`-VE` suffixes, separator chars (`A_pos`, `A-neg`), and the zero-to-O ASCII confusion (`"0+"` → `OPositive`). Unparseable, empty, or rare-phenotype inputs MUST return `None`.
- **FR-80** `Worker::blood_type: Option<BloodType>` MUST carry the recorded ABO+RhD value. `MatchBreakdown::blood_type_score` MUST be `Some(1.0)` when both records have equal blood types, `Some(0.0)` when both have different blood types, and `None` when either side is missing. `MatchConfig::blood_type_weight` defaults to `0.05` (weak positive signal). Blood type MUST NOT contribute to `deterministic_match` (too weak alone) and MUST NOT be part of `Worker::validate`'s identifying-field set.
- **FR-81** `Worker::birth_place: Option<Address>` MUST carry the FHIR `Patient.birthPlace` value, reusing the existing `Address` type. `MatchBreakdown::birth_place_score` MUST be computed against the `city` and `country` sub-fields only (street/postcode are irrelevant for birth places): city via Jaro-Winkler on the name-normalised value, country via exact equality on the name-normalised value; when both are populated, blend as `0.7 × city + 0.3 × country`; when only one is populated on each side, return that single signal; when no comparable subset exists, return `None`. `MatchConfig::birth_place_weight` defaults to `0.05`. Birth place MUST NOT contribute to `deterministic_match` and MUST NOT be part of `Worker::validate`'s identifying-field set.
- **FR-82** `Worker::multiple_birth: Option<u8>` MUST carry the FHIR `Patient.multipleBirth` value. The integer is the 1-indexed birth order within a multiple-birth set; `None` means unknown or singleton. `MatchBreakdown::multiple_birth_score` MUST be `Some(1.0)` when both records carry equal values, `Some(0.0)` when both carry different values, and `None` when either side is missing. `MatchConfig::multiple_birth_weight` defaults to `0.05`. The field MUST NOT contribute to `deterministic_match` and MUST NOT be part of `Worker::validate`'s identifying-field set. Its primary purpose is to disambiguate identical-twin records that otherwise share name, DOB, and demographic data.
- **FR-83** `Worker::death_date: Option<NaiveDate>` MUST carry the FHIR `Patient.deceasedDateTime` value (date precision only). `MatchBreakdown::death_date_score` MUST use the same DOB transposition heuristic as `MatchBreakdown::date_of_birth_score`: `Some(1.0)` for exact equality, `Some(0.5)` for a same-year day/month swap, `Some(0.0)` otherwise; `None` when either side is missing. `MatchConfig::death_date_weight` defaults to `0.10`. Death date MUST NOT contribute to `deterministic_match` and MUST NOT be part of `Worker::validate`'s identifying-field set.
- **FR-84** `Worker::death_place: Option<Address>` MUST carry the place of death, reusing the existing `Address` type for parity with `Worker::birth_place`. `MatchBreakdown::death_place_score` MUST be computed against the `city` and `country` sub-fields only, using the same `0.7 × city (Jaro-Winkler) + 0.3 × country (exact)` blend as the birth-place sub-score (FR-81); when only one of the two is populated on each side, return that single signal; when no comparable subset exists, return `None`. `MatchConfig::death_place_weight` defaults to `0.05`. Death place MUST NOT contribute to `deterministic_match` and MUST NOT be part of `Worker::validate`'s identifying-field set.
- **FR-85** Brazil CPF (*Cadastro de Pessoas Físicas*) MUST be parsed via `identifiers::parse_br_cpf`. 11 digits; strip non-digits; reject all-equal sequences (sentinel data); validate two Mod-11 weighted check digits at positions 9 and 10 using weights `[10, 9, 8, 7, 6, 5, 4, 3, 2]` and `[11, 10, 9, 8, 7, 6, 5, 4, 3, 2]` respectively, with the convention `check = 0` if `r < 2` else `11 − r`.
- **FR-86** China Resident Identity Card numbers (*居民身份证*) MUST be parsed via `identifiers::parse_cn_rrn`. 18 characters: 17 digits + check character (digit or uppercase `X`; lowercase `x` MUST be accepted and canonicalised to uppercase). The substring at positions 6..14 MUST be a valid `YYYYMMDD` calendar date. The check character MUST satisfy `CHECK[(Σ d[i] × W[i]) mod 11]` where `W = [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2]` and `CHECK = ['1','0','X','9','8','7','6','5','4','3','2']`. The pre-1999 15-digit form is NOT accepted; consumers MUST migrate upstream.
- **FR-87** India Aadhaar numbers MUST be parsed via `identifiers::parse_in_aadhaar`. 12 digits; strip non-digits; reject all-equal sequences and the UIDAI-reserved prefixes (numbers starting with `0` or `1`); validate the Verhoeff check digit at the rightmost position using the standard `D` (dihedral multiplication) and `P` (permutation) tables.
- **FR-88** Japan My Number (*個人番号*) MUST be parsed via `identifiers::parse_jp_my_number`. 12 digits; weights `[6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2]` over the first 11 digits; `r = Σ mod 11`; check digit MUST be `0` if `r < 2` else `11 − r`.
- **FR-89** Mexico CURP (*Clave Única de Registro de Población*) MUST be parsed via `identifiers::parse_mx_curp`. 18 characters, uppercased; structural shape `LLLL DDDDDD S LL LLL X D` where `L` is `A..Z` or `Ñ`, `D` is a digit, `S` is `H` or `M`, `X` is alphanumeric. The substring at positions 4..10 MUST be a valid `YYMMDD` calendar date (century inferred: `YY <= 29 → 20YY`, else `19YY`). The check digit MUST equal `(10 − (sum mod 10)) mod 10` where `sum = Σ value(char[i]) × (18 − i)` for `i ∈ 0..17` using the value table (`0..9` literal, `A..N` = 10..23, `Ñ` = 24, `O..Z` = 25..36).
- **FR-90** New Zealand NHI Numbers (original 7-character format: 3 letters + 4 digits) MUST be parsed via `identifiers::parse_nz_nhi`. Letters MUST be `A..Z` excluding `I` and `O`. Letter values are assigned consecutively skipping `I` and `O` (`A=1..H=8, J=9..N=13, P=14..Z=24`); weights `[7, 6, 5, 4, 3, 2]` over the three letters and the first three digits; `r = Σ mod 11`; if `r == 0` check digit is `0`, if `r == 1` the NHI is invalid (no representable check digit), else check = `11 − r`. The 2019 alphanumeric NHI revision is NOT supported by the parser; consumers handling that form SHOULD validate upstream.
- **FR-91** South Africa ID Numbers MUST be parsed via `identifiers::parse_za_id`. 13 digits. The substring at positions 0..6 MUST be a valid `YYMMDD` calendar date (century inferred: `YY <= 29 → 20YY`, else `19YY`). The Luhn check MUST validate over all 13 digits. The remaining substrings (sequence at 6..10, citizenship at 10, legacy race indicator at 11) MUST NOT be validated by the parser — they are demographic data the worker-matcher layer does not use.

### 6.5 Normalization
- **FR-15** Names MUST be lowercased, NFKD-decomposed with combining marks removed, ASCII-punctuation stripped, and whitespace collapsed.
- **FR-16** Postcodes MUST be uppercased and have whitespace removed.
- **FR-31** Address lines MUST be normalisable via a public function (`Normalizer::normalize_address_line`) that expands common street-type and directional abbreviations (`St`→`Street`, `Rd`→`Road`, `Ave`→`Avenue`, `N`→`North`, etc.) before applying the name-normalisation pipeline. Address lines MUST also be parseable via a public function (`Normalizer::parse_address_line`) that returns a `ParsedAddressLine { house_number, unit, street }` triple suitable for structural matching.
- **FR-17** Phone numbers have two normalisation paths. The legacy national-significant form (`Normalizer::normalize_phone`) MUST reduce input to ASCII digits and strip the international prefix `0044`, the dialling code `44` (when the remaining number is long enough), and a single leading trunk `0`, in that order. The international E.164 form (`Normalizer::normalize_phone_e164`) MUST return `Some("+CCNNN…")` when the input parses against a supported country (explicit `+CC` / `00CC` marker, or default-country fallback for national-format inputs) and `None` otherwise.
- **FR-30** `MatchingEngine::match_workers` MUST score phone equality using the E.164 canonical form when both inputs parse to `Some`, and MUST fall back to the legacy national-significant comparison when either input fails to parse. The default country code used for national-format inputs is the ISO 3166-1 alpha-2 string carried on `MatchConfig::phone_default_country` (defaulting to `"GB"`). Numbers from different countries that share the same national-significant digits MUST NOT collide under the E.164 path.
- **FR-35** Email addresses MUST be normalised via `Normalizer::normalize_email(email, gmail_dot_folding)` which returns `Some(canonical)` after trim + lowercase + structural validation, or `None` when the input lacks exactly one `@` or has an empty localpart or domain. When `gmail_dot_folding` is `true` and the domain is `gmail.com` or `googlemail.com`, the localpart MUST have every `.` removed and any `+tag` suffix dropped.
- **FR-36** `MatchingEngine::match_workers` MUST score email equality using the canonical form. `MatchBreakdown::email_score` MUST be `Some(1.0)` for equal canonical forms, `Some(0.0)` for unequal forms when both parse, and `None` when either input is absent or fails to parse. `local_id` MUST NOT be scored: different organisations may issue colliding values, so a positional match would produce false positives.

### 6.6 Configuration
- **FR-18** `MatchConfig::default()` MUST yield the weights in §13.1.
- **FR-19** `MatchConfig::strict()` MUST raise threshold to **0.95** and enable `strict_mode`.
- **FR-20** `MatchConfig::lenient()` MUST lower threshold to **0.75**.
- **FR-21** Weights MAY sum to anything; the engine MUST internally normalise by the sum of weights of fields that participated in the score.

### 6.7 Phonetic Matching
- **FR-22** When `use_phonetic_matching` is `true` and both workers have given AND family names, the engine MUST compute a phonetic name score using Soundex.
- **FR-23** If the phonetic score exceeds **0.9**, it MUST contribute an additional 0.05-weighted bonus to the weighted average.

### 6.7a Nickname Matching
- **FR-33** The library MUST expose a public `NicknameTable` type with at least: `empty()`, `english()`, `with_class(names)`, and `are_equivalent(a, b)`. `MatchConfig::nickname_table` MUST default to `NicknameTable::empty()` so the feature is opt-in and existing behaviour is preserved.
- **FR-34** When `nickname_table.are_equivalent(name1, name2)` is `true` for either the given-name or family-name pair, the corresponding component score MUST be at least **0.9**. The boost MUST NOT lower an already-higher score.

### 6.8 Serialization
- **FR-24** `Worker`, `Address`, `Gender`, `MatchResult`, `MatchBreakdown` MUST round-trip losslessly via `serde_json`.

---

## 7. Non-Functional Requirements

| ID | Requirement |
|---|---|
| **NFR-1 Performance** | A single pairwise match MUST complete in microseconds on commodity hardware. |
| **NFR-2 Memory** | No persistent allocations between calls; bounded per-call allocations proportional to input size. |
| **NFR-3 Concurrency** | All public types MUST be `Send + Sync` where their fields permit; the engine is immutable after construction. |
| **NFR-4 Stability** | The public API MUST follow SemVer. Pre-1.0 minors MAY break; documented in CHANGELOG. |
| **NFR-5 Determinism** | See FR-11. |
| **NFR-6 No IO** | The crate MUST NOT perform file, network, or stdin/stdout/stderr IO from library code (only the `main.rs` demo may print). |
| **NFR-7 No unsafe** | The crate MUST NOT contain `unsafe` blocks. |
| **NFR-8 Linting** | `cargo clippy --all-targets -- -D warnings` MUST pass. |
| **NFR-9 Formatting** | `cargo fmt --check` MUST pass. |
| **NFR-10 Documentation** | All public items MUST have rustdoc; doctests MUST compile. |
| **NFR-11 i18n** | Latin-script diacritics MUST be handled via NFKD decomposition; the same pipeline SHOULD cope with any Unicode combining mark without special-casing per language. |
| **NFR-12 Reproducibility** | `cargo test` MUST pass on a fresh checkout with no environment variables. |

---

## 8. Domain Model

### 8.1 `Worker`

Field naming convention for national identifiers: `<cc>_<scheme>` where `<cc>` is the ISO 3166-1 alpha-2 country code (lower-cased). This keeps related schemes alphabetised within a country, and makes new countries easy to slot in.

| Field | Type | Required | Notes |
|---|---|---|---|
| `uk_nhs_number` | `Option<String>` | At least one identifying field (a name or a national identifier) — see FR-4 | United Kingdom NHS Number (England, Wales, Isle of Man). Raw string; parsed at match time via `identifiers::parse_uk_nhs_number`. |
| `fr_nir` | `Option<String>` | At least one identifying field | France NIR. Raw string; parsed at match time via `identifiers::parse_fr_nir`. |
| `es_tsi` | `Option<String>` | At least one identifying field | España TSI / CIP-SNS. Raw string; parsed at match time via `identifiers::parse_es_tsi`. |
| `ie_ihi` | `Option<String>` | At least one identifying field | Éire IHI. Raw string; parsed at match time via `identifiers::parse_ie_ihi`. |
| `uk_hc_number` | `Option<String>` | At least one identifying field | United Kingdom Northern Ireland H&C Number. Raw string; parsed at match time via `identifiers::parse_uk_hc_number`. |
| `us_ssn` | `Option<String>` | At least one identifying field | United States Social Security Number. Raw string; parsed at match time via `identifiers::parse_us_ssn`. |
| `au_ihi` | `Option<String>` | At least one identifying field | Australia IHI. Raw string; parsed at match time via `identifiers::parse_au_ihi`. |
| `de_kvnr` | `Option<String>` | At least one identifying field | Germany KVNR. Raw string; parsed at match time via `identifiers::parse_de_kvnr`. |
| `it_cf` | `Option<String>` | At least one identifying field | Italy *Codice Fiscale*. Raw string; parsed at match time via `identifiers::parse_it_cf`. |
| `nl_bsn` | `Option<String>` | At least one identifying field | Netherlands BSN. Raw string; parsed at match time via `identifiers::parse_nl_bsn`. |
| `se_workernummer` | `Option<String>` | At least one identifying field | Sweden *Workernummer*. Raw string; parsed at match time via `identifiers::parse_se_workernummer`. |
| `uk_chi_number` | `Option<String>` | At least one identifying field | UK Scotland CHI Number. Raw string; parsed at match time via `identifiers::parse_uk_chi_number`. |
| `given_name` | `Option<String>` | At least one identifying field | First name. |
| `middle_name` | `Option<String>` | No | Not currently used in scoring (see §22 OQ-1). |
| `family_name` | `Option<String>` | At least one identifying field | Surname. |
| `date_of_birth` | `Option<NaiveDate>` | No | `chrono::NaiveDate`. |
| `death_date` | `Option<NaiveDate>` | No | FHIR `Patient.deceasedDateTime` (date precision only). Scored with the DOB transposition heuristic. See §12.2 / FR-83. |
| `gender` | `Option<Gender>` | No | See §8.2. |
| `blood_type` | `Option<BloodType>` | No | ABO+RhD blood type. See §8.2a. |
| `multiple_birth` | `Option<u8>` | No | FHIR `Patient.multipleBirth`. 1-indexed birth order within a multiple-birth set; `None` = unknown / singleton. Disambiguates identical twins. |
| `address` | `Option<Address>` | No | See §8.3. |
| `birth_place` | `Option<Address>` | No | FHIR `Patient.birthPlace`. Typically only `city` and `country` are populated. Scored independently from the current `address`; see §12.2 / §12.4a. |
| `death_place` | `Option<Address>` | No | Place of death (parallel to `birth_place`). Typically only `city` and `country` are populated. See §12.2 / FR-84. |
| `previous_addresses` | `Vec<Address>` | No (defaults to empty) | Used by the address sub-score (best-of cartesian across `address ∪ previous_addresses`, §12.4.2). |
| `passport_books` | `Vec<PassportBook>` | At least one identifying field (a passport book is a valid identifying field) | Multi-country, multi-book, time-varying passport data. See §8.6 and §6.4a. |
| `phone` | `Option<String>` | No | Landline-preferred. |
| `mobile` | `Option<String>` | No | Used as fallback if `phone` is `None`. |
| `email` | `Option<String>` | No | Not currently scored (see §22 OQ-2). |
| `local_id` | `Option<String>` | No | Hospital/practice identifier; not currently scored. |

### 8.2 `Gender`

Enum variants: `Male`, `Female`, `Other`, `Unknown`.

### 8.2a `BloodType`

ABO + RhD blood type, with the 8 standard variants:

| Variant | Short form |
|---|---|
| `APositive` | `A+` |
| `ANegative` | `A-` |
| `BPositive` | `B+` |
| `BNegative` | `B-` |
| `ABPositive` | `AB+` |
| `ABNegative` | `AB-` |
| `OPositive` | `O+` |
| `ONegative` | `O-` |

Blood type is **stable over a lifetime** (modulo bone-marrow transplant edge cases) so disagreement is strong evidence that two records refer to different people. Agreement is a weak positive signal because many people share a blood type (≈38% of the US population is O+). The matcher therefore weights it at `0.05` by default — the same low weight as gender — and exposes the per-field outcome in `MatchBreakdown::blood_type_score` so consumers can flag disagreement explicitly even when the overall score remains high.

`BloodType::parse(s)` is the recommended ingestion entry point; it accepts canonical short forms, word forms, separator-tolerant variants, and the zero-to-O ASCII confusion common in legacy EMR data.

### 8.3 `Address`

All fields are `Option<String>`: `line1`, `line2`, `city`, `county`, `postcode`, `country`.

### 8.4 `MatchResult`

```text
MatchResult {
  score:      f64,            // in [0.0, 1.0]
  is_match:   bool,           // score >= match_threshold
  confidence: Confidence,     // High / Medium / Low band derived from score (§12.5)
  breakdown:  MatchBreakdown, // per-field
}
```

### 8.6 `PassportBook`

```text
PassportBook {
  country: String,            // ISO 3166-1 alpha-2, uppercased, 2 ASCII letters
  number:  String,            // whitespace stripped, uppercased; non-empty
  issued:  Option<NaiveDate>, // metadata only
  expires: Option<NaiveDate>, // metadata only
}
```

Construction via `PassportBook::new(country, number)` returns `Option<PassportBook>`, rejecting invalid country codes or empty numbers and canonicalising both fields. Date fields are optional metadata for downstream display and audit; they are **not** used in matching. Two records using different textual layouts of the same `(country, number)` canonicalise to the same key and therefore match.

The struct is `Debug + Clone + PartialEq + Eq + Serialize + Deserialize` and is re-exported from the crate root as `worker_matcher::PassportBook`.

### 8.5 `MatchBreakdown`

Each field is `Option<f64>`. `None` means "not scored" (input missing on at least one side, or unparseable). `Some(v)` is the component score in `[0.0, 1.0]`.

National-identifier fields (one per scheme): `uk_nhs_number_score`, `fr_nir_score`, `es_tsi_score`, `ie_ihi_score`, `uk_hc_number_score`, `us_ssn_score`, `au_ihi_score`, `de_kvnr_score`, `it_cf_score`, `nl_bsn_score`, `se_workernummer_score`, `uk_chi_number_score`, `passport_book_score`.

Demographic fields: `given_name_score`, `family_name_score`, `date_of_birth_score`, `gender_score`, `address_score`, `phone_score`, `email_score`, `phonetic_name_score`.

---

## 9. Architecture

### 9.1 Module Layout

```
src/
├── lib.rs          Public API re-exports.
├── models.rs       Worker, WorkerBuilder, Address, Gender.
├── identifiers.rs  parse_uk_nhs_number, parse_fr_nir, parse_es_tsi,
│                   parse_ie_ihi, parse_uk_hc_number, parse_us_ssn,
│                   parse_au_ihi, parse_de_kvnr, parse_it_cf,
│                   parse_nl_bsn, parse_se_workernummer, parse_uk_chi_number.
├── matcher.rs      MatchConfig, MatchingEngine, MatchResult, MatchBreakdown.
├── scorer.rs       Similarity primitives (Jaro-Winkler, Levenshtein, Exact, Combined).
├── nicknames.rs    NicknameTable equivalence-class lookup.
├── normalizer.rs   Normalizer for names, postcodes, phones, phonetics.
├── error.rs        MatchingError, Result alias.
└── main.rs         Demonstration binary (not part of the library API).
```

### 9.2 Dependency Graph

```
matcher     ──>  normalizer
   │        ──>  scorer
   │        ──>  models
   │        ──>  identifiers
   └──>  error

identifiers ──>  nhs-number

models      ──>  serde, chrono
scorer      ──>  strsim
normalizer  ──>  unicode-normalization, soundex
error       ──>  thiserror
```

No cycles. `lib.rs` only re-exports.

### 9.3 Layering Rules

- `models` MUST NOT depend on any other crate module.
- `identifiers` MUST NOT depend on `matcher`, `normalizer`, or `scorer`. It is a leaf module beneath `matcher`.
- `normalizer` and `scorer` MUST NOT depend on `matcher`.
- `matcher` is the only orchestration layer.
- `main.rs` is the only place that performs `println!`.

---

## 10. Component Specifications

### 10.1 `Normalizer` (in `normalizer.rs`)

Static utility struct. All methods are `pub fn (input: &str) -> String`.

- `normalize_name(s)` — see §14.1.
- `normalize_postcode(s)` — see §14.2.
- `normalize_phone(s)` — see §14.3.
- `phonetic_code(s)` — Soundex code after name normalisation; empty string if normalised input is empty.

### 10.1a `identifiers` (in `identifiers.rs`)

Free-function module exposing one parser per supported national identifier scheme. Each function takes `&str` and returns `Option<String>`:

- `Some(canonical)` if the input parses; the canonical form is suitable for byte-equality comparison.
- `None` if the input fails the scheme's structural or check-digit test.

| Function | Country / Region | Identifier | Validation summary |
|---|---|---|---|
| `parse_uk_nhs_number` | United Kingdom (England, Wales, Isle of Man) | UK NHS Number | Delegates to `nhs_number::NHSNumber::from_str`; canonical form is the 10-digit compact string. |
| `parse_fr_nir` | France | NIR | 15 characters, Modulus-97 check key, Corsica `2A`/`2B` remapping. |
| `parse_es_tsi` | España (Spain) | TSI / CIP-SNS | Length 10..=20, ASCII alphanumerics only, whitespace and hyphens stripped, uppercased. |
| `parse_ie_ihi` | Éire (Ireland) | IHI | Exactly 7 digits after stripping non-digit characters. |
| `parse_uk_hc_number` | United Kingdom (Northern Ireland) | H&C Number | Same algorithm as `parse_uk_nhs_number`; kept distinct so callers retain scheme provenance. |
| `parse_us_ssn` | United States | SSN | Exactly 9 ASCII digits; reject area `000` / `666` / `900..=999`, group `00`, serial `0000`. |
| `parse_au_ihi` | Australia | IHI | Exactly 16 ASCII digits; Luhn check (ISO/IEC 7812-1). |
| `parse_de_kvnr` | Germany | KVNR | 1 ASCII letter + 9 ASCII digits; Mod-10 check via letter-ordinal expansion. |
| `parse_it_cf` | Italy | *Codice Fiscale* | Exactly 16 ASCII alphanumerics; Mod-26 check via odd/even position tables. |
| `parse_nl_bsn` | Netherlands | BSN | Exactly 9 ASCII digits; 11-test (`9·d₁ + … + 2·d₈ − d₉ ≡ 0 mod 11`); reject all-zero. |
| `parse_se_workernummer` | Sweden | *Workernummer* | 10 or 12 ASCII digits; Luhn check over the 10-digit form. |
| `parse_uk_chi_number` | United Kingdom (Scotland) | CHI | Exactly 10 ASCII digits; same Mod-11 algorithm as the NHS Number. |

The module performs no IO and consults no external registries. See §14.5 for the per-scheme normalisation rules in detail.

### 10.2 `Scorer` (in `scorer.rs`)

Static utility struct exposing similarity primitives in `[0.0, 1.0]`:

- `jaro_winkler_similarity(a, b)` — wraps `strsim::jaro_winkler`. Both empty ⇒ 1.0; exactly one empty ⇒ 0.0.
- `levenshtein_similarity(a, b)` — `1 − distance / max_len`. Same empty rules.
- `exact_match(a, b)` — 1.0 iff `a == b`.
- `combined_similarity(a, b)` — `0.7 × jw + 0.3 × lev`.
- `optional_field_score(opt1, opt2, algorithm)` — Both `None` ⇒ 1.0; one `None` ⇒ 0.0; both `Some` ⇒ algorithm applied.

`SimilarityAlgorithm` is a `Copy` enum: `JaroWinkler | Levenshtein | Exact | Combined`.

### 10.2a `NicknameTable` (in `nicknames.rs`)

Equivalence-class lookup table consulted by name scoring. Public API:

- `NicknameTable::empty()` — table with no classes.
- `NicknameTable::english()` — built-in table of common English nicknames (`Michael`/`Mike`, `Elizabeth`/`Liz`, `Robert`/`Bob`, …).
- `NicknameTable::with_class(names)` — append an equivalence class; entries are normalised via `Normalizer::normalize_name`; classes with fewer than two distinct normalised entries are silently dropped.
- `NicknameTable::are_equivalent(a, b)` — `true` iff both inputs normalise to the same string or share at least one equivalence class.
- `NicknameTable::is_empty()` / `len()` — observers.

Identical normalised strings are trivially equivalent (the table need not list them explicitly). The default English dictionary's exact contents are NOT part of the public contract — entries MAY be added in minor releases.

### 10.3 `MatchingEngine` (in `matcher.rs`)

Holds an immutable `MatchConfig`. Methods:

- `new(config)` / `default_config()` — constructors.
- `match_workers(&p1, &p2) -> MatchResult` — see §12.3.
- `deterministic_match(&p1, &p2) -> bool` — see §12.1.
- `match_one_to_many(&query, &[Worker]) -> Vec<MatchResult>` — score a single query against many candidates; parallel to input slice; see §12.6.
- `rank_one_to_many(&query, &[Worker]) -> Vec<(usize, MatchResult)>` — same as `match_one_to_many` but sorted by descending score with stable index tiebreak.

Private helpers compute each component score (`score_nhs_number`, `score_given_name`, etc.) and the address sub-score (`compare_addresses`).

---

## 11. Public API Specification

The following items are stable re-exports from `lib.rs` and constitute the supported API surface.

```rust
pub mod identifiers;  // 35 workeral-identifier parsers + 9 passport-format
                      // validators. See §10.1a for the full table; the
                      // module's rustdoc lists every parser at a glance.

pub use error::{MatchingError, Result};
pub use matcher::{Confidence, MatchConfig, MatchResult, MatchBreakdown, MatchingEngine};
pub use models::{Address, BloodType, Gender, PassportBook, Worker, WorkerBuilder};
pub use nicknames::NicknameTable;
pub use normalizer::{Normalizer, ParsedAddressLine};
pub use scorer::{Scorer, SimilarityAlgorithm};
```

**Stability rules:**

- `Worker` and `Address` carry `#[non_exhaustive]` (FR-53). External consumers MUST construct them via the builder (`Worker::builder()`) or the constructor + fluent setters (`Address::new().with_postcode(...)`) — struct-literal syntax is reserved for the defining crate. This lets future field additions ship as minor releases without breaking downstream code.
- Adding fields to `Worker`/`Address`: minor bump.
- Removing or renaming fields: major bump.
- Changing default weights: minor bump (with CHANGELOG entry under "Behaviour Change").
- Changing the meaning of `is_match` for the same `score`: major bump.

---

## 12. Algorithm Specifications

### 12.1 Deterministic Matching

`deterministic_match` returns `true` iff **any** of the following hold:

1. **UK NHS Number agreement.** Both records have a `uk_nhs_number`, both parse via `identifiers::parse_uk_nhs_number`, and the canonical forms are equal.
2. **France NIR agreement.** Both records have an `fr_nir`, both parse via `identifiers::parse_fr_nir`, and the canonical forms are equal.
3. **España TSI agreement.** Both records have an `es_tsi`, both parse via `identifiers::parse_es_tsi`, and the canonical forms are equal.
4. **Éire IHI agreement.** Both records have an `ie_ihi`, both parse via `identifiers::parse_ie_ihi`, and the canonical forms are equal.
5. **UK Northern Ireland H&C Number agreement.** Both records have a `uk_hc_number`, both parse via `identifiers::parse_uk_hc_number`, and the canonical forms are equal.
6. **US SSN agreement.** Both records have a `us_ssn`, both parse via `identifiers::parse_us_ssn`, and the canonical forms are equal.
7. **Australia IHI agreement.** Both records have an `au_ihi`, both parse via `identifiers::parse_au_ihi`, and the canonical forms are equal.
8. **Germany KVNR agreement.** Both records have a `de_kvnr`, both parse via `identifiers::parse_de_kvnr`, and the canonical forms are equal.
9. **Italy *Codice Fiscale* agreement.** Both records have an `it_cf`, both parse via `identifiers::parse_it_cf`, and the canonical forms are equal.
10. **Netherlands BSN agreement.** Both records have an `nl_bsn`, both parse via `identifiers::parse_nl_bsn`, and the canonical forms are equal.
11. **Sweden *Workernummer* agreement.** Both records have an `se_workernummer`, both parse via `identifiers::parse_se_workernummer`, and the canonical forms are equal.
12. **UK Scotland CHI Number agreement.** Both records have a `uk_chi_number`, both parse via `identifiers::parse_uk_chi_number`, and the canonical forms are equal.
13. **T-27 schemes agreement.** Same shape for `be_nn`, `bg_egn`, `cz_rc`, `dk_cpr`, `ee_ik`, `es_dni`, `fi_hetu`, `hr_oib`, `is_kt`, `lt_ak`, `lv_pk`, `mt_id`, `no_fnr`, `pl_pesel`, `ro_cnp`, `si_emso`, `sk_rc`, `uk_nino` — each is scheme-local and any pair with equal canonical form fires.
14. **T-28 schemes agreement.** Same shape for `gr_dss`, `li_id`, `nl_id`, `pl_nip`, `pt_nif`.
14a. **T-17.1 schemes agreement.** Same shape for `br_cpf`, `cn_rrn`, `in_aadhaar`, `jp_my_number`, `mx_curp`, `nz_nhi`, `za_id` — each is scheme-local and any pair with equal canonical form fires (FR-85..FR-91).
15. **Passport-book agreement.** At least one `(country, number)` pair is shared across the two workers' `passport_books` lists after the canonicalisation performed by `PassportBook::new` (FR-52). Cross-country values with the same `number` MUST NOT match.
16. **Demographic tuple agreement.**
   - Normalised given names equal AND
   - Normalised family names equal AND
   - Dates of birth are exactly equal AND
   - Genders are equal OR at least one is `None` (missing gender does not fail this branch).

Otherwise it returns `false`.

National identifiers are scheme-local: a UK NHS Number is only ever compared against another UK NHS Number, never against an H&C Number that happens to share the same 10 digits.

### 12.2 Component Scoring

| Field | Score function | Score domain |
|---|---|---|
| UK NHS Number | Exact equality of canonical form from `parse_uk_nhs_number`; both must parse. | `{0.0, 1.0}`, else `None` |
| France NIR | Exact equality of canonical form from `parse_fr_nir`; both must parse. | `{0.0, 1.0}`, else `None` |
| España TSI | Exact equality of canonical form from `parse_es_tsi`; both must parse. | `{0.0, 1.0}`, else `None` |
| Éire IHI | Exact equality of canonical form from `parse_ie_ihi`; both must parse. | `{0.0, 1.0}`, else `None` |
| UK NI H&C Number | Exact equality of canonical form from `parse_uk_hc_number`; both must parse. | `{0.0, 1.0}`, else `None` |
| US SSN | Exact equality of canonical form from `parse_us_ssn`; both must parse. | `{0.0, 1.0}`, else `None` |
| Australia IHI | Exact equality of canonical form from `parse_au_ihi`; both must parse. | `{0.0, 1.0}`, else `None` |
| Germany KVNR | Exact equality of canonical form from `parse_de_kvnr`; both must parse. | `{0.0, 1.0}`, else `None` |
| Italy *Codice Fiscale* | Exact equality of canonical form from `parse_it_cf`; both must parse. | `{0.0, 1.0}`, else `None` |
| Netherlands BSN | Exact equality of canonical form from `parse_nl_bsn`; both must parse. | `{0.0, 1.0}`, else `None` |
| Sweden *Workernummer* | Exact equality of canonical form from `parse_se_workernummer`; both must parse. | `{0.0, 1.0}`, else `None` |
| UK Scotland CHI | Exact equality of canonical form from `parse_uk_chi_number`; both must parse. | `{0.0, 1.0}`, else `None` |
| Passport book | `Some(1.0)` if any `(country, number)` pair is shared across `passport_books` on both sides; `Some(0.0)` if both non-empty but disjoint; `None` if either empty. See §6.4a. | `{0.0, 1.0}`, else `None` |
| Given name | `name_algorithm` applied to normalised strings; raised to `0.9` when both names appear in the same class of `MatchConfig::nickname_table`. When both workers have a `middle_name`, the final score is `0.95 × given_sim + 0.05 × middle_sim` (FR-49). | `[0.0, 1.0]` |
| Family name | Same as given name (table-driven boost applies symmetrically; default English table contains no family-name entries). | `[0.0, 1.0]` |
| Date of birth | Exact equality, or `0.5` for a same-year day/month transposition. | `{0.0, 0.5, 1.0}` |
| Gender | Exact equality. | `{0.0, 1.0}` |
| Blood type | Exact equality of `BloodType` enum value. Stable for life so disagreement is reliable evidence of non-match; weak positive signal because many people share a type. | `{0.0, 1.0}`, else `None` |
| Multiple birth | Exact equality of FHIR `Patient.multipleBirth` integer (1-indexed birth order). Primary purpose: disambiguate identical twins who otherwise share name + DOB + address. See FR-82. | `{0.0, 1.0}`, else `None` |
| Place of birth | City Jaro-Winkler blended with country exact match (`0.7 × city + 0.3 × country` when both present; single signal when only one); `None` when no comparable subset exists. See §12.4a. | `[0.0, 1.0]`, else `None` |
| Date of death | Exact equality, or `0.5` for a same-year day/month transposition (same heuristic as date of birth). See FR-83. | `{0.0, 0.5, 1.0}`, else `None` |
| Place of death | Same scoring rule as place of birth — city + country blend via the shared `score_named_place` helper. See FR-84 / §12.4a. | `[0.0, 1.0]`, else `None` |
| Address | Sub-score; see §12.4. | `[0.0, 1.0]` |
| Phone | Exact equality after normalisation. | `{0.0, 1.0}` |
| Email | Exact equality of canonical form from `normalize_email`; both must parse. | `{0.0, 1.0}`, else `None` |
| Phonetic names | Average of given-name and family-name Soundex equality. | `{0.0, 0.5, 1.0}` |

A component scores `None` whenever input data is missing or unparseable on either side.

### 12.3 Probabilistic Scoring

```text
weighted_sum   = Σ_field  score_field × weight_field   (over fields with score = Some)
total_weight   = Σ_field  weight_field                  (over the same fields)
if phonetic_score is Some(s) and s > 0.9:
    weighted_sum  += s × 0.05
    total_weight  += 0.05
score = weighted_sum / total_weight   (or 0.0 if total_weight == 0)
is_match = score >= match_threshold
```

Notes:
- Weights are renormalised against participating fields. A record with only name and DOB does NOT silently get a low score for "missing" NHS number — the missing field is simply not counted.
- The phonetic bonus is asymmetric: it only ever pushes the score up.

### 12.4 Address Sub-Score

Given both `Address` values, scores are computed where both sides have a value:

| Sub-component | Comparison | Weight in sub-score |
|---|---|---|
| Postcode | Exact equality of normalised postcode (`0.0` or `1.0`). | 0.5 |
| City | Jaro-Winkler on normalised city. | 0.3 |
| Line 1 | Structured sub-score on `(house_number, street)` — see below. | 0.2 |

#### 12.4.1 Line 1 Structured Sub-Score

For line 1, each side is parsed via `Normalizer::parse_address_line` into a `ParsedAddressLine { house_number, unit, street }`. The sub-score is computed as:

1. `street_sim` = Jaro-Winkler similarity of `parsed1.street` and `parsed2.street` (both are abbreviation-expanded and name-normalised, so `"High Street"` and `"High St"` produce equal strings and score `1.0`).
2. `house_score` = `Some(1.0)` if both `house_number`s are present and equal; `Some(0.0)` if both are present and differ; `None` if either is absent.
3. If `house_score` is `Some(h)`, the line-1 sub-score is `0.6 * street_sim + 0.4 * h`; otherwise it is `street_sim`.

The `unit` field (`"Flat 2A"`, `"Apt 5"`, …) is parsed and exposed on `ParsedAddressLine` but is intentionally **not** mixed into the line-1 sub-score: real-world data records unit information inconsistently (sometimes on `line1`, sometimes on `line2`, sometimes omitted), and weighting it would penalise legitimate matches between records that simply use different conventions. Consumers that need stricter unit-level matching can use `parse_address_line` directly.

Address sub-score = `Σ(score × weight) / Σ(weight)` over the contributions that fired, where the per-component weights are postcode = `0.5`, city = `0.3`, line 1 = `0.2`. If nothing fires, **0.5** is returned (neutral). This is the weighted-average form: each sub-component contributes a raw `[0.0, 1.0]` score, the matcher accumulates `score × weight` and `weight` separately, then divides at the end so postcode dominates as documented and the result is bounded in `[0.0, 1.0]` independent of how many sub-components fired. Resolves §22 OQ-4 (T-3).

#### 12.4.2 Best-of Across Historical Addresses

For `MatchBreakdown::address_score`, the engine considers every pair drawn from `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)`. Each pair is scored via the §12.4 algorithm and the **highest** score across the cartesian product is reported. This catches the "worker moved house" failure mode where the current addresses no longer agree but a prior address still matches the other side.

`address_score` is `None` only when **at least one side has no address data at all** (neither current nor historical). If one side has only a `previous_addresses` entry and the other has only a current `address`, they are compared and a score is produced (FR-48; addresses the §21.2 medium-term roadmap item).

For very large `previous_addresses` lists, the cartesian product can grow quadratically. In practice records carry at most 2–3 historical addresses; consumers that ingest large histories SHOULD trim the list before matching.

### 12.4a Place-of-Birth Sub-Score

`MatchingEngine::score_birth_place` consumes `Worker::birth_place: Option<Address>` (FHIR `Patient.birthPlace`). Unlike the current-address sub-score (§12.4), it considers only the `city` and `country` sub-fields — street and postcode are not meaningful for a birth place. The implementation delegates to the shared `score_named_place` free helper, which is also used by `score_death_place` (§12.4b).

Algorithm:

1. If either side has no `birth_place`, return `None`.
2. Let `city = Jaro-Winkler(normalize_name(p1.city), normalize_name(p2.city))` when both sides have a city, else `None`.
3. Let `country = 1.0` if both `country` strings normalise equal, `0.0` if both are present but differ, `None` if either is absent.
4. Blend:
   - Both present: `0.7 × city + 0.3 × country`.
   - Only city: `city`.
   - Only country: `country`.
   - Neither: `None`.

Diacritics are absorbed by the shared name-normalisation pipeline (so `"Zürich"` and `"Zurich"` score identically). The sub-score is bounded `[0.0, 1.0]`.

### 12.4b Place-of-Death Sub-Score

`MatchingEngine::score_death_place` consumes `Worker::death_place: Option<Address>`. The algorithm is identical to the place-of-birth sub-score (§12.4a) — both go through the shared `score_named_place(&Address, &Address) -> Option<f64>` free helper. The same city/country blend (`0.7 × city + 0.3 × country`, single-signal fallbacks) applies. The sub-score is bounded `[0.0, 1.0]`, returns `None` when neither side has comparable sub-fields, and is independent from the `birth_place` and `address` sub-scores (FR-84).

### 12.4c Date-of-Death Sub-Score

`MatchingEngine::score_death_date` consumes `Worker::death_date: Option<NaiveDate>` and reuses the existing `score_dob_pair` free helper: exact equality yields `1.0`, a same-year day/month transposition yields `0.5`, otherwise `0.0`. Returns `None` when either side is absent. The transposition heuristic is justified by the same DD/MM ↔ MM/DD data-entry-error mode that motivates FR-12 for the date of birth (FR-83).

### 12.5 Confidence Bands

`MatchResult::confidence` is a fixed-band classification of `score`. It is independent of `match_threshold`: the same `score` always maps to the same band regardless of preset.

| Confidence | Score range |
|---|---|
| `High` | `score >= 0.90` |
| `Medium` | `0.75 <= score < 0.90` |
| `Low` | `score < 0.75` |

Boundaries are inclusive on the low side (a score of exactly `0.90` is `High`; exactly `0.75` is `Medium`). `Confidence::from_score(f64) -> Confidence` is total over `f64`: NaN and negative scores degrade to `Low`; scores above `1.0` are `High`. Bands are consultative — `is_match` remains the authoritative go/no-go signal.

---

### 12.6 Batch Scoring

`MatchingEngine::match_one_to_many(query, candidates)` iterates `candidates` and produces one `MatchResult` per candidate via the same `match_workers` pipeline (§12.3). The output `Vec<MatchResult>` is parallel to the input slice; index `i` in the output corresponds to index `i` in `candidates`. Empty candidates yield an empty `Vec`.

`MatchingEngine::rank_one_to_many(query, candidates)` returns a `Vec<(usize, MatchResult)>` where the `usize` is the original index in `candidates`. The vector is sorted by descending `MatchResult::score`. Ties are broken by ascending original index so the ranking is fully deterministic across calls.

Neither function performs blocking (candidate pre-filtering). Consumers that need blocking — e.g. only score candidates whose family-name Soundex equals the query's, or whose postcode outward code matches — MUST pre-filter the slice themselves. This keeps the crate a pure scoring library with no implicit indexing strategy.

The engine is `Send + Sync`, so parallel batch scoring (`rayon::par_iter`, `tokio::task::spawn_blocking`, …) is the consumer's choice. The crate intentionally does not take a parallelism dependency.

## 13. Configuration Specification

### 13.1 Default Configuration

| Parameter | Default | Strict | Lenient |
|---|---|---|---|
| `match_threshold` | **0.85** | 0.95 | 0.75 |
| `uk_nhs_number_weight` | 0.30 | 0.30 | 0.30 |
| `fr_nir_weight` | 0.30 | 0.30 | 0.30 |
| `es_tsi_weight` | 0.30 | 0.30 | 0.30 |
| `ie_ihi_weight` | 0.30 | 0.30 | 0.30 |
| `uk_hc_number_weight` | 0.30 | 0.30 | 0.30 |
| `us_ssn_weight` | 0.30 | 0.30 | 0.30 |
| `au_ihi_weight` | 0.30 | 0.30 | 0.30 |
| `de_kvnr_weight` | 0.30 | 0.30 | 0.30 |
| `it_cf_weight` | 0.30 | 0.30 | 0.30 |
| `nl_bsn_weight` | 0.30 | 0.30 | 0.30 |
| `se_workernummer_weight` | 0.30 | 0.30 | 0.30 |
| `uk_chi_number_weight` | 0.30 | 0.30 | 0.30 |
| (T-27 weights: `be_nn_weight`, `bg_egn_weight`, `cz_rc_weight`, `dk_cpr_weight`, `ee_ik_weight`, `es_dni_weight`, `fi_hetu_weight`, `hr_oib_weight`, `is_kt_weight`, `lt_ak_weight`, `lv_pk_weight`, `mt_id_weight`, `no_fnr_weight`, `pl_pesel_weight`, `ro_cnp_weight`, `si_emso_weight`, `sk_rc_weight`, `uk_nino_weight`) | 0.30 | 0.30 | 0.30 |
| (T-28 weights: `gr_dss_weight`, `li_id_weight`, `nl_id_weight`, `pl_nip_weight`, `pt_nif_weight`) | 0.30 | 0.30 | 0.30 |
| (T-17.1 weights: `br_cpf_weight`, `cn_rrn_weight`, `in_aadhaar_weight`, `jp_my_number_weight`, `mx_curp_weight`, `nz_nhi_weight`, `za_id_weight`) | 0.30 | 0.30 | 0.30 |
| `passport_book_weight` | 0.30 | 0.30 | 0.30 |
| `given_name_weight` | 0.15 | 0.15 | 0.15 |
| `family_name_weight` | 0.20 | 0.20 | 0.20 |
| `date_of_birth_weight` | 0.20 | 0.20 | 0.20 |
| `gender_weight` | 0.05 | 0.05 | 0.05 |
| `blood_type_weight` | 0.05 | 0.05 | 0.05 |
| `multiple_birth_weight` | 0.05 | 0.05 | 0.05 |
| `address_weight` | 0.05 | 0.05 | 0.05 |
| `birth_place_weight` | 0.05 | 0.05 | 0.05 |
| `death_date_weight` | 0.10 | 0.10 | 0.10 |
| `death_place_weight` | 0.05 | 0.05 | 0.05 |
| `phone_weight` | 0.05 | 0.05 | 0.05 |
| `email_weight` | 0.05 | 0.05 | 0.05 |
| `use_phonetic_matching` | true | true | true |
| `name_algorithm` | `Combined` | `Combined` | `Combined` |
| `strict_mode` | false | true | false |
| `nickname_table` | `NicknameTable::empty()` | `NicknameTable::empty()` | `NicknameTable::empty()` |
| `gmail_dot_folding` | false | false | false |
| `phone_default_country` | `Some("GB")` | `Some("GB")` | `Some("GB")` |

National-identifier weights all default to `0.30` and are renormalised against the participating fields, so a worker with only one identifier scheme on both sides gets the same effective weighting from that scheme as the pre-multinational behaviour gave to NHS Number alone.

### 13.2 `strict_mode` Semantics

When `strict_mode` is `true` the engine still computes the full probabilistic `score` and `confidence`, but tightens the binary `is_match` decision to also require a deterministic match. Specifically:

```text
is_match = (score >= match_threshold) && deterministic_match(p1, p2)
```

A fuzzy match that lifts the score above the threshold but lacks a deterministic anchor (no identifier agreement, no full demographic-tuple agreement) is rejected as `is_match = false` under strict mode. This narrows the false-positive surface for clinical workflows where the threshold alone is too permissive. Consumers reading `MatchBreakdown` directly are unaffected — the per-field scores and the overall `score` are computed identically across strict and non-strict configurations.

---

## 14. Normalization Specification

### 14.1 Name Normalization

Algorithm:

1. NFKD-decompose the input.
2. Drop characters classified as Unicode combining marks (`unicode_normalization::char::is_combining_mark`).
3. Drop ASCII punctuation (`char::is_ascii_punctuation`).
4. Lowercase the result.
5. Collapse runs of whitespace into single spaces; trim ends.

Examples:

| Input | Output |
|---|---|
| `"  John  Smith  "` | `"john smith"` |
| `"O'Brien"` | `"obrien"` |
| `"José"` | `"jose"` |
| `"MARY-JANE"` | `"maryjane"` |
| `"Siân"` | `"sian"` |

### 14.2 Postcode Normalization

1. Drop whitespace characters.
2. Uppercase.

`"CF10 1AA"` ⇒ `"CF101AA"`. `"cf10 1aa"` ⇒ `"CF101AA"`.

### 14.3 Phone Normalization

Two complementary normalisers cover phone-number handling:

#### 14.3.1 Legacy national-significant form — `Normalizer::normalize_phone`

This is the UK-centric, infallible form used as a fallback by the matcher when the international form cannot parse.

1. Keep only ASCII digits.
2. If result starts with `0044` and is longer than 4 digits, drop the `0044` prefix.
3. Else, if result starts with `44` and is at least 12 digits, drop the `44` prefix.
4. Else, if result starts with `0` and is longer than 1 digit, drop the leading `0`.
5. Return the result.

Examples:

| Input | Output |
|---|---|
| `"07700 900123"` | `"7700900123"` |
| `"+44 7700 900123"` | `"7700900123"` |
| `"0044 7700 900123"` | `"7700900123"` |
| `"(029) 2034 5678"` | `"2920345678"` |

#### 14.3.2 International E.164 form — `Normalizer::normalize_phone_e164`

Returns `Some("+CCNNN…")` when the input parses against a country in the supported table, otherwise `None`. The function accepts:

- `+CC…` — explicit international, the canonical input form.
- `00CC…` — international access code (common across Europe).
- `0…` — national format with a national trunk prefix; interpreted relative to `default_country` (passed by the caller, sourced from `MatchConfig::phone_default_country` in the matcher).
- `NSN…` — bare national-significant number; interpreted relative to `default_country`.

Algorithm:

1. Strip every character that is not an ASCII digit; remember whether the original input contained `+`.
2. If `+` was present, match the longest dial-code prefix from the supported table against the leading digits.
3. Else, if the digits begin with `00`, drop those two and match the longest dial-code prefix against what remains.
4. Else, if a `default_country` is supplied, look it up in the table.
5. If no country is found, return `None`.
6. Strip a single occurrence of the country's national trunk prefix from the remaining digits, if one is configured. The trunk prefix is country-specific (typically `"0"`, but Lithuania uses `"8"`); the field on `CountryPhoneInfo` is `trunk_prefix: Option<&'static str>`.
7. Reject when the remaining national-significant number is outside the country's `min_nsn..=max_nsn` length range.
8. Return `Some(format!("+{dial_code}{nsn}"))`.

Supported countries (ISO 3166-1 alpha-2 code, dial code, trunk prefix, NSN range; **39 jurisdictions** — one for every national identifier scheme the crate parses):

`GB +44 trunk-0 7..=11`, `FR +33 trunk-0 9..=9`, `DE +49 trunk-0 7..=13`, `ES +34 no-trunk 9..=9`, `IE +353 trunk-0 7..=11`, `IT +39 no-trunk 6..=12`, `NL +31 trunk-0 9..=9`, `BE +32 trunk-0 8..=9`, `PT +351 no-trunk 9..=9`, `CH +41 trunk-0 9..=9`, `AT +43 trunk-0 4..=13`, `SE +46 trunk-0 7..=13`, `NO +47 no-trunk 8..=8`, `DK +45 no-trunk 8..=8`, `FI +358 trunk-0 5..=12`, `PL +48 no-trunk 9..=9`, `AU +61 trunk-0 9..=9`, `NZ +64 trunk-0 8..=10`, `US +1 no-trunk 10..=10`, `CA +1 no-trunk 10..=10`, `JP +81 trunk-0 9..=10`, `CN +86 trunk-0 5..=12`, `IN +91 trunk-0 10..=10`, `BR +55 trunk-0 10..=11`, `MX +52 no-trunk 10..=10`, `ZA +27 trunk-0 9..=9`,

`BG +359 trunk-0 8..=9`, `CZ +420 no-trunk 9..=9`, `EE +372 no-trunk 7..=8`, `GR +30 no-trunk 10..=10`, `HR +385 trunk-0 8..=9`, `IS +354 no-trunk 7..=9`, `LI +423 no-trunk 7..=9`, **`LT +370 trunk-8 8..=8`**, `LV +371 no-trunk 8..=8`, `MT +356 no-trunk 8..=8`, `RO +40 trunk-0 9..=9`, `SI +386 trunk-0 8..=8`, `SK +421 trunk-0 9..=9` (added in T-19).

New countries SHOULD be added with explicit trunk-prefix and NSN-range provenance. Lithuania is the canonical example of a non-`0` trunk prefix; the abstraction (`Option<&'static str>`) supports any documented convention.

Examples (with `default_country = Some("GB")` unless noted):

| Input | `default_country` | Output |
|---|---|---|
| `"+44 7700 900123"` | any | `Some("+447700900123")` |
| `"0044 7700 900123"` | any | `Some("+447700900123")` |
| `"07700 900123"` | `"GB"` | `Some("+447700900123")` |
| `"07700 900123"` | `None` | `None` (ambiguous) |
| `"+33 1 23 45 67 89"` | any | `Some("+33123456789")` |
| `"01 23 45 67 89"` | `"FR"` | `Some("+33123456789")` |
| `"912 345 678"` | `"ES"` | `Some("+34912345678")` |
| `"(415) 555-1234"` | `"US"` | `Some("+14155551234")` |
| `"+999 1234567"` | any | `None` (unknown dial code) |
| `""` | any | `None` |

NANP (`+1`) numbers are returned with US's `iso_alpha2` because both US and CA share the same dial code; the canonical E.164 output is identical for both jurisdictions, which is the property the matcher relies on.

### 14.3a Email Normalization

`Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` returns the canonical lowercase form of an email address, or `None` when the input is structurally invalid.

Algorithm:

1. Trim surrounding whitespace.
2. Lowercase the entire address (RFC 5321 makes the domain case-insensitive; healthcare data overwhelmingly treats the localpart case-insensitively too).
3. Split on `@`. Reject (`None`) unless there is exactly one `@` and both localpart and domain are non-empty.
4. If `gmail_dot_folding` is `true` and the domain is `gmail.com` or `googlemail.com`:
   - Truncate the localpart at the first `+` (drops `+tag` suffix).
   - Remove every `.` from the localpart.
   - Reject if the resulting localpart is empty.

Examples:

| Input | `gmail_dot_folding` | Output |
|---|---|---|
| `"  Alice@Example.ORG  "` | any | `Some("alice@example.org")` |
| `"j.smith@gmail.com"` | `false` | `Some("j.smith@gmail.com")` |
| `"j.smith@gmail.com"` | `true` | `Some("jsmith@gmail.com")` |
| `"jsmith+work@gmail.com"` | `true` | `Some("jsmith@gmail.com")` |
| `"j.smith@example.org"` | `true` | `Some("j.smith@example.org")` (not Gmail; no folding) |
| `"no-at-sign"` | any | `None` |
| `"@example.org"` | any | `None` |
| `"a@b@c"` | any | `None` |
| `""` | any | `None` |

`MatchingEngine::match_workers` calls `normalize_email` on both sides; `MatchBreakdown::email_score` is `Some(1.0)` for equal canonical forms, `Some(0.0)` for distinct canonical forms when both parse, and `None` when either input is absent or fails to parse.

`local_id` is **not** scored. Different organisations may issue colliding values (a worker's MRN at hospital A and another worker's MRN at hospital B can be byte-equal), so positional matching would produce false positives. This resolves §22 OQ-2's second half.

### 14.4a Address Line Normalization

`Normalizer::normalize_address_line(line)` and `Normalizer::parse_address_line(line)` are the two public entry points for structural address handling.

#### 14.4a.1 Abbreviation expansion — `expand_street_abbreviations`

Tokenise on whitespace. For each token, strip at most one trailing `.` or `,` and look up the result case-insensitively in the **street-abbreviation table**:

| Abbreviation | Expansion | Abbreviation | Expansion |
|---|---|---|---|
| `st`, `str` | `street` | `n` | `north` |
| `rd` | `road` | `s` | `south` |
| `ave`, `av` | `avenue` | `e` | `east` |
| `blvd`, `bvd` | `boulevard` | `w` | `west` |
| `ln` | `lane` | `ne` | `northeast` |
| `dr` | `drive` | `nw` | `northwest` |
| `ct` | `court` | `se` | `southeast` |
| `pl` | `place` | `sw` | `southwest` |
| `sq` | `square` | | |
| `ter`, `terr` | `terrace` | `hwy` | `highway` |
| `pkwy` | `parkway` | `mt` | `mount` |
| `mtn` | `mountain` | `cres` | `crescent` |
| `gdns` | `gardens` | `gdn` | `garden` |
| `gr` | `grove` | `cl` | `close` |
| `pk` | `park` | `plz` | `plaza` |
| `expy` | `expressway` | `trl` | `trail` |

Matched tokens are replaced with the lower-case long form; unrecognised tokens pass through unchanged. Tokens are re-joined by single spaces.

The expansion is **always token-level** and does not apply position-aware heuristics. The well-known ambiguous case `"St"` (Saint vs Street) is always expanded to `street`; the resulting canonical form is consistent on both sides of a comparison.

#### 14.4a.2 Address-line normalisation — `normalize_address_line`

`expand_street_abbreviations(line) → normalize_name(...)`. Idempotent.

Examples:

| Input | Output |
|---|---|
| `"123 High St"` | `"123 high street"` |
| `"45 N Park Ave"` | `"45 north park avenue"` |
| `"10, DOWNING Street."` | `"10 downing street"` |

#### 14.4a.3 Address-line parsing — `parse_address_line`

Returns `ParsedAddressLine { house_number: Option<String>, unit: Option<String>, street: String }`.

Algorithm:

1. Trim leading whitespace.
2. **Unit prefix**: read the first whitespace-separated token. Strip at most one trailing `.` or `,`. If the lowercase form matches one of `flat`, `apartment`, `apt`, `unit`, `suite`, `ste`, `room`, `rm`, read the next alphanumeric run as the unit identifier. Store `format!("{keyword} {identifier}")` lowercased.
3. Skip a single leading `,` and any whitespace.
4. **House number**: read the leading run of ASCII digits; if non-empty, also consume a single trailing ASCII alphabetic character (e.g. `"10A"`) **only when not followed by another alphanumeric** (otherwise we would absorb the first letter of the street name, as in `"10 Apple Tree Lane"`). Uppercase the result.
5. Skip a single leading `,` and any whitespace.
6. **Street**: `normalize_address_line` of the remainder.

`ParsedAddressLine` is `Serialize + Deserialize` and re-exported from the crate root.

#### 14.4a.4 Limitations and pitfalls

- `"St"` (Saint) is always expanded to `street`. The canonical form is consistent on both sides; fuzzy matching tolerates the resulting inconsistency.
- Multi-line addresses are not parsed; consumers must split them upstream.
- The unit prefix dictionary is English-language. Non-English unit terms (`"Wohnung"`, `"Appartement"`) are not recognised and are passed through verbatim into the street field.
- House numbers that include hyphens (`"123-125 High St"`) are partially parsed: the leading number is captured but the range information is dropped into the street remainder.

### 14.4b Matcher Integration

`MatchingEngine::compare_addresses` calls `Normalizer::parse_address_line` on both `line1` strings and combines the street similarity with the house-number exact-match score as documented in §12.4.1. The `city` and `postcode` comparisons are unchanged.

#### 14.3.3 Matcher integration

`MatchingEngine::match_workers` consults E.164 first and falls back to the legacy form. Specifically, with phone strings `phone1` and `phone2` and default country `cc = MatchConfig::phone_default_country`:

1. Compute `e1 = normalize_phone_e164(phone1, cc)` and `e2 = normalize_phone_e164(phone2, cc)`.
2. If both are `Some`, score `phone_score = 1.0 if e1 == e2 else 0.0`.
3. Otherwise compare `normalize_phone(phone1) == normalize_phone(phone2)`.

This preserves the prior single-country behaviour for inputs the country table does not cover, while adding cross-country disambiguation for inputs it does.

### 14.4 Phonetic Code

1. Apply name normalisation (§14.1).
2. If empty, return empty.
3. Apply `soundex::american_soundex`.

The "American" Soundex is used pragmatically; a locale-aware phonetic algorithm (Double Metaphone, NYSIIS, or similar) is tracked in §21 as a candidate replacement or augmentation.

### 14.5 National Identifier Normalization

Each scheme has its own canonical form. Two inputs that represent the same identifier in different textual layouts MUST canonicalise to the same string.

**UK NHS Number** (`parse_uk_nhs_number`):
1. Delegated to `nhs_number::NHSNumber::from_str`, which accepts the 10-digit compact form (`"9434765919"`) or the 12-character spaced form (`"943 476 5919"`).
2. Canonical form: 10 digits, no spaces.

**France NIR** (`parse_fr_nir`):
1. Strip all Unicode whitespace.
2. Uppercase letters.
3. Reject unless the result is ASCII and exactly 15 characters.
4. Build a numeric body from positions 0..13: if positions 5..7 are `"2A"` replace with `"19"`; if `"2B"` replace with `"18"`; otherwise require all 13 characters to be digits.
5. Reject unless positions 13..15 are both ASCII digits.
6. Validate `97 - (N mod 97) == key`, where `N` is the numeric body parsed as `u64` and `key` is positions 13..15.
7. Canonical form: the cleaned, uppercased 15-character string.

**España TSI / CIP-SNS** (`parse_es_tsi`):
1. Strip Unicode whitespace and ASCII hyphens (`-`).
2. Uppercase letters.
3. Reject unless the result is ASCII, contains only ASCII alphanumerics, and has length in `10..=20`.
4. Canonical form: the cleaned, uppercased string.

**Éire IHI** (`parse_ie_ihi`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 7 digits.
3. Canonical form: the 7-digit string.

**UK NI H&C Number** (`parse_uk_hc_number`):
1. Identical algorithm to UK NHS Number (`parse_uk_nhs_number`).
2. Exposed as a distinct function so that the calling code retains scheme provenance — an NHS Number and an H&C Number with the same 10 digits refer to different workers in different registries and MUST NOT cross-match.

**US SSN** (`parse_us_ssn`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 9 digits.
3. Reject if the area number (digits 0..3) is `000`, `666`, or in `900..=999` — those have never been issued by the Social Security Administration.
4. Reject if the group number (digits 3..5) is `00`.
5. Reject if the serial number (digits 5..9) is `0000`.
6. Canonical form: the 9-digit compact string `"AAAGGSSSS"`.
7. No geographic decoding is performed (SSA assignment has been randomised since June 2011) and no check-digit calculation is performed (none exists in the public specification).

**Australia IHI** (`parse_au_ihi`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 16 digits.
3. Apply the Luhn algorithm (ISO/IEC 7812-1) over all 16 digits with weights `2, 1, 2, 1, …` from the left; products `≥ 10` reduced by digit-sum.
4. Canonical form: the 16-digit compact string. The structural convention that real IHIs begin with `800360` is NOT enforced.

**Germany KVNR** (`parse_de_kvnr`):
1. Strip whitespace; uppercase letters.
2. Reject unless the result is ASCII and has exactly 10 characters: one letter followed by 9 digits.
3. Map the leading letter to a 2-digit ordinal (`A=01`, `B=02`, …, `Z=26`); concatenate with positions 2..=9 of the KVNR → 10 digits.
4. Apply alternating weights `1, 2, 1, 2, …, 1, 2`; reduce products `≥ 10` by digit-sum; sum.
5. The check digit (position 10 of the KVNR) MUST equal `sum mod 10`.
6. Canonical form: the 10-character uppercase string.

**Italy *Codice Fiscale*** (`parse_it_cf`):
1. Strip whitespace; uppercase letters.
2. Reject unless the result is ASCII, exactly 16 characters, and entirely alphanumeric.
3. For each of the first 15 characters, look up a numeric value via the standard tables: odd-positioned characters (1-indexed positions 1, 3, …, 15) use the scattered "odd" table; even-positioned characters (2, 4, …, 14) map digits/letters to their natural value.
4. Sum the 15 values; take mod 26.
5. Map `0..=25` to `A..=Z`. The result MUST equal the 16th character.
6. Canonical form: the 16-character uppercase string.

**Netherlands BSN** (`parse_nl_bsn`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 9 digits.
3. Reject the all-zero string `000000000`.
4. Apply the "11-test": `9·d₁ + 8·d₂ + 7·d₃ + 6·d₄ + 5·d₅ + 4·d₆ + 3·d₇ + 2·d₈ − d₉ ≡ 0 (mod 11)`.
5. Canonical form: the 9-digit compact string.

**Sweden *Workernummer*** (`parse_se_workernummer`):
1. Keep only ASCII digits.
2. Accept exactly 10 or 12 digits; reject anything else.
3. For Luhn validation use the 10-digit form (drop the leading century from a 12-digit input).
4. Apply Luhn with weights `2, 1, 2, 1, …` from the left over the 10 digits; products `≥ 10` reduced by digit-sum; the total mod 10 must be `0`.
5. Canonical form preserves the input length: 10-digit input yields a 10-character string; 12-digit input yields a 12-character string. Records using mixed layouts will not deterministically match on this field.

**UK Scotland CHI Number** (`parse_uk_chi_number`):
1. Keep only ASCII digits.
2. Reject unless the result has exactly 10 digits.
3. Multiply the first 9 digits by weights `10, 9, 8, 7, 6, 5, 4, 3, 2`; sum; take mod 11.
4. The check digit (position 10) MUST equal `(11 − (sum mod 11)) mod 11`. A computed check of `10` indicates an invalid identifier and is rejected.
5. Canonical form: the 10-digit compact string.
6. The CHI Number shares the Mod-11 algorithm with the UK NHS Number and UK NI H&C Number but is scheme-local; cross-scheme matching is forbidden by FR-13 / §12.1.

---

## 15. Error Model

`MatchingError` is a `thiserror`-derived enum with `#[non_exhaustive]` so future variants do not break SemVer for downstream pattern-matches:

```text
MissingField(String)
```

`type Result<T> = std::result::Result<T, MatchingError>;`

`MissingField` is currently the only variant — it is returned by `Worker::validate` when neither a name nor an identifier is populated. The matching engine itself is infallible: scoring two workers always produces a `MatchResult`. Identifier parsers in the `identifiers` module return `Option<String>` rather than `Result`, on purpose: the parser is the source of truth on validity and the consumer should not need to triage a separate `MatchingError` variant for "this looked like a UK NHS Number but failed its check digit". Configuration builders (`MatchConfig::default`, `strict`, `lenient`) are infallible. The earlier `InvalidData`, `InvalidNhsNumber`, `InvalidDate`, and `ConfigError` variants were removed as part of T-13 because they were never returned from any code path; see §22 OQ-6.

---

## 16. Serialization Contract

- All public types in §11 except `MatchingEngine` MUST be `Serialize + Deserialize`.
- JSON is the reference format. `serde_json` is a hard dependency.
- Optional fields MUST round-trip as `null` ⇄ `None`.
- Dates MUST be serialised as ISO-8601 strings via `chrono`'s default `serde` feature.
- `MatchConfig` carries `#[serde(default)]` on the struct so a partial JSON document (overriding a subset of fields) deserialises with the remaining fields filled from `MatchConfig::default()`. This makes production deployments that load config from a file ergonomic without coupling them to the full schema.
- `SimilarityAlgorithm` serialises as the bare variant name (`"JaroWinkler"`, `"Levenshtein"`, `"Exact"`, `"Combined"`).
- `NicknameTable` serialises as `{ "classes": [["michael", "mike", "mickey"], …] }` — entries are pre-normalised at insertion time, so the round-trip is byte-stable.

---

## 17. Quality Attributes

| Attribute | Target | Verification |
|---|---|---|
| Correctness | Behaviour matches §12. | Unit + integration tests (§18). |
| Explainability | Every score has a per-field breakdown. | `MatchBreakdown` returned on every call. |
| Performance | `< 50 µs` per `match_workers` on a 2024-era Mac. | `benches/match_pair.rs` (criterion, §23 T-5); single-pair fuzzy match measures ~4 µs. |
| Maintainability | No single file > 500 lines (`matcher.rs` exempt pending refactor). | Periodic review. |
| Portability | Pure Rust, no C deps beyond `chrono`/`strsim` defaults. | `cargo build` on Linux + macOS. |
| Auditability | All score combinations are documented (§12). | This spec. |

---

## 18. Testing Strategy

### 18.1 Test Pyramid

| Layer | Location | Purpose |
|---|---|---|
| Unit tests | `src/*.rs` `#[cfg(test)]` modules | Verify each function in isolation. |
| Integration tests | `tests/integration_tests.rs` | Exercise the public API end-to-end. |
| Doctests | `///` examples in `lib.rs` and elsewhere | Keep README/usage examples honest. |
| Examples | `examples/basic_usage.rs`, `examples/custom_config.rs` | Smoke-test ergonomics. |

### 18.2 Required Scenarios

Each MUST be covered by at least one test:

1. Perfect match across all fields.
2. UK NHS Number mismatch with otherwise-matching demographics.
3. Typographic given-name variants (`Jon` vs `John`).
4. Phonetic equivalents (`Stephen` vs `Steven`).
5. Diacritic equivalence (e.g. `Siân` vs `Sian`, `José` vs `Jose`).
6. Apostrophes in family names (`O'Connor` vs `OConnor`).
7. Abbreviated address line 1 (`Street` vs `St`).
7.1. Address parsing: house-number extraction (`"10A Downing St"` → number `"10A"`, street `"downing street"`).
7.2. Address parsing: unit prefix recognition (`"Flat 2A, 10 Downing Street"` → unit `"flat 2a"`, number `"10"`, street `"downing street"`).
7.3. Address parsing: directional abbreviation (`"45 N Park Ave"` ↔ `"45 North Park Avenue"`).
7.4. Address parsing: mismatching house number penalises the address sub-score relative to matching house numbers.
8. Phone normalisation across country-code / trunk-prefix variants (`+44…`, `0044…`, `07…`).
8.1. International phone canonicalisation: `+CC` and national-format inputs canonicalise to the same E.164 string within a country (UK, FR, ES, IE, DE at minimum).
8.2. International phone disambiguation: numbers from different countries that share the same national-significant digits do not collide.
8.3. International phone fallback: when the country table cannot parse the input, the matcher falls back to the legacy national-significant comparison without losing existing behaviour.
9. Deterministic match by UK NHS Number alone (different names).
10. Deterministic match by France NIR alone.
11. Deterministic match by España TSI alone.
12. Deterministic match by Éire IHI alone.
13. Deterministic match by UK NI H&C Number alone.
14. Deterministic match by US SSN alone.
15. Deterministic match by demographics alone (no identifier).
16. UK NHS Number and UK NI H&C Number with the same digits MUST NOT cross-match.
17. France NIR Modulus-97 check rejects values with wrong key.
18. France NIR handles Corsica department `"2A"` and `"2B"`.
19. España TSI normalises whitespace, hyphens, and lower-case to a canonical upper-case ASCII alphanumeric form.
20. Éire IHI strips non-digit characters and requires exactly 7 digits.
21. US SSN rejects structurally-invalid area (`000` / `666` / `900..=999`), group (`00`), and serial (`0000`) values.
22. US SSN accepts hyphenated, spaced, and compact textual layouts as equivalent.
22.1. Australia IHI: 16 digits with Luhn validation; deterministic match on identifier alone; whitespace-tolerant; does not cross-match the 7-digit Ireland IHI.
22.2. Germany KVNR: letter + 9 digits with Mod-10 letter-ordinal check; case-insensitive on the leading letter; whitespace-tolerant.
22.3. Italy *Codice Fiscale*: 16 alphanumerics with Mod-26 check via odd/even tables; case-insensitive; whitespace-tolerant.
22.4. Netherlands BSN: 9 digits with the 11-test; rejects the all-zero string.
22.5. Sweden *Workernummer*: 10-digit and 12-digit textual layouts canonicalise distinctly; both validate the Luhn check over the 10-digit form; 10-digit form accepts `-` or `+` separators.
22.6. UK Scotland CHI Number: 10 digits with Mod-11 check; scheme-local and does not cross-match either UK NHS Number or UK NI H&C Number even when the digits agree.
23. Strict mode rejects nicknames (`Michael` vs `Mike`).
24. Lenient mode admits more partial matches.
25. Swapped day/month dates of birth → probabilistic DOB sub-score is `0.5` (transposition heuristic); `deterministic_match` still rejects; overall `is_match` still false under the default threshold.
25.1. Transposition heuristic does not fire across years: `1995-01-10` vs `1996-10-01` scores `0.0`.
25.2. Transposition heuristic does not relax `deterministic_match`: a transposed DOB still blocks the demographic-tuple branch.
26. Missing fields handled gracefully.
27. Completely different workers → low score, no match.
28. `Worker::validate` rejects empty workers.
29. `Worker::validate` accepts a worker carrying only a single national identifier (any scheme).
30. `Worker` and `MatchResult` round-trip via JSON, including all six national-identifier fields.
31. Nickname dictionary: `Mike` ↔ `Michael`, `Bob` ↔ `Robert`, `Liz` ↔ `Elizabeth` lift the given-name score to ≥ 0.9 when `MatchConfig::nickname_table = NicknameTable::english()`.
32. Nickname boost is a one-way lift: it never lowers an already-higher score.
33. Default config has an empty nickname table, so existing scores are unchanged for callers that have not opted in.
34. Email scoring: case- and whitespace-insensitive exact equality; mismatch scores `Some(0.0)`; unparseable input on either side scores `None`.
35. Gmail dot-folding is opt-in: `j.smith@gmail.com` ≡ `jsmith@gmail.com` only when `MatchConfig::gmail_dot_folding = true`; non-Gmail domains are untouched.
36. `local_id` is intentionally not scored (no `local_id_score` in `MatchBreakdown`).

### 18.3 Coverage Goals

- Statement coverage SHOULD be `>= 90%` on `src/`.
- Every public function MUST have at least one direct test or doctest.
- `cargo test` MUST complete in `< 5 s` on commodity hardware (regression budget).

### 18.4 Property Tests

Delivered as task T-6. The harness lives in `tests/property_tests.rs` and uses `proptest` with **1000 cases per property** (the case count is pinned in `proptest_config!`). Properties covered:

- `normalize_name` is idempotent (`normalize_name(normalize_name(s)) == normalize_name(s)`).
- `normalize_name` output carries no ASCII uppercase and no leading / trailing whitespace.
- `score ∈ [0.0, 1.0]` for arbitrary `Worker` pairs (the unit-interval invariant downstream services rely on).
- `match_workers(p, p).is_match == true` for any `p` passing `validate()`.
- Self-match always lands in `Confidence::High`.
- `match_workers` is symmetric in its arguments (score, `is_match`, and `confidence` are all argument-order-independent).
- `deterministic_match` is symmetric.
- `MatchConfig::default()` round-trips through JSON without value drift.
- `Worker` round-trips through JSON.
- The DOB sub-score is order-independent.
- `Confidence::from_score` is monotonic (higher score never yields a lower-ranked band).

`proptest` persists shrunk failure seeds in `tests/property_tests.proptest-regressions`. That file is checked in so prior failure inputs are re-tried on every run; CI runs the property file as part of `cargo test`.

---

## 19. Build, Tooling, and Release

### 19.1 Toolchain

- Rust edition **2024**.
- Build: `cargo build` (debug) / `cargo build --release`.
- Test: `cargo test` (unit + integration + doctests).
- Lint: `cargo clippy --all-targets -- -D warnings`.
- Format: `cargo fmt`.
- Demo: `cargo run` (executes `src/main.rs`).
- Examples: `cargo run --example basic_usage`, `cargo run --example custom_config`.

### 19.2 Release Procedure

1. Update `Cargo.toml` version per SemVer.
2. Update `CHANGELOG.md` with a new dated section.
3. Update this spec if behaviour or API changed.
4. `cargo test`, `cargo clippy`, `cargo fmt --check`.
5. `cargo publish --dry-run`, then `cargo publish`.
6. Tag the commit `v<version>` and push.

### 19.3 Versioning

- Pre-1.0: minor bumps MAY contain breaking changes (per Cargo convention) — document them prominently.
- Post-1.0: strict SemVer.

---

## 20. Security, Privacy, and Compliance

- **No IO**: the library reads no files, makes no network calls, opens no sockets.
- **No logging of PII**: there is no logging in library code at all.
- **No global state**: no thread-locals, no `static mut`, no lazy_statics carrying worker data.
- **Memory hygiene**: input strings are owned by the caller; the library borrows them. There is no zeroing of memory because the library does not hold PII beyond a single call.
- **GDPR**: the library is a pure function; consumer applications carry GDPR responsibility for the records they pass in.
- **Clinical safety**: per the research basis (§5), no algorithm is perfect. Consumers MUST treat probabilistic matches as recommendations, not decisions.

---

## 21. Roadmap and Future Work

Grouped by likely release.

### 21.1 Near-term (0.2.x)
- Make `MatchConfig` and `SimilarityAlgorithm` serde-serialisable. ✅ Delivered (T-1).
- Add `Confidence` enum to `MatchResult`. ✅ Delivered (T-2).
- Resolve address sub-score arithmetic (see §22 OQ-4). ✅ Delivered (T-3).
- Add criterion benchmarks. ✅ Delivered (T-5).
- Add property-based tests via `proptest`. ✅ Delivered (T-6).

### 21.2 Medium-term (0.3.x)
- Locale-aware phonetic encoder (e.g. Double Metaphone, NYSIIS, or a custom encoder) as a replacement for or augmentation of American Soundex. **Spike outcome (T-9, §21.4):** keep Soundex as default; expose a `MatchConfig::phonetic_encoder` opt-in enum (`Soundex` / `DoubleMetaphone` / `DaitchMokotoff`); defer the default-switch decision until an empirical multinational worker corpus is available. Implementation tracked as T-9.1 in §23.2.

Delivered in 0.3.0: nickname dictionary (T-10), date-transposition heuristic for DOB (T-22), email scoring (T-11), `previous_addresses` best-of scoring (T-24), and middle-name scoring (T-25).

### 21.3 Longer-term (0.4.x – 1.0)
- Optional `match_many_to_many` and blocking-key helpers built on top of the delivered batch API (`match_one_to_many` / `rank_one_to_many` shipped in 0.3.0).
- ~~Optional integration with an external postal-address reference for address standardisation, behind a feature flag.~~ — **Declined per §21.4 (T-14)**: the worker-matcher crate stays IO-free and dependency-light; consumers SHOULD standardise addresses in the ingest pipeline before scoring.
- Optional Fellegi-Sunter weight learning (training mode).
- Async batch evaluation with `rayon` or `tokio`.
- Further national identifier schemes. **42 schemes delivered in 0.3.0**: the original 35 (UK NHS Number, France NIR, España TSI, Éire IHI, UK NI H&C Number, US SSN, Australia IHI, Germany KVNR, Italy *Codice Fiscale*, Netherlands BSN, Sweden *Workernummer*, UK Scotland CHI Number, Belgium NN, Bulgaria EGN, Czech *Rodné číslo*, Denmark CPR, Estonia *Isikukood*, Spain DNI/NIE, Finland HETU, Croatia OIB, Iceland *Kennitala*, Lithuania *Asmens kodas*, Latvia *Workeras kods*, Malta National ID, Norway *Fødselsnummer*, Poland PESEL, Romania CNP, Slovenia EMŠO, Slovakia *Rodné číslo*, UK NINO, Greece DSS, Liechtenstein National ID, Netherlands National ID, Poland NIP, Portugal NIF) plus the T-17.1 batch: Brazil CPF, China Resident Identity Card, India Aadhaar, Japan My Number, Mexico CURP, New Zealand NHI, South Africa ID. The T-17.1 batch closes the gap between identifier-parsing coverage and phone-table coverage. Further jurisdictions (HK, SG, KR, TR, RU, AR, CA-provincial) can be added incrementally per consumer demand.
- ~~Expanded phone-number country coverage and richer per-country validation (mobile vs landline prefixes, area-code structure).~~ — partially superseded by T-19 (§21.4): the table now covers all 39 identifier-scheme jurisdictions; per-country mobile/landline validation was **declined** as it does not help matching recall. Initial international E.164 phone-number support across ~25 countries — including all six original national-identifier jurisdictions (UK, FR, ES, IE, UK NI via GB, US) — was delivered in 0.3.0; the additional 13 jurisdictions were added in T-19 (BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK).
- 1.0 stabilisation: ratify API surface and freeze.

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

---

## 22. Open Questions and Risks

Open questions are tracked here until resolved; resolutions move into the relevant numbered section.

- **OQ-1 — Resolved (0.3.0).** `middle_name` participates in the given-name component score with weight `0.05` (blended as `0.95 × given + 0.05 × middle`) when both workers have a middle name (FR-49, §12.2). See task T-25.
- **OQ-2 — Resolved (0.3.0).** `email` is scored after normalisation (FR-35/FR-36, §14.3a); `local_id` is NOT scored because different organisations may issue colliding values. See task T-11.
- **OQ-3 — Resolved (0.3.0).** `Worker` and `Address` carry `#[non_exhaustive]` (FR-53; task T-8).
- **OQ-4 — Resolved (T-3).** The address sub-score now uses the weighted-average form `Σ(score × weight) / Σ(weight)` over the sub-components that fired (postcode = `0.5`, city = `0.3`, line 1 = `0.2`); option (b) of the original framing. Postcode dominates as documented and an exact postcode + slightly different street clears `0.7`. See §12.4.
- **OQ-5 — Resolved (0.3.0).** Under `strict_mode = true`, `is_match` requires both `score >= match_threshold` AND `deterministic_match` (FR-47, §13.2). See task T-4.
- **OQ-6 — Resolved (T-13).** Unused `MatchingError` variants were removed in T-13. The 35 national-identifier parsers all return `Option<String>` (the parser is the source of truth on validity), `MatchConfig` builders are infallible, and the crate does not parse date strings — so `InvalidData`, `InvalidNhsNumber`, `InvalidDate`, and `ConfigError` had no code path returning them. Only `MissingField` is retained. `MatchingError` is now `#[non_exhaustive]` so future fallible code paths can add variants without breaking SemVer.
- **OQ-7** Should the phonetic bonus participate in `total_weight` only when applied (current behaviour) or always (skews the average down when phonetic is weak)? *Current behaviour is correct;* document explicitly.

### 22.1 Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Misuse as a clinical decision oracle | Medium | High | Documentation; require explainable `MatchBreakdown` on every call. |
| Diacritic-heavy name false negatives | Medium | Medium | NFKD pipeline today; locale-aware phonetic encoder tracked under §21.2 and §23 T-9. |
| Drift between this spec and the code | High | Medium | Treat this file as part of every PR; CI check planned (§23 T-7). |
| Soundex collisions cluster too aggressively | Medium | Low | Phonetic only contributes as a bonus, not a primary signal. |
| Dependency `nhs-number` becomes unmaintained | Low | Medium | Pin minor version; have a vendored fallback path documented. |
| Cross-scheme identifier confusion (e.g. recording an H&C Number in `uk_nhs_number` because both are 10 digits) | Medium | High | Distinct `Worker` fields per scheme; deterministic matching strictly scheme-local; spec §12.1 and FR-13 forbid cross-scheme equality. Consumer applications must record provenance at ingest. |
| España TSI lenient validation admits malformed regional values | Medium | Low | Lenient parse is deliberate (Spanish regional schemes vary); consumers needing stronger validation may layer a community-specific check on top of `parse_es_tsi`. |

---

## 23. Tasks and Acceptance Criteria

This section is the single source of truth for outstanding and completed work. It absorbs what an SDD workflow would otherwise place in a separate `tasks.md` document, with each task pointing back at the requirement, design, or open question it serves. Tasks are tagged `T-NN`. Status legend: `[ ]` open, `[~]` in progress, `[x]` done.

### 23.1 Done (carried over from CHANGELOG)
- [x] Initial `Worker` model with builder.
- [x] `MatchingEngine` with configurable weights and thresholds.
- [x] Deterministic matching on national identifiers / demographics.
- [x] Probabilistic matching with weighted average.
- [x] Jaro-Winkler, Levenshtein, Exact, Combined string similarity.
- [x] Soundex-based phonetic matching.
- [x] Name, postcode, phone, identifier normalisation.
- [x] Address comparison (postcode + city + line 1).
- [x] Diacritic handling via NFKD decomposition.
- [x] Three pre-defined configs: default / strict / lenient.
- [x] Unit tests and integration tests per §18.
- [x] `serde` support for `Worker`, `Address`, `Gender`, `MatchResult`, `MatchBreakdown`.
- [x] Multinational national-identifier support (T-16): UK NHS Number, France NIR, España TSI, Éire IHI, UK NI H&C Number. Country-prefixed naming (`<cc>_<scheme>`) applied across `Worker` fields, `WorkerBuilder` setters, `MatchConfig` weights, and `MatchBreakdown` scores. `identifiers` module exposes one parser per scheme.
- [x] United States Social Security Number (T-21): `identifiers::parse_us_ssn` with full structural validation (`000` / `666` / `900..=999` area, `00` group, `0000` serial); `us_ssn` field on `Worker` and `WorkerBuilder`; `us_ssn_weight` on `MatchConfig`; `us_ssn_score` on `MatchBreakdown`; deterministic-match path.
- [x] Sophisticated address parsing (T-20): `Normalizer::expand_street_abbreviations`, `normalize_address_line`, `parse_address_line`, and `ParsedAddressLine`. The matcher's line-1 comparison now uses abbreviation expansion plus a structural house-number sub-component.
- [x] International phone-number support (T-18): `Normalizer::normalize_phone_e164` returns the E.164 canonical form for ~25 supported countries (all six identifier jurisdictions plus the major worker-mobility partners), `MatchConfig::phone_default_country` controls the assumed jurisdiction for national-format inputs, and `MatchingEngine` prefers the E.164 form with a fallback to the legacy national-significant comparison.
- [x] Nickname dictionary (T-10): public `NicknameTable` (empty/english/with_class/are_equivalent) consulted by `score_name`; given/family-name component lifts to `≥ 0.9` when the table considers the pair equivalent; boost never lowers a score.
- [x] Email scoring (T-11): `Normalizer::normalize_email` (trim + lowercase + structural validation, opt-in Gmail dot/+-folding); `MatchConfig::email_weight`, `MatchConfig::gmail_dot_folding`, and `MatchBreakdown::email_score`. `local_id` deliberately not scored (resolves OQ-2).
- [x] `Confidence` enum (T-2): `MatchResult::confidence` populated by `Confidence::from_score(score)`; band boundaries `≥ 0.90 / ≥ 0.75 / else`; independent of `match_threshold`; serde-derived with `#[serde(default)]` for legacy payloads.
- [x] Serialisable config (T-1): `MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable` derive `Serialize + Deserialize`; `MatchConfig` carries `#[serde(default)]` for partial-document config files.
- [x] Date-of-birth transposition heuristic (T-22): probabilistic DOB sub-score returns `0.5` when one side is a day/month transposition of the other (same year, valid swapped date); `deterministic_match` is unchanged. Catches the common DD/MM ↔ MM/DD data-entry bug.
- [x] Batch API (T-15): `MatchingEngine::match_one_to_many(query, candidates)` and `MatchingEngine::rank_one_to_many(query, candidates)`. The engine remains immutable and `Send + Sync`, so consumers can layer parallelism (rayon, tokio) without changes to this crate.
- [x] `strict_mode` enforcement (T-4 / resolves OQ-5): under `strict_mode = true`, `is_match` requires both `score >= match_threshold` AND `deterministic_match(p1, p2)`. Probabilistic score and confidence are unchanged.
- [x] `previous_addresses` best-of scoring (T-24): the address sub-score is the highest score across the cartesian product of `(current ∪ previous_addresses)` on both sides. Catches the "worker moved house" failure mode without dragging down strong current-vs-current matches.
- [x] Middle-name scoring (T-25 / resolves OQ-1): when both sides have a `middle_name`, the given-name component blends `0.95 × given + 0.05 × middle` using the same `name_algorithm` and nickname-table boost as the given-name path.
- [x] Passport books (T-26): public `PassportBook { country, number, issued, expires }` type and `Worker::passport_books: Vec<PassportBook>` field model multi-country dual-citizenship, multi-book historical / current accumulation, and time-varying book numbers. Matching treats any shared `(country, number)` pair as a deterministic match; cross-country values with the same number never cross-match.
- [x] `#[non_exhaustive]` on `Worker` and `Address` (T-8 / resolves OQ-3): formalises that struct-literal construction is reserved for the defining crate. External consumers use `Worker::builder()` or `Address::new()` with the new `with_*` fluent setters. Future field additions can ship as minor releases without breaking downstream code.
- [x] Eighteen additional national workeral identifiers (T-27): Belgium NN, Bulgaria EGN, Czech RČ, Denmark CPR, Estonia *Isikukood*, Spain DNI/NIE, Finland HETU, Croatia OIB, Iceland *Kennitala*, Lithuania *Asmens kodas*, Latvia *Workeras kods*, Malta National ID, Norway *Fødselsnummer*, Poland PESEL, Romania CNP, Slovenia EMŠO, Slovakia RČ, UK NINO. Total schemes supported: 30. Each scheme-local with its own parser, builder setter, weight, breakdown score, and deterministic-match branch.
- [x] Five further workeral identifiers + nine passport-number format validators (T-28) driven by `AGENTS/national-worker-identifiers.tsv`: Greece DSS, Liechtenstein National ID, Netherlands National ID, Poland NIP, Portugal NIF as full `Worker` fields (35 schemes total); Cyprus / Czech / Liechtenstein / Lithuania / Malta / Netherlands / Portugal / Romania / Slovakia passport format validators as standalone parsers feeding `PassportBook`.
- [x] Blood-type scoring (T-29): public `BloodType` enum (8 ABO+RhD variants) with a lenient `parse` accepting canonical, word-form, and zero-to-O variants; `Worker::blood_type: Option<BloodType>` plus `MatchConfig::blood_type_weight` (default 0.05) and `MatchBreakdown::blood_type_score`. Strong negative signal (stable for life) at a low default weight; deliberately excluded from `deterministic_match` and from `Worker::validate`'s identifying-field set.
- [x] Place-of-birth scoring (T-30): `Worker::birth_place: Option<Address>` reusing the existing Address type for FHIR `Patient.birthPlace` parity. Dedicated city + country sub-score (`0.7 × Jaro-Winkler(city) + 0.3 × exact(country)` blend); `MatchConfig::birth_place_weight` (default 0.05); `MatchBreakdown::birth_place_score`. Diacritic-tolerant via the shared name-normalisation pipeline.
- [x] Multiple-birth scoring (T-31): `Worker::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed birth order); `MatchConfig::multiple_birth_weight` (default 0.05); `MatchBreakdown::multiple_birth_score`. Primary use: disambiguating identical twins who otherwise share name, DOB, and demographic data. Not part of `deterministic_match` or `validate`'s identifying-field set.
- [x] Spec/code drift CI check (T-7): first CI workflow in the repo. `.github/workflows/spec-drift.yml` invokes `scripts/spec-drift-check.sh` on every pull request to `main`. The check fails if `src/matcher.rs` changes without `spec.md` changing in the same PR, modulo path-pattern exceptions in `.spec-allow`. PR template at `.github/pull_request_template.md` references the check. POSIX bash, runs locally as well as in CI.
- [x] Seven next-batch national identifier schemes (T-17.1): `parse_br_cpf` (BR CPF, 11 digits, two Mod-11 check digits), `parse_cn_rrn` (CN Resident Identity Card, 18 chars, weighted Mod-11 + date substring), `parse_in_aadhaar` (IN Aadhaar, 12 digits, Verhoeff), `parse_jp_my_number` (JP My Number, 12 digits, weighted Mod-11), `parse_mx_curp` (MX CURP, 18 alphanumeric chars, structural + Mod-10 weighted), `parse_nz_nhi` (NZ NHI original 7-char form, 3 letters + 4 digits, Mod-11 weighted with letter-to-int lookup excluding I/O), `parse_za_id` (ZA ID, 13 digits, Luhn + date substring). Each scheme is scheme-local (no cross-matching) and gets `Worker` field, builder setter, `MatchConfig` weight (0.30), `MatchBreakdown` score, `deterministic_match` branch, and `Worker::validate` inclusion. Sentinel-data rejection per §21.4 (BR CPF all-equal sequences, IN Aadhaar `0`/`1` prefixes). Total scheme count: 35 → **42**.
- [x] More-national-identifiers spike (T-17): the original T-17 candidate list (CHI, KVNR, Codice Fiscale, BSN, PESEL, Workernummer, IHI) all shipped under T-23 / T-27 / T-28; total coverage now 35 schemes. The follow-up §21.4 recommendation identifies the **7 phone-table-covered jurisdictions without an identifier parser** (BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID) as the next batch — each with a per-scheme parser sketch and check-digit algorithm. Implementation tracked as T-17.1.
- [x] Locale-aware phonetic encoder spike (T-9): surveyed Soundex, Double Metaphone, NYSIIS, Daitch-Mokotoff, Beider-Morse, locale-specific encoders, and a custom encoder. Recommendation in §21.4: keep Soundex as the default (no breaking change), expose an opt-in `MatchConfig::phonetic_encoder` enum via the `phonetic-rphonetic` Cargo feature flag, defer the default-switch decision until an empirical multinational worker corpus is available. The asymmetric `0.05`-weighted bonus design (FR-22/FR-23) caps the worst-case false-positive risk of any opt-in encoder. Implementation follow-up tracked as T-9.1.
- [x] Broader phone country table (T-19): expanded `COUNTRY_PHONE_TABLE` from 26 to 39 jurisdictions, covering every country the crate parses a national identifier for (added BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK). Refactored `has_trunk_prefix: bool` → `trunk_prefix: Option<&'static str>` to support Lithuania's `8` trunk prefix. 6 new e164 unit tests. Declined: full ~250-territory ITU-T expansion, `phonenumber` crate dependency, and per-country mobile/landline prefix validation. Recommendation matrix in §21.4.
- [x] Address-parser-exploration research spike (T-14): surveyed libpostal, Rust-native parsers, national reference datasets, and commercial APIs; recommendation recorded in §21.4 is to **decline** external standardisation at this layer — adding it would violate the IO-free, pure-Rust, multinational axioms (§17, §20) for a fractional contribution (line 1 is only 0.2 of the 0.05-weighted address sub-score). Consumers SHOULD standardise upstream in their ingest pipeline. Two additive follow-ups identified (locale-aware street vocab; optional UPRN-style property identifier) but neither is in scope.
- [x] Documentation harmonisation (T-12): every top-level doc (`README.md`, `AGENTS.md`, `spec.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `IMPLEMENTATION_SUMMARY.md`) now points to `index.md` as the entry point. The previously-orphaned `AGENTS/national-worker-identifiers.md` reference table is linked from `AGENTS.md` and `index.md`. `IMPLEMENTATION_SUMMARY.md` carries a "superseded by `spec.md`" banner. All 17 referenced intra-repo doc paths verified to exist.
- [x] Error-model cleanup (T-13 / resolves OQ-6): removed the four `MatchingError` variants that no code path returned in 0.3.0 (`InvalidData`, `InvalidNhsNumber`, `InvalidDate`, `ConfigError`); marked `MatchingError` `#[non_exhaustive]` so future fallible code paths can add variants without breaking SemVer. Only `MissingField` (returned by `Worker::validate`) remains.
- [x] Property tests (T-6): `tests/property_tests.rs` exercises 11 invariants via `proptest` at 1000 cases each — `normalize_name` idempotency and shape, `score ∈ [0.0, 1.0]`, self-match positivity + `Confidence::High`, probabilistic and deterministic symmetry, DOB sub-score symmetry, `Confidence::from_score` monotonicity, and JSON round-trips for `MatchConfig::default()` and arbitrary `Worker` records. Historical failure seeds persisted in `tests/property_tests.proptest-regressions`.
- [x] Criterion benchmarks (T-5): `benches/match_pair.rs` covers `match_pair` (identical / fuzzy / unrelated), `deterministic_match_identifier_hit`, `rank_one_to_many` (`n ∈ {10, 100, 1000}` with throughput reporting), and `config_variants` (default / strict / English nickname table). Confirms the §17 performance budget (`< 50 µs` per pair) on a 2024 Apple Silicon machine: single-pair fuzzy match ~4 µs, deterministic ID hit ~160 ns, batch ranking ~3 µs/element.
- [x] Address sub-score weighted-average arithmetic (T-3 / resolves OQ-4): `compare_addresses` now accumulates `weighted_sum` and `total_weight` independently and returns `weighted_sum / total_weight`, so postcode (weight `0.5`) dominates city (`0.3`) and line 1 (`0.2`) as the spec already documented. Exact postcode + slightly different street clears `0.7`. Neutral `0.5` fallback when no sub-component fires is preserved.
- [x] Date-of-death and place-of-death scoring (T-32): `Worker::death_date: Option<NaiveDate>` (FHIR `Patient.deceasedDateTime`) reuses the DOB transposition heuristic via the shared `score_dob_pair` helper; `Worker::death_place: Option<Address>` parallels `birth_place` and reuses the new `score_named_place` free helper (extracted from the prior `score_birth_place` body). `MatchConfig::death_date_weight` defaults to `0.10`; `MatchConfig::death_place_weight` defaults to `0.05`. Both `MatchBreakdown` scores are independent from `date_of_birth_score` / `birth_place_score`. Neither field contributes to `deterministic_match` or to `validate`'s identifying-field set.
- [x] Six additional national identifiers (T-23): Australia IHI (`parse_au_ihi`), Germany KVNR (`parse_de_kvnr`), Italy *Codice Fiscale* (`parse_it_cf`), Netherlands BSN (`parse_nl_bsn`), Sweden *Workernummer* (`parse_se_workernummer`), and UK Scotland CHI Number (`parse_uk_chi_number`). Each with its own check-digit algorithm, `Worker` field, builder setter, `MatchConfig` weight (default 0.30), and independent `MatchBreakdown` score. Total schemes supported: 12.

### 23.2 Open tasks

**T-1 — Serialisable config.** ✅ Delivered.
- [x] Derive `Serialize, Deserialize` for `MatchConfig`, `SimilarityAlgorithm`, and `NicknameTable`.
- [x] `MatchConfig` carries `#[serde(default)]` so partial JSON documents merge over `MatchConfig::default()`.
- **Acceptance:** `MatchConfig::default()`, `strict()`, and `lenient()` round-trip through `serde_json` with all values preserved; partial JSON inherits defaults; end-to-end test confirms an engine built from a deserialised config matches the original byte-for-byte. Met by `tests/integration_tests.rs` §18 and `src/matcher.rs::tests`.

**T-2 — `Confidence` in `MatchResult`.** ✅ Delivered.
- [x] Add public `Confidence` enum (`High`/`Medium`/`Low`) in `src/matcher.rs`, re-exported from the crate root.
- [x] Add `pub confidence: Confidence` to `MatchResult` populated by `Confidence::from_score(score)` on every `match_workers` call.
- [x] Boundaries: `≥ 0.90 → High`, `≥ 0.75 → Medium`, else `Low` (inclusive on the low side).
- [x] Confidence is **independent of `match_threshold`** — a score of `0.92` is `High` under strict, default, and lenient presets alike. `is_match` remains the authoritative go/no-go signal.
- [x] `confidence` is `#[serde(default = "default_confidence")]` so legacy JSON payloads lacking the field deserialise to `Low` (interpretable as "needs re-scoring").
- **Acceptance:** Unit tests pin band boundaries, threshold independence, and serde round-trip. Integration tests pin: High for exact clones, Low for completely different workers, threshold-independence under strict vs lenient, and legacy JSON deserialisation. Met by `tests/integration_tests.rs` §17 and `src/matcher.rs::tests`.

**T-3 — Address sub-score correction.** ✅ Delivered.
- [x] Resolve §22 OQ-4. Implemented option (b): the address sub-score now accumulates `weighted_sum` and `total_weight` independently and divides at the end (`Σ(score × weight) / Σ(weight)`), so postcode (`0.5`) dominates as documented.
- [x] Neutral fallback (`0.5`) preserved when no sub-component fires.
- **Acceptance:** 4 unit tests + 2 integration tests in §26 pin the behaviour: exact postcode + slight street typo clears `0.7`, postcode-only match collapses to `1.0`, postcode-match + line1-mismatch is dominated by postcode, and the empty-address neutral fallback still returns `0.5`.

**T-31 — Multiple-birth scoring.** ✅ Delivered.
- [x] Add `Worker::multiple_birth: Option<u8>` (FHIR `Patient.multipleBirth`, 1-indexed birth order) with `#[serde(default)]`.
- [x] Add `WorkerBuilder::multiple_birth(value)` setter.
- [x] Add `MatchConfig::multiple_birth_weight` (default 0.05) and `MatchBreakdown::multiple_birth_score` (with `#[serde(default)]`).
- [x] `score_multiple_birth` helper: `Some(1.0)` for equal values, `Some(0.0)` for different, `None` when either side is missing.
- [x] Not part of `deterministic_match` (too weak alone) and not part of `Worker::validate`'s identifying-field set.
- **Acceptance:** 6 integration tests pin: match, identical-twin disambiguation (the canonical clinical failure mode), missing-on-one-side `None`, not-part-of-deterministic invariant, serde round-trip carrying the field, legacy-payload deserialisation to `None`.

**T-32 — Date-of-death and place-of-death scoring.** ✅ Delivered.
- [x] Add `Worker::death_date: Option<NaiveDate>` and `Worker::death_place: Option<Address>` (both with `#[serde(default)]`).
- [x] Add `WorkerBuilder::death_date(value)` and `WorkerBuilder::death_place(value)` setters.
- [x] Add `MatchConfig::death_date_weight` (default 0.10) and `MatchConfig::death_place_weight` (default 0.05).
- [x] Add `MatchBreakdown::death_date_score` and `death_place_score` (both with `#[serde(default)]`).
- [x] Extract a free `score_named_place(&Address, &Address) -> Option<f64>` helper from the prior `score_birth_place` body; refactor `score_birth_place` to delegate to it; introduce `score_death_place` that delegates likewise. Death-place data goes through the same `0.7 × city + 0.3 × country` blend as birth-place data.
- [x] `score_death_date` delegates to the existing free `score_dob_pair` helper, so DD/MM ↔ MM/DD transpositions on death dates are also recognised as half-credit.
- [x] Neither field contributes to `deterministic_match` (weak alone) nor to `Worker::validate`'s identifying-field set.
- **Acceptance:** 14 integration tests in `tests/integration_tests.rs` §25 plus 8 unit tests in `src/matcher.rs::tests` pin: exact match, day/month transposition, unrelated dates, missing-on-one-side `None`, independence from `date_of_birth_score`, place exact / different / city-only-partial / missing / independence from birth_place / not-part-of-deterministic, serde round-trip, legacy-payload deserialisation to `None`, composite-score non-regression, and free-helper edge cases.

**T-30 — Place-of-birth scoring.** ✅ Delivered.
- [x] Add `Worker::birth_place: Option<Address>` (with `#[serde(default)]`) reusing the existing `Address` type for FHIR `Patient.birthPlace` parity.
- [x] Add `WorkerBuilder::birth_place(value)` setter.
- [x] Add `MatchConfig::birth_place_weight` (default 0.05) and `MatchBreakdown::birth_place_score` (with `#[serde(default)]`).
- [x] Dedicated `score_birth_place` helper that considers only `city` (Jaro-Winkler) and `country` (exact), blended `0.7 × city + 0.3 × country` when both present; single signal when only one; `None` when no comparable subset.
- [x] **Not** part of `deterministic_match` (too weak alone) and **not** part of `Worker::validate`'s identifying-field set.
- **Acceptance:** 10 integration tests pin: identical-birth-place scores ~1.0; wildly-different scores low; same-city / different-country = 0.7; missing-on-one-side = None; country-only fallback; empty subfields = None; not-deterministic invariant; diacritic-tolerant city; serde round-trip with Worker; legacy-payload deserialisation.

**T-29 — Blood-type scoring.** ✅ Delivered.
- [x] Add public `BloodType` enum (8 ABO+RhD variants) in `src/models.rs` with serde-rename to canonical short forms (`"A+"`, …).
- [x] Add `BloodType::parse(s)` accepting canonical, lowercase, word, `+VE`/`-VE`, separator, and zero-to-O variants.
- [x] Add `Worker::blood_type: Option<BloodType>` with a builder setter and `#[serde(default)]` (legacy JSON deserialises with `None`).
- [x] Add `MatchConfig::blood_type_weight` (default 0.05) and `MatchBreakdown::blood_type_score` (`Some(1.0)`/`Some(0.0)`/`None`).
- [x] Blood type is deliberately **not** consulted by `deterministic_match` and **not** an identifying field for `Worker::validate`.
- **Acceptance:** 11 unit tests pin canonical / lowercase / word / `+VE` / separator / zero-O variants plus serde round-trip and builder behaviour. 7 integration tests pin match, mismatch, missing, not-part-of-deterministic, parse-through-builder, serde round-trip, legacy-payload deserialisation. Met by `src/models.rs::tests` and `tests/integration_tests.rs` §22.

**T-28 — Five further workeral IDs + nine passport-format validators.** ✅ Delivered.
- [x] Drive from `AGENTS/national-worker-identifiers.tsv`.
- [x] Add five Worker-field identifiers: `gr_dss` (Greece DSS, format-only 10 digits), `li_id` (Liechtenstein National ID, 2 letters + 8–9 digits, format-only with renewal caveat), `nl_id` (Netherlands National ID, 9-char `[A-Z\O]{2}[A-Z0-9\O]{6}[0-9]`), `pl_nip` (Poland NIP, 10 digits weighted Mod-11), `pt_nif` (Portugal NIF, 9 digits weighted Mod-11). Total Worker-field schemes: **35**.
- [x] Add nine passport-format validators in the `identifiers` module (Cyprus, Czech, Liechtenstein, Lithuania, Malta, Netherlands, Portugal, Romania, Slovakia). These are pure format validators with no Worker field; passport data is canonically stored via `Worker::passport_books: Vec<PassportBook>`.
- **Acceptance:** ≥3 unit tests per parser pinning canonical / variant / wrong-shape cases (43 new identifier tests); per-scheme integration tests cover deterministic match, mismatch, validate-accepts-solo, plus scheme-locality (NL ID ≠ NL BSN; PL NIP ≠ PL PESEL); composition test demonstrates `parse_<cc>_passport` feeding `PassportBook::new`. Met by `src/identifiers.rs::tests` and `tests/integration_tests.rs` §21b / §21c.

**T-27 — Eighteen additional national workeral identifiers.** ✅ Delivered.
- [x] Add 18 new parsers to `src/identifiers.rs`: `parse_be_nn` (Belgium Mod-97), `parse_bg_egn` (Bulgaria weighted Mod-11), `parse_cz_rc` (Czech Mod-11 divisibility), `parse_dk_cpr` (Denmark format-only), `parse_ee_ik` (Estonia cascading Mod-11), `parse_es_dni` (Spain DNI/NIE Mod-23 letter), `parse_fi_hetu` (Finland Mod-31 letter), `parse_hr_oib` (Croatia ISO 7064 MOD 11,10), `parse_is_kt` (Iceland Mod-11), `parse_lt_ak` (Lithuania cascading Mod-11), `parse_lv_pk` (Latvia weighted Mod-11), `parse_mt_id` (Malta format + letter), `parse_no_fnr` (Norway dual Mod-11), `parse_pl_pesel` (Poland weighted Mod-10), `parse_ro_cnp` (Romania Mod-11), `parse_si_emso` (Slovenia Mod-11), `parse_sk_rc` (Slovakia Mod-11), `parse_uk_nino` (UK format with prefix blacklist).
- [x] Extend `Worker`, `WorkerBuilder`, `MatchConfig` (per-scheme weight 0.30), `MatchBreakdown` (per-scheme `Option<f64>` with `#[serde(default)]`), `MatchingEngine` deterministic and breakdown paths, and `Worker::validate` to cover the 18 new schemes. Total: 30 schemes.
- **Acceptance:** ≥4 unit tests per parser pinning canonical / wrong-check / wrong-length / format-variant cases; integration tests pin deterministic match per scheme and verify three UK Mod-11 schemes (NHS / NI H&C / Scotland CHI) remain scheme-local plus NINO never cross-matches. Met by `src/identifiers.rs::tests` and `tests/integration_tests.rs` §21a.

**T-26 — Passport books (multi-country, multi-book, time-varying).** ✅ Delivered.
- [x] Public `PassportBook { country, number, issued, expires }` type in `src/models.rs`; constructor canonicalises country (uppercased 2-letter ASCII) and number (whitespace stripped, uppercased); date fields are metadata only.
- [x] `Worker::passport_books: Vec<PassportBook>` with `add_passport_book` and `passport_books` builder methods. `Worker::validate` accepts a non-empty `passport_books` as a sufficient identifying field.
- [x] `MatchConfig::passport_book_weight` (default `0.30`); `MatchBreakdown::passport_book_score: Option<f64>` with `#[serde(default)]`.
- [x] `MatchingEngine` deterministic path: `true` when any `(country, number)` pair is shared across the two workers' lists. Cross-country values with the same `number` never cross-match.
- **Acceptance:** Unit tests in `src/models.rs::tests` pin constructor canonicalisation, invalid input rejection (bad country / empty number), date setters, serde round-trip (including legacy payloads without date fields). Integration tests in `tests/integration_tests.rs` §21 pin: single-pair deterministic match (with mixed case + whitespace inputs); multi-country any-pair match; same digits different country never match; historical-book pair still matches; one-side-empty → `None`; both-non-empty disjoint → `0.0`; dates are metadata; serde round-trip; legacy Worker JSON deserialises with empty `passport_books`.

**T-25 — Middle-name scoring.** ✅ Delivered.
- [x] Extend `score_given_name` to blend `0.95 × given_sim + 0.05 × middle_sim` when both workers carry a `middle_name`.
- [x] Reuse the existing `score_name` helper so the configured similarity algorithm and nickname-table boost apply to middle names.
- [x] One-sided middle-name data MUST leave the score unchanged (no penalty for asymmetric metadata).
- **Acceptance:** Integration tests pin (a) matching given + matching middle ≈ 1.0; (b) matching given + different middle drops modestly (≥ 0.93, < 1.0); (c) one-sided middle name leaves the score unchanged; (d) matching middle names lift the score relative to a no-middle comparison when given names are close but not equal.

**T-24 — `previous_addresses` best-of scoring.** ✅ Delivered.
- [x] Extend `score_address` to take the highest score across every pair drawn from `(p1.address ∪ p1.previous_addresses) × (p2.address ∪ p2.previous_addresses)`.
- [x] Returns `None` only when at least one side has no address data at all.
- **Acceptance:** Integration tests pin: (a) a matching historical pair lifts the score when currents differ; (b) only-historical-on-both-sides still produces a score; (c) no-data-on-one-side stays `None`; (d) an unrelated historical address does not lower a strong current-vs-current match (relative non-regression). Met by `tests/integration_tests.rs` §4.

**T-4 — `strict_mode` enforcement.** ✅ Delivered.
- [x] Resolve §22 OQ-5: under `strict_mode = true`, set `is_match = (score >= threshold) && deterministic_match(...)`.
- [x] Probabilistic `score` and `confidence` remain unchanged across modes.
- **Acceptance:** Existing strict-mode integration tests continue to pass; new tests verify (a) a fuzzy match clearing a lowered strict threshold but lacking a deterministic anchor is rejected; (b) a deterministic match clearing the strict threshold is accepted; (c) the non-strict default still accepts fuzzy matches above the default threshold. Met by `src/matcher.rs::tests` and `tests/integration_tests.rs` §7.

**T-5 — Benchmarks.** ✅ Delivered.
- [x] Add `benches/match_pair.rs` using `criterion` (HTML reports enabled via the `html_reports` feature).
- [x] Four bench groups cover the hot paths: `match_pair` (identical / fuzzy / unrelated), `deterministic_match_identifier_hit`, `rank_one_to_many` (`n ∈ {10, 100, 1000}`, with criterion throughput reporting per candidate), and `config_variants` (default vs strict vs nickname-table-loaded).
- **Acceptance:** `cargo bench` compiles and runs end-to-end. Indicative single-machine numbers (2024 Apple Silicon, `--quick`): `match_pair / fuzzy_near_match ≈ 4 µs`, `deterministic_match_identifier_hit ≈ 160 ns`, `rank_one_to_many @ n=1000 ≈ 3 ms` (~3 µs/element). All well under the §17 budget of `< 50 µs` per pair.

**T-6 — Property tests.** ✅ Delivered.
- [x] Add `proptest` dev-dependency and properties listed in §18.4.
- [x] Eleven properties in `tests/property_tests.rs` covering normalisation idempotency, score bounds, self-match, symmetry (probabilistic and deterministic), confidence monotonicity, and serde round-trips for `Worker` and `MatchConfig`.
- **Acceptance:** `cargo test --test property_tests` runs 1000 cases per property with zero failures. `tests/property_tests.proptest-regressions` is checked in so historical shrunk seeds are re-tried on every run.

**T-7 — Spec/code drift CI check.** ✅ Delivered.
- [x] Workflow: `.github/workflows/spec-drift.yml` runs on every pull request targeting `main`. It fetches full history (`fetch-depth: 0`) so the diff against the base ref is accurate, then invokes `scripts/spec-drift-check.sh` with the GitHub-provided base ref and head SHA.
- [x] Check script: `scripts/spec-drift-check.sh` (POSIX bash, no external dependencies beyond `git`). Resolves the base ref (`origin/<ref>` if available, else local `<ref>`), computes the changed-file set via `git merge-base` + `git diff --name-only`, then enforces: if any file matching the watched pattern (initially `^src/matcher\.rs$`) changed, `spec.md` MUST also have changed. Path patterns in `.spec-allow` (extended regex, blank / `#`-prefixed lines ignored) override the requirement for genuinely spec-irrelevant paths.
- [x] Allowlist: `.spec-allow` ships empty (modulo header comment), so the discipline starts maximally strict; reviewers add patterns as concrete need arises.
- [x] PR template: `.github/pull_request_template.md` references the spec-drift check, lists the spec / allowlist / CHANGELOG checkboxes, and prompts contributors for a test plan.
- [x] Script also runs cleanly from a contributor's machine pre-push (no GitHub-specific assumptions) and exits 0 gracefully when the base ref cannot be resolved (avoids spurious failures in fork CI).
- [x] Verified the script's pass paths against historical commits (`bash scripts/spec-drift-check.sh <older> <newer>` for commits that touched both matcher.rs and spec.md returns `OK`).
- **Acceptance:** Met. CI green on initial introduction because this PR ships `spec.md` updates alongside its source changes. Future PRs that touch `src/matcher.rs` without `spec.md` will fail the `spec-drift` check unless the changed paths match `.spec-allow`. The PR template references the check by name.

**T-8 — Mark `Worker` and `Address` `#[non_exhaustive]`.** ✅ Delivered.
- [x] Add `#[non_exhaustive]` to both struct definitions in `src/models.rs`.
- [x] `Worker::builder()` is the canonical constructor; `Address::new()` + field assignment + new `with_*` fluent setters cover ergonomic external construction.
- [x] Field-assignment syntax on `Address` (`a.line1 = Some(...)`) continues to work because `#[non_exhaustive]` does not block individual field access.
- **Acceptance:** Crate compiles unchanged; tests / examples / doctests all pass (the crate-internal struct-literal use in `Address::new()` is allowed inside the defining crate). External struct-literal construction is now a compile error pointing consumers at the builder. Met by passing test suite at 524 tests after the attribute is added.

**T-9 — Locale-aware phonetic encoder (research spike).** ✅ Delivered (recommendation; implementation deferred to T-9.1).
- [x] Surveyed Soundex (status quo), Double Metaphone, NYSIIS, Daitch-Mokotoff Soundex, Beider-Morse, locale-specific encoders (Kölner Phonetik etc.), and a custom encoder. Decision matrix and rationale in §21.4.
- [x] Sample size and methodology documented: §21.4 specifies the corpus shape (≥ 10k triples per jurisdiction across English-majority + Romance + Germanic + Slavic + Nordic populations) and metrics (TPR at the FR-23 `> 0.9` threshold, FPR, AUC, per-jurisdiction breakdown) a future empirical evaluation should use.
- [x] Recommendation: **stay with Soundex as the default**, add `MatchConfig::phonetic_encoder: PhoneticEncoder` enum with `Soundex` / `DoubleMetaphone` / `DaitchMokotoff` variants behind a `phonetic-rphonetic` Cargo feature flag, defer the default-switch decision until an empirical corpus exists.
- **Acceptance:** Met — written recommendation in §21.4 with sample-size proposal, corpus specification, and evaluation methodology.

**T-9.1 — Phonetic encoder enum (implementation follow-up to T-9).**
- [ ] Add `rphonetic` as an optional dev/build dep behind the `phonetic-rphonetic` Cargo feature flag.
- [ ] Add `PhoneticEncoder` enum (`Soundex` default + `DoubleMetaphone` + `DaitchMokotoff`) and `MatchConfig::phonetic_encoder` field; default value preserves current behaviour exactly.
- [ ] Refactor `Normalizer::phonetic_code(name)` → `Normalizer::phonetic_code(name, encoder)` (additive overload, no-encoder form retained for backward compat).
- [ ] Wire `MatchingEngine::score_phonetic_names` to honour `config.phonetic_encoder`.
- [ ] Define and test the multi-code comparison semantics for Daitch-Mokotoff (FR-22a candidate): non-empty code-set intersection → `1.0`, single-name match → `0.5`, disjoint → `0.0`.
- **Acceptance:** Default-config behaviour and existing tests unchanged. New unit tests cover Double Metaphone primary/secondary equality (`"Stephen"/"Steven"` clean), Daitch-Mokotoff Slavic-cluster equality (`"Schwarz"/"Shvarts"` clean). Documented as "no empirical default-switch claim — opt-in only" until T-9's corpus methodology is run.

**T-10 — Nickname dictionary.** ✅ Delivered.
- [x] Public `NicknameTable` type in `src/nicknames.rs` exposing `empty()`, `english()`, `with_class()`, `are_equivalent()`, `is_empty()`, `len()`.
- [x] `MatchConfig::nickname_table: NicknameTable` defaults to `NicknameTable::empty()`; the feature is opt-in.
- [x] `MatchingEngine`'s `score_name` lifts the per-name component score to `max(score, 0.9)` when the table considers the pair equivalent. The boost never lowers a score.
- [x] Built-in English dictionary covers ≥40 common classes including the acceptance set.
- **Acceptance:** `Mike`↔`Michael`, `Liz`↔`Elizabeth`, `Bob`↔`Robert` lift the given-name score to ≥ 0.9. Met by `tests/integration_tests.rs` §15 and `src/nicknames.rs::tests`.

**T-11 — Email and `local_id` scoring (per OQ-2).** ✅ Delivered.
- [x] Implement `Normalizer::normalize_email(email, gmail_dot_folding) -> Option<String>` with trim + lowercase + structural validation, and opt-in Gmail dot/plus-tag folding for `gmail.com` / `googlemail.com`.
- [x] Add `MatchConfig::email_weight: f64` (default 0.05) and `MatchConfig::gmail_dot_folding: bool` (default false).
- [x] Add `MatchBreakdown::email_score: Option<f64>` populated from the canonical form.
- [x] `local_id` is deliberately not scored (cross-organisation collision risk); document explicitly.
- **Acceptance:** Unit tests cover case/whitespace canonicalisation, Gmail dot-folding (on/off), `+tag` stripping, non-Gmail untouched, malformed input → `None`, idempotence. Integration tests cover exact match, mismatch, missing on one side, unparseable yields `None`, and the dot-folding opt-in toggle. Met by `tests/integration_tests.rs` §16 and `src/normalizer.rs::tests`.

**T-12 — Documentation harmonisation.** ✅ Delivered.
- [x] Every top-level doc file now points to `index.md` as the entry point — verified by `rg index\.md` finding all 7 top-level markdown files (README, AGENTS, spec, CHANGELOG, CONTRIBUTING, CODE_OF_CONDUCT, IMPLEMENTATION_SUMMARY) plus `AGENTS/spec-driven-development.md`.
- [x] The previously-orphaned `AGENTS/national-worker-identifiers.md` (35-scheme reference table) is now linked from both `AGENTS.md` and `index.md`.
- [x] `IMPLEMENTATION_SUMMARY.md` carries an explicit "superseded by `spec.md`" banner so readers do not mistake the historical snapshot for current behaviour.
- **Acceptance:** All intra-repo links resolve (manual sweep against the 17 referenced files). Every top-level doc + the spec-driven-development guide points to `index.md`. No orphaned guides under `AGENTS/`.

**T-13 — Remove or wire-up unused error variants.** ✅ Delivered.
- [x] Resolved §22 OQ-6. Removed `InvalidData`, `InvalidNhsNumber`, `InvalidDate`, and `ConfigError`: none was returned from any code path in 0.3.0 — identifier parsers return `Option<String>` instead of `Result`, `MatchConfig` builders are infallible, and the crate does not parse date strings.
- [x] Marked `MatchingError` as `#[non_exhaustive]` so future fallible code paths can introduce variants without breaking SemVer for downstream pattern-matches.
- **Acceptance:** `MissingField` is the sole surviving variant and is exercised by `Worker::validate`'s test in `src/models.rs::tests` and by the `missing_field_display` test in `src/error.rs::tests`.

**T-14 — Address parser exploration (research spike).** ✅ Delivered (recommendation: **decline**).
- [x] Surveyed libpostal (C dep + multi-GB runtime model; ruled out by §17 portability axiom), Rust-native parsers (no incremental value over the existing `parse_address_line` from T-20), national reference datasets (jurisdiction-locked, fail the multinational scope), and commercial APIs (network IO; ruled out by §17 / §20 PII egress constraints).
- [x] Recommendation recorded in §21.4. Verdict: do **not** integrate an external postal-address reference at the worker-matcher layer; consumers should standardise upstream in their ingest pipeline. The line-1 sub-component is only 0.2 of the 0.05-weighted address sub-score — the cost-benefit doesn't justify a heavyweight dependency.
- [x] Two incremental in-house improvements identified as potential follow-ups if recall data later demands it: locale-aware street-type vocabulary (FR `rue`, DE `straße`, IT `via`, ES `calle`, NL `straat`, …) and an optional `uprn`-style property identifier on `Address` scored like a national-identifier scheme. Neither is in scope for T-14; both are additive and unblocked.
- **Acceptance:** Met — written recommendation now lives at §21.4.

**T-15 — Batch API.** ✅ Delivered.
- [x] `MatchingEngine::match_one_to_many(query, candidates) -> Vec<MatchResult>` parallel to the input slice.
- [x] `MatchingEngine::rank_one_to_many(query, candidates) -> Vec<(usize, MatchResult)>` sorted by descending score with deterministic ascending-index tiebreak.
- [x] Blocking is a consumer concern; the crate stays a pure scoring library and the API surface is deliberately minimal.
- **Acceptance:** Unit tests pin empty-candidates, order preservation, individual-equivalence, ranking ordering, tie-break determinism, and call-to-call determinism. Integration tests pin filtered consumption, confidence-band carry-through, and `Arc`-shared threadsafe batch scoring. Met by `tests/integration_tests.rs` §20 and `src/matcher.rs::tests`.

**T-16 — Multinational national-identifier support.** ✅ Delivered.
- [x] Add `identifiers` module with `parse_uk_nhs_number`, `parse_fr_nir`, `parse_es_tsi`, `parse_ie_ihi`, `parse_uk_hc_number`.
- [x] Extend `Worker` with `uk_nhs_number`, `fr_nir`, `es_tsi`, `ie_ihi`, `uk_hc_number` (each `Option<String>`).
- [x] Extend `MatchConfig` with per-scheme weights (all default `0.30`).
- [x] Extend `MatchBreakdown` with per-scheme `Option<f64>` scores.
- [x] `deterministic_match` returns `true` on any same-scheme identifier equality; identifiers across schemes never cross-match.
- [x] Use ISO 3166-1 alpha-2 country-code prefix (`<cc>_<scheme>`) consistently across fields, weights, scores, and parser names.
- **Acceptance:** Each scheme has at least one integration test exercising both deterministic and probabilistic matching, plus rejection of cross-scheme collisions. Met by §12 of `tests/integration_tests.rs`.

**T-17 — Add more national identifier schemes (research spike).** ✅ Delivered (recommendation; implementation tracked as T-17.1).
- [x] The original T-17 candidate list (UK Scotland CHI, DE KVNR, IT Codice Fiscale, NL BSN, PL PESEL, SE Workernummer, AU IHI) **all shipped** under T-23 / T-27 / T-28 / T-30; total identifier-scheme coverage is now 35.
- [x] Survey of the next batch: §21.4 identifies the **7 jurisdictions where the crate already parses phones but not identifiers** (BR CPF, CN RRN, IN Aadhaar, JP My Number, MX CURP, NZ NHI, ZA ID) as the recommended next tranche. Closes the symmetry between the phone surface (39 jurisdictions post-T-19) and the identifier surface (35 schemes).
- [x] Per-scheme parser sketch + check-digit algorithm in §21.4 for each of the 7. Highlights: BR CPF uses two weighted Mod-11 digits, CN RRN combines weighted Mod-11 with date-substring validation, IN Aadhaar uses Verhoeff, MX CURP requires structural validation + date substring + Mod-10 check digit, ZA ID layers Luhn over a date-encoding 13-digit format.
- **Acceptance:** Met — recommendation in §21.4 with per-scheme parser sketch and check-digit specification.

**T-17.1 — Next-batch national identifiers (implementation follow-up to T-17).** ✅ Delivered.
- [x] Added `parse_br_cpf`, `parse_cn_rrn`, `parse_in_aadhaar`, `parse_jp_my_number`, `parse_mx_curp`, `parse_nz_nhi`, `parse_za_id` to `src/identifiers.rs` (FR-85..FR-91). Each includes its check-digit algorithm and sentinel-data rejection per §21.4 guidance.
- [x] Extended `Worker`, `WorkerBuilder`, `MatchConfig` (default weight `0.30`), `MatchBreakdown` (with `#[serde(default)]`), `MatchingEngine` (`deterministic_match` branch + `calculate_breakdown` + `calculate_weighted_score`), and `Worker::validate` for each.
- [x] Updated the 3 external `MatchConfig` struct-literal sites (matcher.rs docstring, examples/custom_config.rs ×2, tests/integration_tests.rs ×1).
- [x] 39 new unit tests + 14 new integration tests pin canonical forms, formatted inputs, check-digit rejection, length / character rejection, scheme-local isolation, breakdown wiring, serde round-trips, and legacy-payload defaulting.
- [x] Demo block added to `examples/basic_usage.rs` showing all 7 parsers canonicalising real inputs.
- [x] Sentinel-data rejection per §21.4: BR CPF all-equal sequences rejected; IN Aadhaar `0xxxxxxxxxxx` and `1xxxxxxxxxxx` UIDAI-reserved prefixes rejected.
- [ ] TSV rows in `AGENTS/national-worker-identifiers.tsv` deferred — the TSV is a manually-curated reference table and is out of scope for the immediate ship.
- **Acceptance:** Met. All seven schemes match deterministically and probabilistically within-scheme, never cross-match, and pass per-scheme integration tests. Total scheme count is now **42** (35 → 42). Test totals: 414 unit (was 374, +40), 233 integration (was 219, +14), 11 property, 176 doc (was 169, +7). Clippy + fmt clean.

**T-18 — International phone-number support.** ✅ Delivered.
- [x] Add `Normalizer::normalize_phone_e164(phone, default_country)` returning `Some("+CCNNN…")` for inputs that parse against the supported country table, else `None`.
- [x] Add `MatchConfig::phone_default_country: Option<String>` defaulting to `Some("GB")`.
- [x] Update `MatchingEngine::score_phone` to prefer the E.164 comparison and fall back to the legacy national-significant form when either input fails to parse.
- [x] Cover all six identifier jurisdictions (UK, FR, ES, IE, plus UK NI via GB dial code, plus US for SSN) and the major worker-mobility partners (CA, DE, IT, NL, BE, PT, CH, AT, SE, NO, DK, FI, PL, AU, NZ, JP, CN, IN, BR, MX, ZA).
- **Acceptance:** §14.3 documents the algorithm and table; integration tests pin the within-country, cross-country, and fallback behaviour. Met by `tests/integration_tests.rs` §13.

**T-21 — United States Social Security Number.** ✅ Delivered.
- [x] Add `identifiers::parse_us_ssn` enforcing the structural rules: exactly 9 ASCII digits, area not in `{000, 666, 900..=999}`, group not `00`, serial not `0000`.
- [x] Add `us_ssn: Option<String>` to `Worker` and a `us_ssn(value)` setter on `WorkerBuilder`.
- [x] Add `us_ssn_weight: f64` (default 0.30) to `MatchConfig`; add `us_ssn_score: Option<f64>` to `MatchBreakdown`.
- [x] Extend `deterministic_match` and `match_workers` to treat `us_ssn` as an independent scheme-local identifier.
- [x] Extend `Worker::validate` to accept a solo `us_ssn`.
- **Acceptance:** Unit tests cover canonical and hyphenated layouts, boundary area numbers (`001`, `665`, `667`, `899`), invalid area / group / serial values, wrong length, letters, and arbitrary punctuation stripping. Integration tests pin deterministic and probabilistic match, mismatch, structurally-invalid-yields-None, and inclusion in `Worker::validate`. Met by `tests/integration_tests.rs` §12 (US SSN block) and `src/identifiers.rs::tests`.

**T-20 — Sophisticated address parsing.** ✅ Delivered.
- [x] Add `Normalizer::expand_street_abbreviations` covering street-type (St/Rd/Ave/Blvd/Ln/Dr/Ct/Pl/Sq/Ter/Hwy/Pkwy/Mt/Cres/Gdns/Gr/Cl/Pk/Plz/Expy/Trl) and directional (N/S/E/W/NE/NW/SE/SW) abbreviations.
- [x] Add `Normalizer::normalize_address_line` (abbreviation expansion + name-normalisation pipeline).
- [x] Add `Normalizer::parse_address_line` returning `ParsedAddressLine { house_number, unit, street }`. Public struct, serde-derived, re-exported from the crate root.
- [x] Update `MatchingEngine::compare_addresses` to combine the abbreviation-aware street similarity with an exact house-number sub-component, preserving the existing `count`-based aggregation.
- **Acceptance:** Unit tests cover abbreviation expansion, directional expansion, house-number extraction (including alphanumeric suffix and non-greedy stop), unit prefix recognition for `Flat`/`Apt`/`Apartment`/`Unit`/`Suite`/`Ste`/`Room`/`Rm`, idempotence, and serde round-trip. Integration tests pin: (a) `"123 High St"` vs `"123 High Street"` matches; (b) `"45 N Park Ave"` vs `"45 North Park Avenue"` matches; (c) `"10 Downing St"` outscores `"20 Downing St"`; (d) unit prefix on one side does not block the structured match. Met by `tests/integration_tests.rs` §14.

**T-23 — Six additional national identifier schemes.** ✅ Delivered.
- [x] Add `parse_au_ihi` (16-digit Luhn-checked Australian Individual Healthcare Identifier).
- [x] Add `parse_de_kvnr` (letter + 9 digits Mod-10 German *Krankenversichertennummer*).
- [x] Add `parse_it_cf` (16-character alphanumeric Mod-26 Italian *Codice Fiscale*).
- [x] Add `parse_nl_bsn` (9-digit 11-test Dutch *Burgerservicenummer*).
- [x] Add `parse_se_workernummer` (10- or 12-digit Luhn Swedish workeral identity number).
- [x] Add `parse_uk_chi_number` (10-digit Mod-11 Scottish Community Health Index Number).
- [x] Extend `Worker`, `WorkerBuilder`, `MatchConfig` (per-scheme weight 0.30), `MatchBreakdown` (per-scheme `Option<f64>` with `#[serde(default)]`), `MatchingEngine` deterministic and breakdown paths, and `Worker::validate`.
- **Acceptance:** 6 × 6 unit tests in `src/identifiers.rs` (canonical / wrong check / wrong length / wrong chars / format variants / empty); per-scheme integration tests covering deterministic match, mismatch, unparseable yields `None`, and breakdown carries each score; cross-scheme: AU IHI ↔ IE IHI scheme-local; UK CHI ↔ UK NHS and UK CHI ↔ UK NI H&C scheme-local. Met by `tests/integration_tests.rs` §12 (extended polyglot block) and `src/identifiers.rs::tests`.

**T-22 — Date-of-birth transposition heuristic.** ✅ Delivered.
- [x] Extend `MatchingEngine`'s DOB component score to return `0.5` when one side is a day/month transposition of the other (same year, swapped form is a valid calendar date).
- [x] Leave `deterministic_match` unchanged — it still requires exact `NaiveDate` equality on the demographic-tuple branch.
- **Acceptance:** Unit tests pin the four outcomes (exact, transposition, same-year unrelated, cross-year). Integration tests pin: classic DD/MM ↔ MM/DD lift; cross-year non-fire; deterministic still rejects; partial credit lifts the overall score relative to a zero DOB; transposition alone is not enough to clear the default 0.85 threshold. Met by `tests/integration_tests.rs` §19 and `src/matcher.rs::tests`.

**T-19 — Broader phone country table.** ✅ Delivered (tactical expansion; declined the heavyweight `phonenumber` dependency and per-country mobile-prefix validation).
- [x] Surveyed: status quo (26 countries), tactical expansion (39 covering every identifier-scheme jurisdiction), full ITU-T (~250 territories), `phonenumber` crate dependency, and mobile/landline prefix validation. Recommendation + decision matrix in §21.4.
- [x] Refactored `CountryPhoneInfo::has_trunk_prefix: bool` → `trunk_prefix: Option<&'static str>` so non-`0` trunk conventions work cleanly (Lithuania uses `8`).
- [x] Added 13 new entries to `COUNTRY_PHONE_TABLE` (BG, CZ, EE, GR, HR, IS, LI, LT, LV, MT, RO, SI, SK), bringing total coverage to 39 jurisdictions — one for every national identifier scheme the crate parses.
- [x] Added 6 new e164 unit tests pinning Lithuania's `8` trunk, Greece's no-trunk form, Romania's `0` trunk, Czech canonical form, Iceland's 7-digit NSN, and Croatia/Slovenia overlapping-3-digit-code disambiguation.
- [x] Declined: per-country mobile/landline prefix validation (doesn't help matching recall — the matcher canonicalises, doesn't classify). Declined: `phonenumber` dependency (marginal recall lift for compile-time / dep-surface cost). Consumers with global worker bases SHOULD standardise upstream (same pattern as T-14).
- **Acceptance:** Met — recommendation in §21.4 with sample size (13-jurisdiction gap), decision matrix (6 options), and concrete code shipped (13 entries + 6 tests + struct refactor).

### 23.3 Acceptance Criteria — Project-level

The project as a whole is considered "1.0-ready" when:

- All §21.1 (near-term) tasks are complete.
- This document and code agree (T-7 enforced).
- `Worker`/`Address` are `#[non_exhaustive]` (T-8).
- Public API has not changed in two consecutive minor releases.
- Test coverage `>= 90%` and `cargo test` runs in `< 5 s`.

---

## 24. Change Control

### 24.1 Authority

- This file is **the** specification. Any change that affects observable behaviour MUST update this file in the same PR as the code change.
- A PR that changes only the spec is acceptable when documenting an existing behaviour or recording a decision.
- Editorial fixes (typos, formatting) may be made without an accompanying code change but SHOULD be batched.
- Section numbering is stable: prefer appending to a section over renumbering.
- The `CHANGELOG.md` records *what changed*; this spec records *what is*.

### 24.2 Spec-Driven Development Workflow

The project follows spec-driven development (SDD). The canonical SDD artefacts — specification, plan, and task breakdown — are consolidated into this single document rather than split across separate files:

| SDD artefact | Where it lives in this document |
|---|---|
| Specification (what to build, why, for whom) | §1 Purpose · §2 Scope · §3 Stakeholders · §6 Functional Requirements · §7 Non-Functional Requirements |
| Plan (how to build it: design, architecture, contracts) | §8 Domain Model · §9 Architecture · §10 Component Specifications · §11 Public API · §12 Algorithms · §13 Configuration · §14 Normalization · §15 Error Model · §16 Serialization · §17 Quality Attributes · §18 Testing · §19 Build & Release · §20 Security |
| Forward look (roadmap, open questions, risks) | §21 Roadmap · §22 Open Questions and Risks |
| Tasks (work breakdown, acceptance criteria, status) | §23 Tasks and Acceptance Criteria |
| Provenance (what changed when) | `CHANGELOG.md` (separate file by convention) |

There is no separate `plan.md` and no separate `tasks.md`. If a contributor expects those files, point them here.

### 24.3 Lifecycle of a Change

1. **Identify** the section(s) of this spec the change affects. If the spec is silent on the topic, draft an addition first.
2. **Update the spec** with the new normative text (using RFC 2119 MUST/SHOULD/MAY where appropriate).
3. **Update or add tests** that pin the new behaviour.
4. **Implement** the change in `src/`.
5. **Record** the change in `CHANGELOG.md` under "Unreleased".
6. **Open a PR** that references the affected spec section(s) in its description.

### 24.4 Resolving Disagreements

- If the spec disagrees with the code, the spec wins. File a task under §23 to bring the code into line. Do not silently rewrite the spec to match broken code.
- If two sections of the spec disagree, the more specific section wins; file an editorial fix.
- If a contributor disagrees with the spec on a design point, propose a change to §22 (Open Questions) rather than acting unilaterally.

---

## 25. References

1. Grannis SJ, Overhage JM, Hui S, McDonald CJ. *Worker matcher within a Health Information Exchange.* AMIA Annu Symp Proc, 2014. Local copy: `help/worker-matcher-within-a-health-information-exchange.pdf`.
2. Reisman M. *Patient Identification Techniques — Approaches, Implications, and Findings.* NCVHS, 2020. Local copy: `help/healthit-worker-matcher-aggregation-and-linking-2019-08-16.pdf`.
3. Winkler WE. *String Comparator Metrics and Enhanced Decision Rules in the Fellegi-Sunter Model of Record Linkage.* US Census Bureau, 1990.
4. `nhs-number` crate documentation, including the Modulus-11 check-digit algorithm: https://docs.rs/nhs-number
5. Unicode® Technical Report #15: *Unicode Normalization Forms.*
6. `strsim` crate: https://docs.rs/strsim
7. `soundex` crate: https://docs.rs/soundex
8. `unicode-normalization` crate: https://docs.rs/unicode-normalization
