//! Field-level validation for incoming `CarePathway` payloads.
//!
//! The service stores the matcher's `CarePathway` verbatim, so payload
//! validation is the *service's* responsibility — the matcher is a pure
//! scoring library and deliberately performs no validation. These checks
//! enforce the required `name`, clinical code-format constraints on
//! `condition_codes`, structural checks on the globally-unique
//! `identifiers`, and BCP-47 syntax on `in_language` — returning
//! human-readable problem strings that the controller surfaces as a
//! single `422 Unprocessable Entity`.
//!
//! ## Code-format rules
//!
//! The format checks are intentionally structural — they reject obviously
//! malformed input without attempting to verify that a code exists in a
//! published release of its code system (that needs a terminology server,
//! out of scope here). Per coding system:
//!
//! - **ICD-10** (WHO / ICD-10-CM): a letter, two digits, an alphanumeric
//!   third character, then an optional `.` plus 1–4 alphanumerics —
//!   e.g. `I63`, `I63.9`, `C7A`, `S72.001A`.
//! - **ICD-11** (MMS stem codes): 2–7 alphanumerics whose **second
//!   character is always a letter** (the defining trait distinguishing
//!   ICD-11 from ICD-10), excluding the letters `O` and `I` (ICD-11 omits
//!   them to avoid confusion with `0`/`1`), then an optional `.`
//!   extension — e.g. `1A00`, `BA00`, `8B20.0`.
//! - **SNOMED CT**: an SCTID — 6–18 digits whose final digit is a
//!   [Verhoeff] check digit. The check digit is verified exactly.
//! - **Custom**: only required to be non-blank; no format is imposed.
//!
//! ## Identifier rules
//!
//! Identifiers under a **deterministic** scheme are globally unique and
//! drive the matcher's R-0 short-circuit (a shared value pins the score
//! to `1.0`), so a malformed value there is not merely untidy — it can
//! poison matching. These checks reject the obviously-wrong shapes
//! before they reach storage:
//!
//! - **Uuid**: a canonical 8-4-4-4-12 hex UUID (any version/variant).
//! - **Doi**: the registrant prefix `10.` followed by a registrant code
//!   and a `/` suffix — e.g. `10.1000/xyz123`.
//! - **Wikidata / `GuidelineId` / Uri**: only required to be non-blank
//!   (their value spaces are too open to format-check meaningfully here);
//!   a blank deterministic identifier is rejected because the matcher
//!   silently drops it, so storing it is a latent data-quality defect.
//! - **`PathwayCode` / `LocalId` / Custom**: provider-scoped or free-form;
//!   only required to be non-blank.
//!
//! ## Language rules
//!
//! - **`in_language`**: each entry must be a syntactically valid BCP-47
//!   language tag — a 2–3 letter (or 5–8 letter) primary subtag,
//!   optionally followed by `-` subtags of 1–8 alphanumerics — e.g.
//!   `en`, `en-GB`, `zh-Hans`. Existence in the IANA registry is not
//!   checked.
//!
//! [Verhoeff]: https://en.wikipedia.org/wiki/Verhoeff_algorithm

use care_pathway_matcher::{
    CarePathway, CodeSystem, ConditionCode, IdentifierScheme, PathwayIdentifier,
};

/// Collect every validation problem for `pathway`. An empty vector means
/// the payload is valid.
///
/// The controller joins these into one `422` response, so the operator
/// sees all problems at once rather than fixing them one round-trip at a
/// time.
#[must_use]
pub fn problems(pathway: &CarePathway) -> Vec<String> {
    let mut out = Vec::new();
    if pathway.name.trim().is_empty() {
        out.push("name is required".to_string());
    }
    for (i, code) in pathway.condition_codes.iter().enumerate() {
        if let Some(problem) = condition_code_problem(i, code) {
            out.push(problem);
        }
    }
    for (i, id) in pathway.identifiers.iter().enumerate() {
        if let Some(problem) = identifier_problem(i, id) {
            out.push(problem);
        }
    }
    for (i, tag) in pathway.in_language.iter().enumerate() {
        if !is_valid_bcp47(tag.trim()) {
            out.push(format!(
                "in_language[{i}]: {tag:?} is not a valid BCP-47 language tag"
            ));
        }
    }
    out
}

/// Return a problem string for one `identifiers[i]`, or `None` when it is
/// well-formed for its declared scheme.
fn identifier_problem(i: usize, id: &PathwayIdentifier) -> Option<String> {
    let raw = id.value.trim();
    let (label, ok) = match &id.scheme {
        IdentifierScheme::Uuid => ("UUID", is_valid_uuid(raw)),
        IdentifierScheme::Doi => ("DOI", is_valid_doi(raw)),
        // Open value spaces: only require non-blank.
        IdentifierScheme::Wikidata
        | IdentifierScheme::GuidelineId
        | IdentifierScheme::Uri
        | IdentifierScheme::PathwayCode
        | IdentifierScheme::LocalId
        | IdentifierScheme::Custom(_) => {
            return raw
                .is_empty()
                .then(|| format!("identifiers[{i}]: value must not be blank"));
        }
    };
    (!ok).then(|| format!("identifiers[{i}]: {:?} is not a valid {label}", id.value))
}

/// Return a problem string for one `condition_codes[i]`, or `None` when it
/// is well-formed for its declared system.
fn condition_code_problem(i: usize, cc: &ConditionCode) -> Option<String> {
    let raw = cc.code.trim();
    let (label, ok) = match &cc.system {
        CodeSystem::Icd10 => ("ICD-10", is_valid_icd10(raw)),
        CodeSystem::Icd11 => ("ICD-11", is_valid_icd11(raw)),
        CodeSystem::Snomed => ("SNOMED CT", is_valid_snomed(raw)),
        CodeSystem::Custom(system) => {
            return raw
                .is_empty()
                .then(|| format!("condition_codes[{i}]: {system} code must not be blank"));
        }
    };
    (!ok).then(|| {
        format!(
            "condition_codes[{i}]: {:?} is not a valid {label} code",
            cc.code
        )
    })
}

/// ICD-10: `[A-Z] [0-9] [0-9A-Z]` then optional `. [0-9A-Z]{1,4}`.
#[must_use]
fn is_valid_icd10(code: &str) -> bool {
    let (head, tail) = split_extension(code);
    let h = head.as_bytes();
    if h.len() != 3 {
        return false;
    }
    if !h[0].is_ascii_uppercase() || !h[1].is_ascii_digit() || !is_alnum_upper(h[2]) {
        return false;
    }
    match tail {
        None => true,
        Some(t) => {
            let tb = t.as_bytes();
            (1..=4).contains(&tb.len()) && tb.iter().all(|&b| is_alnum_upper(b))
        }
    }
}

/// ICD-11 MMS stem code: 2–7 alphanumerics, second char a letter, no
/// `O`/`I`, then an optional `.` extension of the same character class.
#[must_use]
fn is_valid_icd11(code: &str) -> bool {
    let (head, tail) = split_extension(code);
    let h = head.as_bytes();
    if !(2..=7).contains(&h.len()) {
        return false;
    }
    // The defining ICD-11 trait: the second character is always a letter.
    if !h[1].is_ascii_uppercase() {
        return false;
    }
    if !h.iter().all(|&b| is_icd11_char(b)) {
        return false;
    }
    match tail {
        None => true,
        Some(t) => !t.is_empty() && t.bytes().all(is_icd11_char),
    }
}

/// SNOMED CT SCTID: 6–18 digits with a trailing Verhoeff check digit.
#[must_use]
fn is_valid_snomed(code: &str) -> bool {
    if !(6..=18).contains(&code.len()) {
        return false;
    }
    if !code.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    verhoeff_valid(code)
}

/// Split a code into its stem and optional post-`.` extension. A code with
/// no `.` yields `(code, None)`; `"I63.9"` yields `("I63", Some("9"))`.
fn split_extension(code: &str) -> (&str, Option<&str>) {
    match code.split_once('.') {
        Some((head, tail)) => (head, Some(tail)),
        None => (code, None),
    }
}

/// ASCII digit or uppercase letter.
fn is_alnum_upper(b: u8) -> bool {
    b.is_ascii_digit() || b.is_ascii_uppercase()
}

/// Allowed ICD-11 character: digit or uppercase letter excluding `O`/`I`.
fn is_icd11_char(b: u8) -> bool {
    b.is_ascii_digit() || (b.is_ascii_uppercase() && b != b'O' && b != b'I')
}

/// Canonical 8-4-4-4-12 hex UUID (case-insensitive, any version/variant).
/// `urn:`/brace wrappers are intentionally not accepted — the stored
/// value should be the bare UUID.
#[must_use]
fn is_valid_uuid(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (i, &c) in b.iter().enumerate() {
        let expect_hyphen = matches!(i, 8 | 13 | 18 | 23);
        if expect_hyphen {
            if c != b'-' {
                return false;
            }
        } else if !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

/// DOI: the registrant prefix `10.`, then a non-empty registrant code
/// (digits / `.` per ISO 26324), a `/`, and a non-empty suffix —
/// e.g. `10.1000/xyz123`. Case is irrelevant. A bare URL form
/// (`https://doi.org/10…`) is intentionally not accepted; store the DOI.
#[must_use]
fn is_valid_doi(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("10.") else {
        return false;
    };
    let Some((registrant, suffix)) = rest.split_once('/') else {
        return false;
    };
    if suffix.is_empty() {
        return false;
    }
    // The registrant code is one or more dot-separated numeric runs.
    !registrant.is_empty()
        && registrant
            .split('.')
            .all(|seg| !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()))
}

/// Syntactic BCP-47 check: a primary subtag of 2–3 or 5–8 ASCII letters,
/// then zero or more `-`-separated subtags of 1–8 alphanumerics. This
/// covers the common `en`, `en-GB`, `zh-Hans`, `de-DE-1996` shapes
/// without consulting the IANA registry. (Length 4 is reserved for
/// script subtags and is not a valid primary subtag.)
#[must_use]
fn is_valid_bcp47(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    let n = primary.len();
    if !((2..=3).contains(&n) || (5..=8).contains(&n)) {
        return false;
    }
    if !primary.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    parts.all(|sub| (1..=8).contains(&sub.len()) && sub.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// Verhoeff dihedral-group (D5) checksum validation. Returns `true` when
/// the trailing digit correctly checksums the preceding digits, which is
/// how SNOMED CT SCTIDs embed their check digit.
fn verhoeff_valid(digits: &str) -> bool {
    /// Multiplication table for the dihedral group D5.
    const D: [[u8; 10]; 10] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
        [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
        [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
        [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
        [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
        [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
        [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
        [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
        [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    ];
    /// Permutation table applied per position.
    const P: [[u8; 10]; 8] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
        [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
        [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
        [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
        [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
        [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
        [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
    ];

    let mut c: u8 = 0;
    // Process digits right-to-left; position 0 is the check digit itself.
    for (i, ch) in digits.bytes().rev().enumerate() {
        let d = ch - b'0';
        c = D[c as usize][P[i % 8][d as usize] as usize];
    }
    c == 0
}

/// Tests for the format validators: each clinical coding system, the
/// deterministic identifier shapes, and BCP-47 language tags, plus the
/// per-index problem reporting. All DB-free.
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a [`ConditionCode`] from a system + code (test brevity).
    fn cc(system: CodeSystem, code: &str) -> ConditionCode {
        ConditionCode {
            system,
            code: code.to_string(),
        }
    }

    /// Canonical ICD-10 codes (with and without `.` extensions) are accepted.
    #[test]
    fn icd10_accepts_canonical_codes() {
        for c in ["I63", "I63.9", "A00", "Z99", "C7A", "S72.001A", "M1A.0"] {
            assert!(is_valid_icd10(c), "should accept ICD-10 {c:?}");
        }
    }

    /// Malformed ICD-10 inputs (wrong shape, lowercase, ICD-11 form, …)
    /// are rejected.
    #[test]
    fn icd10_rejects_malformed_codes() {
        for c in [
            "",
            "63",
            "I6",
            "II3",
            "i63",
            "I63.",
            "I63.ABCDE",
            "1A00",
            "I639",
        ] {
            assert!(!is_valid_icd10(c), "should reject ICD-10 {c:?}");
        }
    }

    /// Valid ICD-11 MMS stem codes (second char a letter) are accepted.
    #[test]
    fn icd11_accepts_stem_codes() {
        for c in ["1A00", "BA00", "8B20.0", "ME24", "1C62.0Z"] {
            assert!(is_valid_icd11(c), "should accept ICD-11 {c:?}");
        }
    }

    /// Malformed ICD-11 inputs are rejected (see inline cases).
    #[test]
    fn icd11_rejects_malformed_codes() {
        // second char not a letter; contains O/I; too short; empty extension.
        for c in ["12", "1234", "1O00", "1I00", "A", "1A00.", "i a"] {
            assert!(!is_valid_icd11(c), "should reject ICD-11 {c:?}");
        }
    }

    /// Real, Verhoeff-correct SNOMED CT SCTIDs are accepted.
    #[test]
    fn snomed_accepts_valid_sctids() {
        // Real, Verhoeff-correct SNOMED CT identifiers.
        for c in ["22298006", "73211009", "386661006", "195967001"] {
            assert!(is_valid_snomed(c), "should accept SCTID {c:?}");
        }
    }

    /// A broken Verhoeff check digit or wrong shape (length / non-digit)
    /// is rejected.
    #[test]
    fn snomed_rejects_bad_check_digit_and_shape() {
        // 22298006 is valid; flipping the last digit breaks the checksum.
        assert!(!is_valid_snomed("22298007"));
        for c in ["", "123", "12345", "2229800A", "1234567890123456789"] {
            assert!(!is_valid_snomed(c), "should reject {c:?}");
        }
    }

    /// A `Custom` coding system imposes no format — only non-blankness.
    #[test]
    fn custom_only_requires_non_blank() {
        let p = CarePathway {
            condition_codes: vec![cc(CodeSystem::Custom("local".into()), "anything-goes")],
            ..CarePathway::new("X")
        };
        assert!(problems(&p).is_empty());

        let blank = CarePathway {
            condition_codes: vec![cc(CodeSystem::Custom("local".into()), "  ")],
            ..CarePathway::new("X")
        };
        assert_eq!(problems(&blank).len(), 1);
    }

    /// Every malformed code is reported, each tagged with its array index
    /// so the operator can locate it.
    #[test]
    fn problems_reports_every_bad_code_with_index() {
        let p = CarePathway {
            condition_codes: vec![
                cc(CodeSystem::Icd10, "I63.9"),     // ok
                cc(CodeSystem::Icd10, "nope"),      // bad → [1]
                cc(CodeSystem::Snomed, "22298007"), // bad check digit → [2]
            ],
            ..CarePathway::new("Stroke pathway")
        };
        let problems = problems(&p);
        assert_eq!(problems.len(), 2);
        assert!(problems[0].contains("condition_codes[1]"));
        assert!(problems[1].contains("condition_codes[2]"));
    }

    /// `Uuid`-scheme identifiers must be canonical 8-4-4-4-12 hex; wrapped
    /// or malformed values are rejected.
    #[test]
    fn uuid_identifiers_must_be_canonical() {
        for v in [
            "550e8400-e29b-41d4-a716-446655440000",
            "00000000-0000-0000-0000-000000000000",
            "550E8400-E29B-41D4-A716-446655440000", // case-insensitive
        ] {
            assert!(is_valid_uuid(v), "should accept UUID {v:?}");
        }
        for v in [
            "",
            "not-a-uuid",
            "550e8400e29b41d4a716446655440000",       // no hyphens
            "550e8400-e29b-41d4-a716-44665544000",    // too short
            "550e8400-e29b-41d4-a716-4466554400000",  // too long
            "550e8400-e29b-41d4-a716-44665544000g",   // non-hex
            "{550e8400-e29b-41d4-a716-446655440000}", // wrapped
        ] {
            assert!(!is_valid_uuid(v), "should reject UUID {v:?}");
        }
    }

    /// `Doi`-scheme identifiers require the `10.<registrant>/<suffix>`
    /// shape; the URL form is rejected.
    #[test]
    fn doi_identifiers_must_have_prefix_and_suffix() {
        for v in ["10.1000/xyz123", "10.1038/nphys1170", "10.1000.10/123"] {
            assert!(is_valid_doi(v), "should accept DOI {v:?}");
        }
        for v in [
            "",
            "1000/xyz",                       // missing 10. prefix
            "10.1000",                        // no slash
            "10.1000/",                       // empty suffix
            "10./xyz",                        // empty registrant
            "10.abc/xyz",                     // non-numeric registrant
            "https://doi.org/10.1000/xyz123", // URL form not accepted
        ] {
            assert!(!is_valid_doi(v), "should reject DOI {v:?}");
        }
    }

    /// Open-value-space deterministic schemes (e.g. `Uri`, `Wikidata`)
    /// only require a non-blank value.
    #[test]
    fn open_deterministic_schemes_only_require_non_blank() {
        let ok = PathwayIdentifier {
            scheme: IdentifierScheme::Uri,
            value: "urn:nice:ng128".into(),
        };
        assert!(identifier_problem(0, &ok).is_none());
        let blank = PathwayIdentifier {
            scheme: IdentifierScheme::Wikidata,
            value: "   ".into(),
        };
        assert!(identifier_problem(0, &blank).is_some());
    }

    /// A malformed identifier is reported tagged with its array index.
    #[test]
    fn malformed_identifier_is_reported_with_index() {
        let p = CarePathway {
            identifiers: vec![
                PathwayIdentifier {
                    scheme: IdentifierScheme::Uuid,
                    value: "550e8400-e29b-41d4-a716-446655440000".into(),
                }, // ok
                PathwayIdentifier {
                    scheme: IdentifierScheme::Doi,
                    value: "not-a-doi".into(),
                }, // bad → [1]
            ],
            ..CarePathway::new("Stroke pathway")
        };
        let problems = problems(&p);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("identifiers[1]"));
    }

    /// Common BCP-47 shapes are accepted and malformed tags rejected
    /// (see inline cases).
    #[test]
    fn bcp47_language_tags_are_checked() {
        for t in ["en", "en-GB", "zh-Hans", "de-DE-1996", "cy", "yue"] {
            assert!(is_valid_bcp47(t), "should accept BCP-47 {t:?}");
        }
        for t in [
            "",
            "e",     // primary too short
            "engl",  // length 4 reserved (not a primary subtag)
            "en_GB", // wrong separator
            "en-",   // empty subtag
            "en-toolongsubtag",
            "123", // primary not letters
        ] {
            assert!(!is_valid_bcp47(t), "should reject BCP-47 {t:?}");
        }
    }

    /// A malformed `in_language` entry is reported tagged with its index.
    #[test]
    fn malformed_language_tag_is_a_problem() {
        let p = CarePathway {
            in_language: vec!["en".into(), "nope_NO".into()],
            ..CarePathway::new("X")
        };
        let problems = problems(&p);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("in_language[1]"));
    }

    /// A blank `name` yields exactly the `name is required` problem.
    #[test]
    fn blank_name_is_a_problem() {
        assert_eq!(problems(&CarePathway::new("   ")), vec!["name is required"]);
    }

    /// A fully valid payload (good name, codes, identifiers, languages)
    /// produces no problems.
    #[test]
    fn valid_payload_has_no_problems() {
        let p = CarePathway {
            condition_codes: vec![
                cc(CodeSystem::Icd10, "I63.9"),
                cc(CodeSystem::Snomed, "22298006"),
            ],
            identifiers: vec![
                PathwayIdentifier {
                    scheme: IdentifierScheme::Uuid,
                    value: "550e8400-e29b-41d4-a716-446655440000".into(),
                },
                PathwayIdentifier {
                    scheme: IdentifierScheme::Doi,
                    value: "10.1038/nphys1170".into(),
                },
            ],
            in_language: vec!["en".into(), "en-GB".into()],
            ..CarePathway::new("Acute Stroke Care Pathway")
        };
        assert!(problems(&p).is_empty());
    }

    /// The §6.1 "all problems reported together" guarantee across *every*
    /// validated dimension at once: a blank `name`, a malformed
    /// `condition_codes` entry, a malformed `identifiers` entry, and a
    /// malformed `in_language` tag must each surface as its own problem in
    /// a single call — none masking another.
    #[test]
    fn problems_aggregates_across_all_dimensions() {
        let p = CarePathway {
            condition_codes: vec![cc(CodeSystem::Icd10, "not-a-code")],
            identifiers: vec![PathwayIdentifier {
                scheme: IdentifierScheme::Uuid,
                value: "not-a-uuid".into(),
            }],
            in_language: vec!["nope_NO".into()],
            ..CarePathway::new("   ") // blank name
        };
        let problems = problems(&p);
        assert_eq!(problems.len(), 4, "every dimension reports once: {problems:?}");
        assert!(problems.iter().any(|m| m.contains("name is required")));
        assert!(problems.iter().any(|m| m.contains("condition_codes[0]")));
        assert!(problems.iter().any(|m| m.contains("identifiers[0]")));
        assert!(problems.iter().any(|m| m.contains("in_language[0]")));
    }
}
