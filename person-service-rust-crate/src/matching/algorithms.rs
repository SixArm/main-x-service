//! Field-level matching algorithms used by the scoring layer.
//!
//! Each submodule scores one comparable facet of a person record and
//! returns a value in `[0.0, 1.0]`, where `1.0` is a confident match and
//! `0.0` is no match (or insufficient data). The scorers in
//! [`crate::matching::scoring`] combine these per-field scores into an
//! overall weighted result.
//!
//! | Submodule | Compares | Technique |
//! |-----------|----------|-----------|
//! | [`name_matching`](crate::matching::algorithms::name_matching) | names | Jaro-Winkler + Levenshtein + Soundex + nickname table |
//! | [`dob_matching`](crate::matching::algorithms::dob_matching) | birth dates | exact / typo-tolerant proximity |
//! | [`gender_matching`](crate::matching::algorithms::gender_matching) | gender | exact, with `Unknown` neutral |
//! | [`address_matching`](crate::matching::algorithms::address_matching) | addresses | weighted postal/city/state/street |
//! | [`identifier_matching`](crate::matching::algorithms::identifier_matching) | identifiers | type + system + value exact |
//! | [`tax_id_matching`](crate::matching::algorithms::tax_id_matching) | tax IDs | normalized exact (deterministic) |
//! | [`document_matching`](crate::matching::algorithms::document_matching) | ID documents | type + number exact |
//!
//! All functions here are pure (no I/O), which makes them cheap to unit
//! test and safe to call from hot matching loops.

use jiff::civil::Date;
use strsim::{jaro_winkler, normalized_levenshtein};

use crate::models::{Address, HumanName, Identifier, IdentityDocument};

/// Name comparison: family + given + prefix/suffix, fuzzy and phonetic.
pub mod name_matching {
    use super::*;

    /// Score overall name similarity in `[0.0, 1.0]`.
    ///
    /// Combines three weighted components: family name (0.5), given
    /// names (0.4), and prefix/suffix (0.1).
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::matching::algorithms::name_matching::match_names;
    /// use person_service::models::HumanName;
    ///
    /// let a = HumanName { use_type: None, family: "Smith".into(), given: vec!["John".into()], prefix: vec![], suffix: vec![] };
    /// let b = a.clone();
    /// assert!(match_names(&a, &b) > 0.99);
    /// ```
    pub fn match_names(name1: &HumanName, name2: &HumanName) -> f64 {
        // Weight factors for different components
        const FAMILY_WEIGHT: f64 = 0.5;
        const GIVEN_WEIGHT: f64 = 0.4;
        const PREFIX_SUFFIX_WEIGHT: f64 = 0.1;

        let family_score = match_family_names(&name1.family, &name2.family);
        let given_score = match_given_names(&name1.given, &name2.given);
        let prefix_suffix_score =
            match_prefix_suffix(&name1.prefix, &name2.prefix, &name1.suffix, &name2.suffix);

        (family_score * FAMILY_WEIGHT)
            + (given_score * GIVEN_WEIGHT)
            + (prefix_suffix_score * PREFIX_SUFFIX_WEIGHT)
    }

    /// Score family-name similarity in `[0.0, 1.0]`.
    ///
    /// Returns `0.0` if either name is empty, `1.0` on a
    /// case-insensitive exact match, and otherwise the maximum of three
    /// fuzzy measures: Jaro-Winkler, normalized Levenshtein, and a
    /// Soundex-phonetic floor (`0.85` when the names sound alike).
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::matching::algorithms::name_matching::match_family_names;
    ///
    /// assert_eq!(match_family_names("Smith", "Smith"), 1.0);
    /// assert!(match_family_names("Smith", "Smyth") > 0.8);
    /// assert_eq!(match_family_names("", "Smith"), 0.0);
    /// ```
    pub fn match_family_names(family1: &str, family2: &str) -> f64 {
        if family1.is_empty() || family2.is_empty() {
            return 0.0;
        }

        // Normalize: lowercase and trim
        let f1 = family1.trim().to_lowercase();
        let f2 = family2.trim().to_lowercase();

        // Exact match
        if f1 == f2 {
            return 1.0;
        }

        // Use Jaro-Winkler (good for name matching)
        let jw_score = jaro_winkler(&f1, &f2);

        // Use normalized Levenshtein distance
        let lev_score = normalized_levenshtein(&f1, &f2);

        // Phonetic matching (Soundex)
        let phonetic_score = crate::matching::phonetic::phonetic_similarity(&f1, &f2);
        // Phonetic match provides a floor — if names sound alike, score at least 0.85
        let phonetic_boost = if phonetic_score >= 1.0 { 0.85 } else { 0.0 };

        // Take the maximum of all methods
        f64::max(f64::max(jw_score, lev_score), phonetic_boost)
    }

    /// Score given-name similarity in `[0.0, 1.0]`.
    ///
    /// Compares only the *first* given name of each list. Returns `0.0`
    /// when either list is empty, `1.0` on an exact match, `0.95` when
    /// the two are known nickname variants (see `are_name_variants`),
    /// and otherwise the max of Jaro-Winkler and normalized Levenshtein.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::matching::algorithms::name_matching::match_given_names;
    ///
    /// assert_eq!(match_given_names(&["John".into()], &["John".into()]), 1.0);
    /// assert_eq!(match_given_names(&["William".into()], &["Bill".into()]), 0.95);
    /// assert_eq!(match_given_names(&[], &["John".into()]), 0.0);
    /// ```
    pub fn match_given_names(given1: &[String], given2: &[String]) -> f64 {
        if given1.is_empty() || given2.is_empty() {
            return 0.0;
        }

        // Compare first names primarily
        let first1 = given1.first().unwrap().trim().to_lowercase();
        let first2 = given2.first().unwrap().trim().to_lowercase();

        if first1 == first2 {
            return 1.0;
        }

        // Check for common nicknames/variants
        if are_name_variants(&first1, &first2) {
            return 0.95;
        }

        // Fuzzy match
        let jw_score = jaro_winkler(&first1, &first2);
        let lev_score = normalized_levenshtein(&first1, &first2);

        f64::max(jw_score, lev_score)
    }

    /// Return `true` when two (lowercased) given names belong to the
    /// same nickname group.
    ///
    /// Backed by a small hard-coded table of common English nickname
    /// clusters (e.g. `william`/`bill`/`will`). Both inputs must already
    /// be lowercased — callers normalize before calling.
    fn are_name_variants(name1: &str, name2: &str) -> bool {
        // Common name variants (simplified list)
        let variants = [
            vec!["william", "bill", "billy", "will"],
            vec!["robert", "bob", "bobby", "rob"],
            vec!["richard", "dick", "rick", "ricky"],
            vec!["james", "jim", "jimmy", "jamie"],
            vec!["john", "jack", "johnny"],
            vec!["michael", "mike", "mickey"],
            vec!["elizabeth", "liz", "beth", "betty", "betsy"],
            vec!["margaret", "maggie", "meg", "peggy"],
            vec!["catherine", "cathy", "kate", "katie"],
            vec!["jennifer", "jen", "jenny"],
            vec!["christopher", "chris"],
            vec!["anthony", "tony"],
            vec!["thomas", "tom", "tommy"],
            vec!["joseph", "joe", "joey"],
            vec!["charles", "charlie", "chuck"],
        ];

        for variant_group in &variants {
            if variant_group.contains(&name1) && variant_group.contains(&name2) {
                return true;
            }
        }

        false
    }

    /// Score the combined prefix/suffix similarity in `[0.0, 1.0]`.
    ///
    /// Each side (prefixes, suffixes) scores `1.0` when both lists are
    /// empty (nothing to disagree on), `0.5` when exactly one side is
    /// empty (partial evidence), or the best pairwise Jaro-Winkler score
    /// otherwise. The two sides are averaged.
    fn match_prefix_suffix(
        prefix1: &[String],
        prefix2: &[String],
        suffix1: &[String],
        suffix2: &[String],
    ) -> f64 {
        let prefix_match = if prefix1.is_empty() && prefix2.is_empty() {
            1.0
        } else if prefix1.is_empty() || prefix2.is_empty() {
            0.5
        } else {
            // Check if any prefix matches
            let mut max_score = 0.0;
            for p1 in prefix1 {
                for p2 in prefix2 {
                    let score = jaro_winkler(&p1.to_lowercase(), &p2.to_lowercase());
                    max_score = f64::max(max_score, score);
                }
            }
            max_score
        };

        let suffix_match = if suffix1.is_empty() && suffix2.is_empty() {
            1.0
        } else if suffix1.is_empty() || suffix2.is_empty() {
            0.5
        } else {
            // Check if any suffix matches
            let mut max_score = 0.0;
            for s1 in suffix1 {
                for s2 in suffix2 {
                    let score = jaro_winkler(&s1.to_lowercase(), &s2.to_lowercase());
                    max_score = f64::max(max_score, score);
                }
            }
            max_score
        };

        (prefix_match + suffix_match) / 2.0
    }
}

/// Birth-date comparison tolerant of common data-entry errors.
pub mod dob_matching {
    use super::*;

    /// Score birth-date similarity in `[0.0, 1.0]`, tolerating typos.
    ///
    /// Two missing dates are neutral (`0.5`); one missing date scores
    /// `0.0`. For two present dates the score is graded by how the
    /// values differ — exact (`1.0`), day off by 1-2 (`0.95`),
    /// month/day transposition (`0.90`), year off by one (`0.85`), same
    /// year+month (`0.80`), same year only (`0.50`), else `0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use jiff::civil::date;
    /// use person_service::matching::algorithms::dob_matching::match_birth_dates;
    ///
    /// let d = Some(date(1980, 1, 15));
    /// assert_eq!(match_birth_dates(d, d), 1.0);
    /// assert_eq!(match_birth_dates(None, None), 0.5);
    /// assert_eq!(match_birth_dates(d, None), 0.0);
    /// ```
    pub fn match_birth_dates(dob1: Option<Date>, dob2: Option<Date>) -> f64 {
        match (dob1, dob2) {
            (None, None) => 0.5,                      // Both missing - neutral
            (None, Some(_)) | (Some(_), None) => 0.0, // One missing - no match
            (Some(d1), Some(d2)) => {
                if d1 == d2 {
                    return 1.0; // Exact match
                }

                // Allow for common data entry errors
                let days_diff = (d1 - d2).get_days().abs();

                // Same month and year, day off by 1-2 (typo)
                if d1.year() == d2.year() && d1.month() == d2.month() {
                    if days_diff <= 2 {
                        return 0.95;
                    }
                }

                // Month/day transposition (e.g., 03/12 vs 12/03)
                if d1.year() == d2.year() && d1.month() == d2.day() && d1.day() == d2.month() {
                    return 0.90;
                }

                // Same year and month
                if d1.year() == d2.year() && d1.month() == d2.month() {
                    return 0.80;
                }

                // Same year, different month
                if d1.year() == d2.year() {
                    return 0.50;
                }

                // Year off by 1 (typo in year)
                if (d1.year() - d2.year()).abs() == 1
                    && d1.month() == d2.month()
                    && d1.day() == d2.day()
                {
                    return 0.85;
                }

                // No significant match
                0.0
            }
        }
    }
}

/// Gender comparison with neutral handling of `Unknown`.
pub mod gender_matching {
    use crate::models::Gender;

    /// Score gender similarity in `[0.0, 1.0]`.
    ///
    /// Returns `1.0` on an exact match, `0.5` when either side is
    /// [`Gender::Unknown`] (insufficient evidence to penalize), and
    /// `0.0` on a definite mismatch.
    ///
    /// # Examples
    ///
    /// ```
    /// use person_service::models::Gender;
    /// use person_service::matching::algorithms::gender_matching::match_gender;
    ///
    /// assert_eq!(match_gender(Gender::Male, Gender::Male), 1.0);
    /// assert_eq!(match_gender(Gender::Male, Gender::Unknown), 0.5);
    /// assert_eq!(match_gender(Gender::Male, Gender::Female), 0.0);
    /// ```
    pub fn match_gender(gender1: Gender, gender2: Gender) -> f64 {
        if gender1 == gender2 {
            1.0
        } else if gender1 == Gender::Unknown || gender2 == Gender::Unknown {
            0.5 // Unknown is neutral
        } else {
            0.0 // Mismatch
        }
    }
}

/// Postal-address comparison across postal code, city, state, street.
pub mod address_matching {
    use super::*;

    /// Score the best address pair between two address lists.
    ///
    /// Currently compares only the *primary* (first) address of each
    /// list via [`match_address`]. Returns `0.0` if either list is
    /// empty.
    pub fn match_addresses(addresses1: &[Address], addresses2: &[Address]) -> f64 {
        if addresses1.is_empty() || addresses2.is_empty() {
            return 0.0;
        }

        // Compare primary addresses if available
        let addr1 = addresses1.first().unwrap();
        let addr2 = addresses2.first().unwrap();

        match_address(addr1, addr2)
    }

    /// Score two individual addresses in `[0.0, 1.0]`.
    ///
    /// A weighted sum of four components: postal code (0.3), city (0.2),
    /// state (0.2), and street line 1 (0.3). Missing components on
    /// either side contribute `0.0`.
    pub fn match_address(addr1: &Address, addr2: &Address) -> f64 {
        const POSTAL_CODE_WEIGHT: f64 = 0.3;
        const CITY_WEIGHT: f64 = 0.2;
        const STATE_WEIGHT: f64 = 0.2;
        const STREET_WEIGHT: f64 = 0.3;

        let postal_score =
            match_postal_codes(addr1.postal_code.as_deref(), addr2.postal_code.as_deref());

        let city_score = match_cities(addr1.city.as_deref(), addr2.city.as_deref());

        let state_score = match_states(addr1.state.as_deref(), addr2.state.as_deref());

        let street_score = match_street_addresses(addr1.line1.as_deref(), addr2.line1.as_deref());

        (postal_score * POSTAL_CODE_WEIGHT)
            + (city_score * CITY_WEIGHT)
            + (state_score * STATE_WEIGHT)
            + (street_score * STREET_WEIGHT)
    }

    /// Score postal-code similarity in `[0.0, 1.0]`.
    ///
    /// After stripping hyphens, an exact match scores `1.0`; sharing the
    /// first five digits (the US ZIP without the +4) scores `0.95`;
    /// sharing the first three digits (same sectional area) scores
    /// `0.70`; anything else (or a missing code) scores `0.0`.
    pub(crate) fn match_postal_codes(zip1: Option<&str>, zip2: Option<&str>) -> f64 {
        match (zip1, zip2) {
            (None, None) => 0.0,
            (None, Some(_)) | (Some(_), None) => 0.0,
            (Some(z1), Some(z2)) => {
                let z1 = z1.trim().replace("-", "");
                let z2 = z2.trim().replace("-", "");

                if z1 == z2 {
                    return 1.0;
                }

                // Match first 5 digits (US ZIP)
                if z1.len() >= 5 && z2.len() >= 5 {
                    if &z1[0..5] == &z2[0..5] {
                        return 0.95;
                    }
                }

                // Match first 3 digits (same area)
                if z1.len() >= 3 && z2.len() >= 3 {
                    if &z1[0..3] == &z2[0..3] {
                        return 0.70;
                    }
                }

                0.0
            }
        }
    }

    /// Score city similarity in `[0.0, 1.0]`.
    ///
    /// Exact (case-insensitive) match scores `1.0`; otherwise a
    /// Jaro-Winkler fuzzy score absorbs typos. A missing city on either
    /// side scores `0.0`.
    fn match_cities(city1: Option<&str>, city2: Option<&str>) -> f64 {
        match (city1, city2) {
            (None, None) => 0.0,
            (None, Some(_)) | (Some(_), None) => 0.0,
            (Some(c1), Some(c2)) => {
                let c1 = c1.trim().to_lowercase();
                let c2 = c2.trim().to_lowercase();

                if c1 == c2 {
                    return 1.0;
                }

                // Fuzzy match for typos
                jaro_winkler(&c1, &c2)
            }
        }
    }

    /// Score state/province similarity as a binary `1.0`/`0.0`.
    ///
    /// State codes are short and standardized, so only an exact
    /// (uppercased) match counts; a missing state on either side scores
    /// `0.0`.
    fn match_states(state1: Option<&str>, state2: Option<&str>) -> f64 {
        match (state1, state2) {
            (None, None) => 0.0,
            (None, Some(_)) | (Some(_), None) => 0.0,
            (Some(s1), Some(s2)) => {
                let s1 = s1.trim().to_uppercase();
                let s2 = s2.trim().to_uppercase();

                if s1 == s2 { 1.0 } else { 0.0 }
            }
        }
    }

    /// Score street-line similarity in `[0.0, 1.0]`.
    ///
    /// Both sides are normalized (see [`normalize_street`]) before
    /// comparison; an exact normalized match scores `1.0`, otherwise a
    /// Jaro-Winkler fuzzy score applies. A missing street scores `0.0`.
    fn match_street_addresses(street1: Option<&str>, street2: Option<&str>) -> f64 {
        match (street1, street2) {
            (None, None) => 0.0,
            (None, Some(_)) | (Some(_), None) => 0.0,
            (Some(s1), Some(s2)) => {
                let s1 = normalize_street(s1);
                let s2 = normalize_street(s2);

                if s1 == s2 {
                    return 1.0;
                }

                // Fuzzy match
                jaro_winkler(&s1, &s2)
            }
        }
    }

    /// Canonicalize a street line for comparison.
    ///
    /// Lowercases, trims, expands common street-type words to their
    /// abbreviations (`street`→`st`, `avenue`→`ave`, …), and drops `.`
    /// and `,` so that e.g. `"123 Main Street"` and `"123 Main St."`
    /// compare equal.
    fn normalize_street(street: &str) -> String {
        street
            .trim()
            .to_lowercase()
            .replace("street", "st")
            .replace("avenue", "ave")
            .replace("road", "rd")
            .replace("drive", "dr")
            .replace("boulevard", "blvd")
            .replace("lane", "ln")
            .replace("court", "ct")
            .replace("circle", "cir")
            .replace(".", "")
            .replace(",", "")
    }
}

/// Identifier comparison: type + system + value exact match.
pub mod identifier_matching {
    use super::*;

    /// Score the best identifier pair across two identifier lists.
    ///
    /// Returns the maximum [`match_identifier`] score over every
    /// cross-pair, or `0.0` if either list is empty.
    pub fn match_identifiers(ids1: &[Identifier], ids2: &[Identifier]) -> f64 {
        if ids1.is_empty() || ids2.is_empty() {
            return 0.0;
        }

        let mut max_score = 0.0;

        for id1 in ids1 {
            for id2 in ids2 {
                let score = match_identifier(id1, id2);
                max_score = f64::max(max_score, score);
            }
        }

        max_score
    }

    /// Score two identifiers in `[0.0, 1.0]`.
    ///
    /// The identifiers must agree on both `identifier_type` and `system`
    /// (otherwise `0.0`). Values then compare exactly for `1.0`, or
    /// `0.98` when they differ only by hyphen/space formatting.
    pub fn match_identifier(id1: &Identifier, id2: &Identifier) -> f64 {
        // Must be same type and system
        if id1.identifier_type != id2.identifier_type {
            return 0.0;
        }

        if id1.system != id2.system {
            return 0.0;
        }

        // Compare values
        let v1 = id1.value.trim().to_lowercase();
        let v2 = id2.value.trim().to_lowercase();

        if v1 == v2 {
            1.0 // Exact match
        } else {
            // Allow minor differences (e.g., formatting)
            let v1_clean = v1.replace("-", "").replace(" ", "");
            let v2_clean = v2.replace("-", "").replace(" ", "");

            if v1_clean == v2_clean {
                0.98 // Formatting difference
            } else {
                0.0 // Different values
            }
        }
    }
}

/// Deterministic tax-ID comparison (a scoring short-circuit signal).
pub mod tax_id_matching {
    use crate::models::Person;

    /// Score two persons' effective tax IDs as a binary `1.0`/`0.0`.
    ///
    /// Reads each person's [`Person::effective_tax_id`], normalizes both
    /// (alphanumerics only, lowercased — see `normalize_tax_id`), and
    /// returns `1.0` only on a non-empty exact match. A missing tax ID
    /// on either side yields `0.0`. A `1.0` here drives the deterministic
    /// scorer's short-circuit to a certain match.
    pub fn match_tax_ids(person: &Person, candidate: &Person) -> f64 {
        let tid1 = person.effective_tax_id();
        let tid2 = candidate.effective_tax_id();

        match (tid1, tid2) {
            (Some(t1), Some(t2)) => {
                let t1 = normalize_tax_id(t1);
                let t2 = normalize_tax_id(t2);
                if !t1.is_empty() && t1 == t2 { 1.0 } else { 0.0 }
            }
            _ => 0.0,
        }
    }

    /// Canonicalize a tax ID by keeping only ASCII alphanumerics and
    /// lowercasing, so that formatting like `123-45-6789` compares equal
    /// to `123456789`.
    fn normalize_tax_id(tid: &str) -> String {
        tid.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_lowercase()
    }
}

/// Identity-document comparison: type + number exact match.
pub mod document_matching {
    use super::*;

    /// Score the best document pair across two document lists.
    ///
    /// Returns the maximum [`match_document`] score over every
    /// cross-pair, or `0.0` if either list is empty.
    pub fn match_documents(docs1: &[IdentityDocument], docs2: &[IdentityDocument]) -> f64 {
        if docs1.is_empty() || docs2.is_empty() {
            return 0.0;
        }

        let mut max_score = 0.0;

        for d1 in docs1 {
            for d2 in docs2 {
                let score = match_document(d1, d2);
                max_score = f64::max(max_score, score);
            }
        }

        max_score
    }

    /// Score two identity documents in `[0.0, 1.0]`.
    ///
    /// The documents must share a `document_type` (otherwise `0.0`).
    /// Numbers are normalized (uppercased, with `-`, ` `, `.` removed)
    /// before comparison; an exact number match scores `1.0` when the
    /// issuing country also matches, or `0.95` when it differs. An empty
    /// or mismatched number scores `0.0`.
    pub fn match_document(doc1: &IdentityDocument, doc2: &IdentityDocument) -> f64 {
        // Must be same document type
        if doc1.document_type != doc2.document_type {
            return 0.0;
        }

        // Compare document numbers after normalization
        let n1 = doc1
            .number
            .trim()
            .to_uppercase()
            .replace(['-', ' ', '.'], "");
        let n2 = doc2
            .number
            .trim()
            .to_uppercase()
            .replace(['-', ' ', '.'], "");

        if n1.is_empty() || n2.is_empty() {
            return 0.0;
        }

        if n1 == n2 {
            // Boost score if issuing country also matches
            if doc1.issuing_country == doc2.issuing_country {
                1.0
            } else {
                0.95
            }
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two identical names score essentially 1.0.
    #[test]
    fn test_exact_name_match() {
        let name1 = HumanName {
            use_type: None,
            family: "Smith".to_string(),
            given: vec!["John".to_string()],
            prefix: vec![],
            suffix: vec![],
        };

        let name2 = name1.clone();

        let score = name_matching::match_names(&name1, &name2);
        assert!(score > 0.99, "Exact match should score ~1.0, got {}", score);
    }

    /// A one-letter spelling variant (Smith/Smyth) still scores high.
    #[test]
    fn test_fuzzy_name_match() {
        let name1 = HumanName {
            use_type: None,
            family: "Smith".to_string(),
            given: vec!["John".to_string()],
            prefix: vec![],
            suffix: vec![],
        };

        let name2 = HumanName {
            use_type: None,
            family: "Smyth".to_string(), // Spelling variant
            given: vec!["John".to_string()],
            prefix: vec![],
            suffix: vec![],
        };

        let score = name_matching::match_names(&name1, &name2);
        assert!(
            score > 0.85,
            "Similar names should score high, got {}",
            score
        );
    }

    /// Nickname variants (William/Bill) score high via the variant table.
    #[test]
    fn test_name_variants() {
        let name1 = HumanName {
            use_type: None,
            family: "Smith".to_string(),
            given: vec!["William".to_string()],
            prefix: vec![],
            suffix: vec![],
        };

        let name2 = HumanName {
            use_type: None,
            family: "Smith".to_string(),
            given: vec!["Bill".to_string()],
            prefix: vec![],
            suffix: vec![],
        };

        let score = name_matching::match_names(&name1, &name2);
        assert!(
            score > 0.90,
            "Name variants should score high, got {}",
            score
        );
    }

    /// Identical birth dates score 1.0.
    #[test]
    fn test_exact_dob_match() {
        let dob = Some(jiff::civil::date(1980, 1, 15));
        let score = dob_matching::match_birth_dates(dob, dob);
        assert_eq!(score, 1.0);
    }

    /// A single-day difference is treated as a typo and scores high.
    #[test]
    fn test_dob_typo() {
        let dob1 = Some(jiff::civil::date(1980, 1, 15));
        let dob2 = Some(jiff::civil::date(1980, 1, 16)); // Day off by 1
        let score = dob_matching::match_birth_dates(dob1, dob2);
        assert!(
            score > 0.90,
            "Minor DOB typo should score high, got {}",
            score
        );
    }

    /// Gender scoring: equal=1.0, mismatch=0.0, unknown=0.5.
    #[test]
    fn test_gender_match() {
        use crate::models::Gender;

        assert_eq!(
            gender_matching::match_gender(Gender::Male, Gender::Male),
            1.0
        );
        assert_eq!(
            gender_matching::match_gender(Gender::Male, Gender::Female),
            0.0
        );
        assert_eq!(
            gender_matching::match_gender(Gender::Male, Gender::Unknown),
            0.5
        );
    }

    /// Exact ZIP scores 1.0; ZIP+4 vs 5-digit shares the prefix and scores high.
    #[test]
    fn test_postal_code_match() {
        let score = address_matching::match_postal_codes(Some("12345"), Some("12345"));
        assert_eq!(score, 1.0);

        let score = address_matching::match_postal_codes(Some("12345-6789"), Some("12345"));
        assert!(score > 0.90);
    }

    /// Empty names yield a low overall score (no positive evidence).
    #[test]
    fn test_name_match_empty_strings() {
        let name1 = HumanName {
            use_type: None,
            family: "".to_string(),
            given: vec![],
            prefix: vec![],
            suffix: vec![],
        };
        let name2 = HumanName {
            use_type: None,
            family: "".to_string(),
            given: vec![],
            prefix: vec![],
            suffix: vec![],
        };
        let score = name_matching::match_names(&name1, &name2);
        assert!(score < 0.5, "Empty names should score low, got {}", score);
    }

    /// Near-spellings like Muller/Mueller score reasonably high.
    #[test]
    fn test_name_match_unicode_characters() {
        let name1 = HumanName {
            use_type: None,
            family: "Muller".to_string(),
            given: vec!["Hans".to_string()],
            prefix: vec![],
            suffix: vec![],
        };
        let name2 = HumanName {
            use_type: None,
            family: "Mueller".to_string(),
            given: vec!["Hans".to_string()],
            prefix: vec![],
            suffix: vec![],
        };
        let score = name_matching::match_names(&name1, &name2);
        assert!(
            score > 0.70,
            "Unicode-similar names should score reasonably, got {}",
            score
        );
    }

    /// Name matching is case-insensitive (SMITH == smith).
    #[test]
    fn test_name_match_case_insensitivity() {
        let name1 = HumanName {
            use_type: None,
            family: "SMITH".to_string(),
            given: vec!["JOHN".to_string()],
            prefix: vec![],
            suffix: vec![],
        };
        let name2 = HumanName {
            use_type: None,
            family: "smith".to_string(),
            given: vec!["john".to_string()],
            prefix: vec![],
            suffix: vec![],
        };
        let score = name_matching::match_names(&name1, &name2);
        assert!(
            score > 0.99,
            "Case-insensitive match should score ~1.0, got {}",
            score
        );
    }

    /// Same year/month/day scores exactly 1.0.
    #[test]
    fn test_dob_match_exact() {
        let dob1 = Some(jiff::civil::date(1990, 6, 15));
        let dob2 = Some(jiff::civil::date(1990, 6, 15));
        let score = dob_matching::match_birth_dates(dob1, dob2);
        assert_eq!(score, 1.0, "Exact DOB match should be 1.0");
    }

    /// A one-year-off date with the same month/day scores high.
    #[test]
    fn test_dob_match_off_by_one_year() {
        let dob1 = Some(jiff::civil::date(1980, 3, 10));
        let dob2 = Some(jiff::civil::date(1981, 3, 10));
        let score = dob_matching::match_birth_dates(dob1, dob2);
        assert!(
            score > 0.80,
            "Off-by-one year with same month/day should score high, got {}",
            score
        );
    }

    /// Both-None is neutral (0.5); one-None scores 0.0.
    #[test]
    fn test_dob_match_none_values() {
        let dob = Some(jiff::civil::date(1980, 1, 15));
        assert_eq!(
            dob_matching::match_birth_dates(None, None),
            0.5,
            "Both None should be neutral 0.5"
        );
        assert_eq!(
            dob_matching::match_birth_dates(dob, None),
            0.0,
            "One None should be 0.0"
        );
        assert_eq!(
            dob_matching::match_birth_dates(None, dob),
            0.0,
            "One None should be 0.0"
        );
    }

    /// Equal genders (including Other) score 1.0.
    #[test]
    fn test_gender_match_same() {
        use crate::models::Gender;
        assert_eq!(
            gender_matching::match_gender(Gender::Female, Gender::Female),
            1.0
        );
        assert_eq!(
            gender_matching::match_gender(Gender::Other, Gender::Other),
            1.0
        );
    }

    /// Two known but differing genders score 0.0.
    #[test]
    fn test_gender_match_different() {
        use crate::models::Gender;
        assert_eq!(
            gender_matching::match_gender(Gender::Male, Gender::Female),
            0.0
        );
        assert_eq!(
            gender_matching::match_gender(Gender::Female, Gender::Other),
            0.0
        );
    }

    /// Unknown vs known is neutral (0.5); Unknown vs Unknown is 1.0.
    #[test]
    fn test_gender_match_unknown() {
        use crate::models::Gender;
        assert_eq!(
            gender_matching::match_gender(Gender::Unknown, Gender::Male),
            0.5
        );
        assert_eq!(
            gender_matching::match_gender(Gender::Female, Gender::Unknown),
            0.5
        );
        assert_eq!(
            gender_matching::match_gender(Gender::Unknown, Gender::Unknown),
            1.0
        );
    }

    /// An address compared to itself scores essentially 1.0.
    #[test]
    fn test_address_match_exact() {
        let addr = Address {
            use_type: None,
            line1: Some("123 Main Street".to_string()),
            line2: None,
            city: Some("Springfield".to_string()),
            state: Some("IL".to_string()),
            postal_code: Some("62701".to_string()),
            country: Some("US".to_string()),
        };
        let score = address_matching::match_addresses(&[addr.clone()], &[addr]);
        assert!(
            score > 0.99,
            "Exact address match should score ~1.0, got {}",
            score
        );
    }

    /// Shared city/state but differing street/ZIP scores between 0 and 1.
    #[test]
    fn test_address_match_partial() {
        let addr1 = Address {
            use_type: None,
            line1: Some("123 Main Street".to_string()),
            line2: None,
            city: Some("Springfield".to_string()),
            state: Some("IL".to_string()),
            postal_code: Some("62701".to_string()),
            country: None,
        };
        let addr2 = Address {
            use_type: None,
            line1: Some("456 Oak Avenue".to_string()),
            line2: None,
            city: Some("Springfield".to_string()),
            state: Some("IL".to_string()),
            postal_code: Some("62702".to_string()),
            country: None,
        };
        let score = address_matching::match_addresses(&[addr1], &[addr2]);
        assert!(
            score > 0.0,
            "Partial address match (same city/state) should score > 0, got {}",
            score
        );
        assert!(score < 1.0, "Partial match should be < 1.0");
    }

    /// Empty address lists (either or both) score 0.0.
    #[test]
    fn test_address_match_empty() {
        let addr = Address {
            use_type: None,
            line1: None,
            line2: None,
            city: None,
            state: None,
            postal_code: None,
            country: None,
        };
        let score = address_matching::match_addresses(&[], &[addr]);
        assert_eq!(score, 0.0, "Empty address list should score 0.0");
        let score2 = address_matching::match_addresses(&[], &[]);
        assert_eq!(score2, 0.0, "Both empty address lists should score 0.0");
    }

    /// Same type+system+value identifiers score 1.0.
    #[test]
    fn test_identifier_match_exact() {
        let id1 = Identifier::new(
            crate::models::IdentifierType::MRN,
            "urn:oid:facility:hospital-a".to_string(),
            "MRN-12345".to_string(),
        );
        let id2 = Identifier::new(
            crate::models::IdentifierType::MRN,
            "urn:oid:facility:hospital-a".to_string(),
            "MRN-12345".to_string(),
        );
        let score = identifier_matching::match_identifiers(&[id1], &[id2]);
        assert_eq!(score, 1.0, "Exact identifier match should be 1.0");
    }

    /// Differing identifier types never match, even with equal values.
    #[test]
    fn test_identifier_match_different_type() {
        let id1 = Identifier::new(
            crate::models::IdentifierType::MRN,
            "urn:oid:facility:hospital-a".to_string(),
            "12345".to_string(),
        );
        let id2 = Identifier::new(
            crate::models::IdentifierType::SSN,
            "urn:oid:facility:hospital-a".to_string(),
            "12345".to_string(),
        );
        let score = identifier_matching::match_identifiers(&[id1], &[id2]);
        assert_eq!(score, 0.0, "Different identifier types should not match");
    }

    /// Equal tax IDs (after normalization) score 1.0.
    #[test]
    fn test_tax_id_match_exact() {
        use crate::models::{Gender, HumanName, Person};
        let mut p1 = Person::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        p1.tax_id = Some("123-45-6789".to_string());

        let mut p2 = Person::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        p2.tax_id = Some("123-45-6789".to_string());

        let score = tax_id_matching::match_tax_ids(&p1, &p2);
        assert_eq!(score, 1.0, "Exact tax ID match should be 1.0");
    }

    /// Persons without tax IDs score 0.0.
    #[test]
    fn test_tax_id_match_none() {
        use crate::models::{Gender, HumanName, Person};
        let p1 = Person::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let p2 = Person::new(
            HumanName {
                use_type: None,
                family: "Smith".into(),
                given: vec!["John".into()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        let score = tax_id_matching::match_tax_ids(&p1, &p2);
        assert_eq!(score, 0.0, "Both None tax IDs should score 0.0");
    }

    /// Same type+number+country documents score 1.0 (verified flag ignored).
    #[test]
    fn test_document_match_exact() {
        let doc1 = IdentityDocument {
            document_type: crate::models::DocumentType::Passport,
            number: "X12345678".to_string(),
            issuing_country: Some("US".to_string()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: true,
        };
        let doc2 = IdentityDocument {
            document_type: crate::models::DocumentType::Passport,
            number: "X12345678".to_string(),
            issuing_country: Some("US".to_string()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: false,
        };
        let score = document_matching::match_documents(&[doc1], &[doc2]);
        assert_eq!(
            score, 1.0,
            "Exact document match with same country should be 1.0"
        );
    }

    /// Differing document types never match, even with equal numbers.
    #[test]
    fn test_document_match_different_type() {
        let doc1 = IdentityDocument {
            document_type: crate::models::DocumentType::Passport,
            number: "X12345678".to_string(),
            issuing_country: Some("US".to_string()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: true,
        };
        let doc2 = IdentityDocument {
            document_type: crate::models::DocumentType::DriversLicense,
            number: "X12345678".to_string(),
            issuing_country: Some("US".to_string()),
            issuing_authority: None,
            issue_date: None,
            expiry_date: None,
            verified: true,
        };
        let score = document_matching::match_documents(&[doc1], &[doc2]);
        assert_eq!(score, 0.0, "Different document types should not match");
    }
}
