//! Field-level validation for incoming `Organization` payloads.
//!
//! The service stores the matcher's `Organization` verbatim (JSONB), so
//! payload validation is the *service's* responsibility — the matcher is a
//! pure scoring library and deliberately performs no validation. These
//! checks return human-readable problem strings that the controller
//! surfaces as a single `422 Unprocessable Entity`.
//!
//! ## Rules
//!
//! - **`name`** — required; must not be blank.
//! - **`identifiers[i].value`** — must not be blank.
//! - **Input-size caps (SEC-M1)** — every scalar text field, array
//!   cardinality, and per-entry string length is bounded, so a single huge
//!   string or huge array cannot be used as a CPU/memory `DoS` against the
//!   matcher's O(n·m) Jaro-Winkler / Levenshtein / Jaccard scoring
//!   (amplified across the `check-duplicates` scan). Oversized input is
//!   rejected with a `422` *before* the record is stored or matched.

use organization_matcher::Organization;

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single scalar text field (`name`, `legal_name`, `url`, `jurisdiction`,
/// `founding_date`, `telephone`, `email`, and each address sub-field).
/// Bounds the per-field cost of the matcher's character-level comparisons.
const MAX_TEXT_LEN: usize = 1024;

/// Maximum number of entries in any array field (`alternate_names`,
/// `identifiers`, `same_as`, `keywords`). Bounds the O(n·m) Jaccard /
/// overlap work the matcher does over arrays.
const MAX_ARRAY_LEN: usize = 256;

/// Maximum length, in Unicode scalar values (`.chars().count()`), of any
/// single string entry inside an array field.
const MAX_ITEM_LEN: usize = 512;

/// Collect every validation problem for `org`. An empty vector means the
/// payload is valid.
///
/// The controller joins these into one `422` response, so the operator
/// sees all problems at once rather than fixing them one round-trip at a
/// time.
#[must_use]
pub fn problems(org: &Organization) -> Vec<String> {
    let mut out = Vec::new();

    if org.name.trim().is_empty() {
        out.push("name is required".to_string());
    }

    // Input-size caps (SEC-M1): scalar text fields.
    check_text(&mut out, "name", &org.name);
    check_opt_text(&mut out, "legal_name", org.legal_name.as_ref());
    check_opt_text(&mut out, "url", org.url.as_ref());
    check_opt_text(&mut out, "jurisdiction", org.jurisdiction.as_ref());
    check_opt_text(&mut out, "founding_date", org.founding_date.as_ref());
    check_opt_text(&mut out, "telephone", org.telephone.as_ref());
    check_opt_text(&mut out, "email", org.email.as_ref());
    if let Some(addr) = &org.address {
        check_opt_text(
            &mut out,
            "address.street_address",
            addr.street_address.as_ref(),
        );
        check_opt_text(&mut out, "address.locality", addr.locality.as_ref());
        check_opt_text(&mut out, "address.region", addr.region.as_ref());
        check_opt_text(&mut out, "address.postal_code", addr.postal_code.as_ref());
        check_opt_text(&mut out, "address.country", addr.country.as_ref());
    }

    // Input-size caps (SEC-M1): array cardinality.
    check_array(&mut out, "alternate_names", org.alternate_names.len());
    check_array(&mut out, "identifiers", org.identifiers.len());
    check_array(&mut out, "same_as", org.same_as.len());
    check_array(&mut out, "keywords", org.keywords.len());

    // Per-entry blank checks (existing intent) + size caps (SEC-M1).
    for (i, ident) in org.identifiers.iter().enumerate() {
        if ident.value.trim().is_empty() {
            out.push(format!("identifiers[{i}]: value must not be blank"));
        }
        check_item(&mut out, "identifiers", i, &ident.value);
    }
    for (i, alt) in org.alternate_names.iter().enumerate() {
        check_item(&mut out, "alternate_names", i, alt);
    }
    for (i, url) in org.same_as.iter().enumerate() {
        check_item(&mut out, "same_as", i, url);
    }
    for (i, keyword) in org.keywords.iter().enumerate() {
        check_item(&mut out, "keywords", i, keyword);
    }

    out
}

/// Push a problem when a scalar text `field` exceeds [`MAX_TEXT_LEN`]
/// Unicode scalar values.
fn check_text(out: &mut Vec<String>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        out.push(format!("{field}: exceeds {MAX_TEXT_LEN} characters"));
    }
}

/// [`check_text`] for an optional field, a no-op when absent.
fn check_opt_text(out: &mut Vec<String>, field: &str, value: Option<&String>) {
    if let Some(v) = value {
        check_text(out, field, v);
    }
}

/// Push a problem when an array `field` holds more than [`MAX_ARRAY_LEN`]
/// entries.
fn check_array(out: &mut Vec<String>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        out.push(format!("{field}: exceeds {MAX_ARRAY_LEN} entries"));
    }
}

/// Push a problem when the `index`-th entry of an array `field` exceeds
/// [`MAX_ITEM_LEN`] Unicode scalar values.
fn check_item(out: &mut Vec<String>, field: &str, index: usize, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        out.push(format!(
            "{field}[{index}]: exceeds {MAX_ITEM_LEN} characters"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use organization_matcher::{IdentifierScheme, OrgIdentifier, PostalAddress};

    /// Build a `Wikidata`-scheme `OrgIdentifier` with the given value.
    fn ident(value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme: IdentifierScheme::Wikidata,
            value: value.to_string(),
        }
    }

    /// A fully populated, well-formed organization yields zero problems.
    #[test]
    fn valid_organization_has_no_problems() {
        let org = Organization {
            legal_name: Some("Acme, Inc.".into()),
            alternate_names: vec!["Acme".into()],
            identifiers: vec![ident("Q4547858")],
            keywords: vec!["manufacturing".into()],
            ..Organization::new("Acme Corporation")
        };
        assert!(problems(&org).is_empty());
    }

    /// A whitespace-only name is the single `"name is required"` problem.
    #[test]
    fn blank_name_is_a_problem() {
        assert_eq!(
            problems(&Organization::new("   ")),
            vec!["name is required".to_string()]
        );
    }

    /// A blank identifier value is one problem, reported with its index.
    #[test]
    fn blank_identifier_value_is_a_problem() {
        let org = Organization {
            identifiers: vec![ident("   ")],
            ..Organization::new("Acme")
        };
        let p = problems(&org);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("identifiers[0]"));
    }

    /// SEC-M1: an oversized scalar text field is exactly one problem.
    #[test]
    fn oversized_text_field_is_a_problem() {
        let org = Organization::new("x".repeat(MAX_TEXT_LEN + 1));
        assert_eq!(
            problems(&org),
            vec![format!("name: exceeds {MAX_TEXT_LEN} characters")]
        );
    }

    /// SEC-M1: an oversized optional/nested address field is one problem.
    #[test]
    fn oversized_address_field_is_a_problem() {
        let org = Organization {
            address: Some(PostalAddress {
                locality: Some("x".repeat(MAX_TEXT_LEN + 1)),
                ..PostalAddress::default()
            }),
            ..Organization::new("Acme")
        };
        assert_eq!(
            problems(&org),
            vec![format!(
                "address.locality: exceeds {MAX_TEXT_LEN} characters"
            )]
        );
    }

    /// SEC-M1: an over-long array is exactly one problem (entries within cap).
    #[test]
    fn oversized_array_is_a_problem() {
        let org = Organization {
            keywords: vec!["ok".to_string(); MAX_ARRAY_LEN + 1],
            ..Organization::new("Acme")
        };
        assert_eq!(
            problems(&org),
            vec![format!("keywords: exceeds {MAX_ARRAY_LEN} entries")]
        );
    }

    /// SEC-M1: an oversized single entry inside an array is one problem,
    /// reported with its index.
    #[test]
    fn oversized_array_item_is_a_problem() {
        let org = Organization {
            keywords: vec!["ok".into(), "x".repeat(MAX_ITEM_LEN + 1)],
            ..Organization::new("Acme")
        };
        assert_eq!(
            problems(&org),
            vec![format!("keywords[1]: exceeds {MAX_ITEM_LEN} characters")]
        );
    }

    /// SEC-M1: a large-but-within-caps record (every field at exactly its
    /// limit) is accepted — the caps reject only what exceeds them.
    #[test]
    fn within_caps_large_record_has_no_problems() {
        let org = Organization {
            name: "x".repeat(MAX_TEXT_LEN),
            legal_name: Some("l".repeat(MAX_TEXT_LEN)),
            alternate_names: vec!["a".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            identifiers: vec![ident(&"i".repeat(MAX_ITEM_LEN)); MAX_ARRAY_LEN],
            same_as: vec!["s".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            keywords: vec!["k".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN],
            ..Organization::new("placeholder")
        };
        assert!(problems(&org).is_empty());
    }

    /// The report-everything contract: multiple problems are collected in
    /// one pass.
    #[test]
    fn problems_reports_every_issue_with_index() {
        let org = Organization {
            identifiers: vec![ident("ok"), ident(" ")], // [1] blank
            ..Organization::new("")                     // blank name
        };
        let p = problems(&org);
        assert_eq!(p.len(), 2);
        assert!(p.iter().any(|m| m.contains("name is required")));
        assert!(p.iter().any(|m| m.contains("identifiers[1]")));
    }
}
