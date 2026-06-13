//! Field-level validation for incoming `CarePathway` payloads.
//!
//! The service stores the matcher's `CarePathway` verbatim, so payload
//! validation is the *service's* responsibility — the matcher is a pure
//! scoring library and deliberately performs no validation. These checks
//! enforce the required `name` and clinical code-format constraints on
//! `condition_codes`, returning human-readable problem strings that the
//! controller surfaces as a single `422 Unprocessable Entity`.
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
//! [Verhoeff]: https://en.wikipedia.org/wiki/Verhoeff_algorithm

use care_pathway_matcher::{CarePathway, CodeSystem, ConditionCode};

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
    out
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cc(system: CodeSystem, code: &str) -> ConditionCode {
        ConditionCode {
            system,
            code: code.to_string(),
        }
    }

    #[test]
    fn icd10_accepts_canonical_codes() {
        for c in ["I63", "I63.9", "A00", "Z99", "C7A", "S72.001A", "M1A.0"] {
            assert!(is_valid_icd10(c), "should accept ICD-10 {c:?}");
        }
    }

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

    #[test]
    fn icd11_accepts_stem_codes() {
        for c in ["1A00", "BA00", "8B20.0", "ME24", "1C62.0Z"] {
            assert!(is_valid_icd11(c), "should accept ICD-11 {c:?}");
        }
    }

    #[test]
    fn icd11_rejects_malformed_codes() {
        // second char not a letter; contains O/I; too short; empty extension.
        for c in ["12", "1234", "1O00", "1I00", "A", "1A00.", "i a"] {
            assert!(!is_valid_icd11(c), "should reject ICD-11 {c:?}");
        }
    }

    #[test]
    fn snomed_accepts_valid_sctids() {
        // Real, Verhoeff-correct SNOMED CT identifiers.
        for c in ["22298006", "73211009", "386661006", "195967001"] {
            assert!(is_valid_snomed(c), "should accept SCTID {c:?}");
        }
    }

    #[test]
    fn snomed_rejects_bad_check_digit_and_shape() {
        // 22298006 is valid; flipping the last digit breaks the checksum.
        assert!(!is_valid_snomed("22298007"));
        for c in ["", "123", "12345", "2229800A", "1234567890123456789"] {
            assert!(!is_valid_snomed(c), "should reject {c:?}");
        }
    }

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

    #[test]
    fn blank_name_is_a_problem() {
        assert_eq!(problems(&CarePathway::new("   ")), vec!["name is required"]);
    }

    #[test]
    fn valid_payload_has_no_problems() {
        let p = CarePathway {
            condition_codes: vec![
                cc(CodeSystem::Icd10, "I63.9"),
                cc(CodeSystem::Snomed, "22298006"),
            ],
            ..CarePathway::new("Acute Stroke Care Pathway")
        };
        assert!(problems(&p).is_empty());
    }
}
