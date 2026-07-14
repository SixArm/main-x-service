//! Data-quality validation and normalization for
//! [`Thing`](crate::models::thing::Thing) records.
//!
//! Two entry points cover the create/update boundary:
//!
//! - [`validate_thing`](crate::validation::validate_thing) collects *all*
//!   problems into a `Vec<ValidationError>` (an empty vec means valid). It
//!   never short-circuits on the first error, so a caller can surface every
//!   field problem at once (HTTP `422`).
//! - [`normalize_thing`](crate::validation::normalize_thing) mutates a record
//!   into canonical form: trims text,
//!   lowercases URL schemes (preserving host/path), and dedupes repeatable
//!   lists.
//!
//! Validation is intentionally lenient on punctuation: ISBN/ISSN/GTIN
//! checks strip dashes and whitespace before counting digits, and only
//! deterministic schemes with a well-known format (ISBN, ISSN, DOI, GTIN,
//! UUID, URI) are format-checked. SKU/MPN/SerialNumber/Custom values are
//! accepted as-is once non-empty.
//!
//! # Examples
//!
//! ```
//! use thing_service::models::thing::Thing;
//! use thing_service::validation::{validate_thing, normalize_thing};
//!
//! let mut thing = Thing::new("  Pride and Prejudice  ");
//! assert!(validate_thing(&thing).is_empty()); // a name is all that's required
//!
//! normalize_thing(&mut thing);
//! assert_eq!(thing.name, "Pride and Prejudice"); // trimmed in place
//! ```

use crate::models::identifier::{IdentifierType, ThingIdentifier};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::models::thing::Thing;

/// A single field-level validation failure.
///
/// [`validate_thing`] returns a `Vec` of these; `field` names the offending
/// field (with an index for list entries, e.g. `"same_as[0]"`) and `message`
/// is a human-readable explanation suitable for an API error body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ValidationError {
    /// The offending field path (e.g. `"name"`, `"identifiers[2].value"`).
    pub field: String,
    /// Human-readable description of what is wrong.
    pub message: String,
}

/// Validate a Thing, returning all errors. An empty Vec means valid.
#[must_use]
pub fn validate_thing(thing: &Thing) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if thing.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "name is required and must not be empty".into(),
        });
    }

    check_optional_http_url(thing.url.as_ref(), "url", &mut errors);
    check_optional_http_url(
        thing.additional_type.as_ref(),
        "additional_type",
        &mut errors,
    );
    check_optional_http_url(
        thing.main_entity_of_page.as_ref(),
        "main_entity_of_page",
        &mut errors,
    );
    check_optional_http_url(thing.subject_of.as_ref(), "subject_of", &mut errors);

    for (i, img) in thing.images.iter().enumerate() {
        if !is_http_url(img) {
            errors.push(ValidationError {
                field: format!("images[{i}]"),
                message: "image must be an http(s) URL".into(),
            });
        }
    }

    for (i, url) in thing.same_as.iter().enumerate() {
        if !is_http_url(url) {
            errors.push(ValidationError {
                field: format!("same_as[{i}]"),
                message: "same_as entry must be an http(s) URL".into(),
            });
        }
    }

    for (i, name) in thing.alternate_names.iter().enumerate() {
        if name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("alternate_names[{i}]"),
                message: "alternate name must not be empty".into(),
            });
        }
    }

    for (i, id) in thing.identifiers.iter().enumerate() {
        if id.value.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("identifiers[{i}].value"),
                message: "identifier value must not be empty".into(),
            });
            continue;
        }
        if let Err(msg) = validate_identifier(id) {
            errors.push(ValidationError {
                field: format!("identifiers[{i}]"),
                message: msg,
            });
        }
    }

    // SEC-M1 input-size caps (additive; no field here has a stricter bound).
    thing_size_caps(&mut errors, thing);

    errors
}

/// Normalize a Thing in place: trim text fields, normalize URL schemes,
/// dedupe repeatable lists.
pub fn normalize_thing(thing: &mut Thing) {
    thing.name = thing.name.trim().to_string();
    if let Some(d) = &mut thing.description {
        *d = d.trim().to_string();
    }
    if let Some(d) = &mut thing.disambiguating_description {
        *d = d.trim().to_string();
    }
    if let Some(u) = &mut thing.url {
        *u = normalize_url(u);
    }
    if let Some(u) = &mut thing.additional_type {
        *u = normalize_url(u);
    }
    if let Some(u) = &mut thing.main_entity_of_page {
        *u = normalize_url(u);
    }
    if let Some(u) = &mut thing.subject_of {
        *u = normalize_url(u);
    }
    thing.alternate_names = dedupe(thing.alternate_names.iter().map(|s| s.trim().to_string()));
    thing.same_as = dedupe(thing.same_as.iter().map(|s| normalize_url(s)));
    thing.images = dedupe(thing.images.iter().map(|s| normalize_url(s)));
}

/// SEC-M1 — maximum length, in Unicode scalar values (`.chars().count()`),
/// of any single scalar text field. Bounds the per-field cost of the
/// matcher's character-level string comparisons (Jaro-Winkler / Soundex on
/// name / description / URLs) so one huge string cannot be a CPU/memory
/// `DoS`, amplified across the `check-duplicates` / `deduplicate` scans.
const MAX_TEXT_LEN: usize = 1024;
/// SEC-M1 — maximum number of entries in any array field. Bounds the O(n·m)
/// best-pair / list work the matcher does over `same_as` / `images` /
/// `identifiers`.
const MAX_ARRAY_LEN: usize = 256;
/// SEC-M1 — maximum length of any single string entry inside an array.
const MAX_ITEM_LEN: usize = 512;

/// SEC-M1: push an error when a scalar text `field` exceeds [`MAX_TEXT_LEN`]
/// Unicode scalar values.
fn cap_text(errors: &mut Vec<ValidationError>, field: &str, value: &str) {
    if value.chars().count() > MAX_TEXT_LEN {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("must not exceed {MAX_TEXT_LEN} characters"),
        });
    }
}

/// SEC-M1: [`cap_text`] for an optional field; a no-op when absent.
fn cap_opt_text(errors: &mut Vec<ValidationError>, field: &str, value: Option<&String>) {
    if let Some(v) = value {
        cap_text(errors, field, v);
    }
}

/// SEC-M1: push an error when an array `field` holds more than
/// [`MAX_ARRAY_LEN`] entries.
fn cap_array(errors: &mut Vec<ValidationError>, field: &str, len: usize) {
    if len > MAX_ARRAY_LEN {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("must not exceed {MAX_ARRAY_LEN} entries"),
        });
    }
}

/// SEC-M1: push an error when the string at field path `field` exceeds
/// [`MAX_ITEM_LEN`] Unicode scalar values.
fn cap_named_item(errors: &mut Vec<ValidationError>, field: &str, value: &str) {
    if value.chars().count() > MAX_ITEM_LEN {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("must not exceed {MAX_ITEM_LEN} characters"),
        });
    }
}

/// SEC-M1: cap the cardinality of a string array and the length of each
/// entry (`field[index]` on overflow) — the common shape for the `Thing`
/// list fields (`alternate_names`, `images`, `same_as`).
fn cap_string_array(errors: &mut Vec<ValidationError>, field: &str, values: &[String]) {
    cap_array(errors, field, values.len());
    for (i, v) in values.iter().enumerate() {
        cap_named_item(errors, &format!("{field}[{i}]"), v);
    }
}

/// SEC-M1: cap the identifier array cardinality and each identifier's inner
/// text (`value` / optional `name` / `url`) at [`MAX_ITEM_LEN`]. The
/// `value` emptiness check is a separate lower bound in [`validate_thing`];
/// this only adds the upper bound.
fn cap_identifiers(errors: &mut Vec<ValidationError>, identifiers: &[ThingIdentifier]) {
    cap_array(errors, "identifiers", identifiers.len());
    for (i, id) in identifiers.iter().enumerate() {
        cap_named_item(errors, &format!("identifiers[{i}].value"), &id.value);
        if let Some(name) = &id.name {
            cap_named_item(errors, &format!("identifiers[{i}].name"), name);
        }
        if let Some(url) = &id.url {
            cap_named_item(errors, &format!("identifiers[{i}].url"), url);
        }
    }
}

/// SEC-M1: apply the input-size caps to a [`Thing`]'s scalar text fields,
/// string arrays, and identifier array. Factored out of [`validate_thing`]
/// to keep that function within the clippy line budget. No field here has a
/// pre-existing stricter bound, so these caps are purely additive.
fn thing_size_caps(errors: &mut Vec<ValidationError>, thing: &Thing) {
    cap_text(errors, "name", &thing.name);
    cap_opt_text(errors, "description", thing.description.as_ref());
    cap_opt_text(
        errors,
        "disambiguating_description",
        thing.disambiguating_description.as_ref(),
    );
    cap_opt_text(errors, "additional_type", thing.additional_type.as_ref());
    cap_opt_text(errors, "url", thing.url.as_ref());
    cap_opt_text(
        errors,
        "main_entity_of_page",
        thing.main_entity_of_page.as_ref(),
    );
    cap_opt_text(errors, "owner", thing.owner.as_ref());
    cap_opt_text(errors, "subject_of", thing.subject_of.as_ref());
    cap_opt_text(errors, "potential_action", thing.potential_action.as_ref());

    cap_string_array(errors, "alternate_names", &thing.alternate_names);
    cap_string_array(errors, "images", &thing.images);
    cap_string_array(errors, "same_as", &thing.same_as);

    cap_identifiers(errors, &thing.identifiers);
}

/// Push a `must be an http(s) URL` error for `field` when an optional URL is
/// present but not an http(s) URL. A `None` value is valid (the field is
/// optional) and produces no error.
fn check_optional_http_url(value: Option<&String>, field: &str, errors: &mut Vec<ValidationError>) {
    if let Some(v) = value
        && !is_http_url(v)
    {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("{field} must be an http(s) URL"),
        });
    }
}

/// True if `s` (trimmed, case-insensitive) begins with `http://` or
/// `https://`. The only scheme test the validator applies to URL fields.
fn is_http_url(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    t.starts_with("http://") || t.starts_with("https://")
}

/// Canonicalize a URL by trimming and lowercasing *only* the scheme,
/// leaving host and path untouched (URLs can be path-case-sensitive).
/// Strings without a `://` separator are returned trimmed but otherwise
/// unchanged.
fn normalize_url(s: &str) -> String {
    let t = s.trim();
    // Split at the scheme separator and lowercase the left half only.
    let lower_scheme = t
        .find("://")
        .map(|i| (&t[..i], &t[i..]))
        .map(|(scheme, rest)| format!("{}{rest}", scheme.to_lowercase()));
    lower_scheme.unwrap_or_else(|| t.to_string())
}

/// Collect an iterator into a `Vec`, dropping empty strings and preserving
/// first-seen order while removing later duplicates (a stable dedupe).
fn dedupe(iter: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    // `insert` returns false for an already-seen value, filtering duplicates.
    iter.filter(|s| !s.is_empty() && seen.insert(s.clone()))
        .collect()
}

/// Dispatch per-scheme format validation. Deterministic schemes with a
/// well-known shape are checked; all other schemes (SKU, MPN,
/// `SerialNumber`, Custom) are accepted unconditionally.
fn validate_identifier(id: &ThingIdentifier) -> Result<(), String> {
    match &id.property_id {
        IdentifierType::Isbn => validate_isbn(&id.value),
        IdentifierType::Issn => validate_issn(&id.value),
        IdentifierType::Doi => validate_doi(&id.value),
        IdentifierType::Gtin => validate_gtin(&id.value),
        IdentifierType::Uuid => validate_uuid(&id.value),
        IdentifierType::Uri => {
            if id.value.contains(':') {
                Ok(())
            } else {
                Err("URI must contain a scheme separator (':')".into())
            }
        }
        _ => Ok(()),
    }
}

/// Validate an ISBN: 10 or 13 digits after stripping dashes/whitespace. An
/// ISBN-10 may end in the check character `X`; ISBN-13 must be all digits.
fn validate_isbn(v: &str) -> Result<(), String> {
    // Strip the cosmetic separators humans use; count only payload chars.
    let digits: String = v
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    let n = digits.len();
    if n != 10 && n != 13 {
        return Err(format!("ISBN must have 10 or 13 digits, got {n}"));
    }
    let valid = if n == 10 {
        digits.chars().take(9).all(|c| c.is_ascii_digit())
            && digits
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii_digit() || c == 'X')
    } else {
        digits.chars().all(|c| c.is_ascii_digit())
    };
    if !valid {
        return Err("ISBN contains invalid characters".into());
    }
    Ok(())
}

/// Validate an ISSN: exactly 8 characters after stripping dashes/whitespace,
/// the last of which may be the check character `X`.
fn validate_issn(v: &str) -> Result<(), String> {
    let s: String = v
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .collect();
    if s.len() != 8 {
        return Err(format!("ISSN must be 8 digits, got {}", s.len()));
    }
    let head_ok = s.chars().take(7).all(|c| c.is_ascii_digit());
    let tail_ok = s
        .chars()
        .last()
        .is_some_and(|c| c.is_ascii_digit() || c == 'X');
    if !head_ok || !tail_ok {
        return Err("ISSN contains invalid characters".into());
    }
    Ok(())
}

/// Validate a DOI: must begin with the `10.` registrant prefix and contain
/// a `/` separating prefix from suffix. The suffix itself is unconstrained.
fn validate_doi(v: &str) -> Result<(), String> {
    if !v.starts_with("10.") || !v.contains('/') {
        return Err("DOI must start with '10.' and contain a '/'".into());
    }
    Ok(())
}

/// Validate a GTIN: 8, 12, 13, or 14 digits (GTIN-8/UPC/EAN/GTIN-14) after
/// dropping any non-digit characters. The check digit is not verified.
fn validate_gtin(v: &str) -> Result<(), String> {
    let digits: String = v.chars().filter(char::is_ascii_digit).collect();
    if !matches!(digits.len(), 8 | 12 | 13 | 14) {
        return Err(format!(
            "GTIN must be 8, 12, 13, or 14 digits, got {}",
            digits.len()
        ));
    }
    Ok(())
}

/// Validate a UUID by delegating to [`uuid::Uuid::parse_str`] (RFC 4122).
fn validate_uuid(v: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(v)
        .map(|_| ())
        .map_err(|e| format!("invalid UUID: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::ThingIdentifier;

    /// A thing with just a name is valid (name is the only requirement).
    #[test]
    fn test_valid_thing() {
        let thing = Thing::new("Pride and Prejudice");
        assert!(validate_thing(&thing).is_empty());
    }

    /// An empty name yields exactly one `name` error.
    #[test]
    fn test_empty_name() {
        let thing = Thing::new("");
        let errors = validate_thing(&thing);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    /// A whitespace-only name is treated as empty (one `name` error).
    #[test]
    fn test_whitespace_name() {
        let thing = Thing::new("   ");
        let errors = validate_thing(&thing);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    /// A well-formed https `url` passes validation.
    #[test]
    fn test_valid_url() {
        let mut thing = Thing::new("X");
        thing.url = Some("https://example.com".into());
        assert!(validate_thing(&thing).is_empty());
    }

    /// A non-http `url` produces a `url` error.
    #[test]
    fn test_invalid_url() {
        let mut thing = Thing::new("X");
        thing.url = Some("not-a-url".into());
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "url"));
    }

    /// A schema.org subtype URL is a valid `additional_type`.
    #[test]
    fn test_valid_additional_type() {
        let mut thing = Thing::new("X");
        thing.additional_type = Some("https://schema.org/Book".into());
        assert!(validate_thing(&thing).is_empty());
    }

    /// A bare word (not a URL) produces an `additional_type` error.
    #[test]
    fn test_invalid_additional_type() {
        let mut thing = Thing::new("X");
        thing.additional_type = Some("Book".into());
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "additional_type"));
    }

    /// A `data:` image URL is rejected (only http(s) is allowed).
    #[test]
    fn test_invalid_image_url() {
        let mut thing = Thing::new("X");
        thing.images = vec!["data:image/png;base64,AAAA".into()];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "images[0]"));
    }

    /// A non-http `same_as` entry produces a `same_as[0]` error.
    #[test]
    fn test_invalid_same_as_url() {
        let mut thing = Thing::new("X");
        thing.same_as = vec!["wikidata://Q42".into()];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "same_as[0]"));
    }

    /// A dashed ISBN-10 is valid (dashes are stripped before counting).
    #[test]
    fn test_valid_isbn_10() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::isbn("0-141-43951-9")];
        assert!(validate_thing(&thing).is_empty());
    }

    /// A 13-digit ISBN is valid.
    #[test]
    fn test_valid_isbn_13() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        assert!(validate_thing(&thing).is_empty());
    }

    /// An ISBN with the wrong digit count is rejected.
    #[test]
    fn test_invalid_isbn_length() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::isbn("123")];
        assert!(!validate_thing(&thing).is_empty());
    }

    /// A well-formed DOI (`10.…/…`) is valid.
    #[test]
    fn test_valid_doi() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::doi("10.1000/xyz123")];
        assert!(validate_thing(&thing).is_empty());
    }

    /// A DOI not starting with `10.` is rejected.
    #[test]
    fn test_invalid_doi() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::doi("99.1000/xyz123")];
        assert!(!validate_thing(&thing).is_empty());
    }

    /// A 13-digit GTIN is valid.
    #[test]
    fn test_valid_gtin_13() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::gtin("0012345600012")];
        assert!(validate_thing(&thing).is_empty());
    }

    /// A GTIN with the wrong digit count is rejected.
    #[test]
    fn test_invalid_gtin_length() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::gtin("1234")];
        assert!(!validate_thing(&thing).is_empty());
    }

    /// A canonical RFC 4122 UUID is valid.
    #[test]
    fn test_valid_uuid() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::uuid(
            "550e8400-e29b-41d4-a716-446655440000",
        )];
        assert!(validate_thing(&thing).is_empty());
    }

    /// An unparseable UUID string is rejected.
    #[test]
    fn test_invalid_uuid() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::uuid("not-a-uuid")];
        assert!(!validate_thing(&thing).is_empty());
    }

    /// An identifier with an empty value produces a `.value` error.
    #[test]
    fn test_identifier_empty_value() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::sku("")];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "identifiers[0].value"));
    }

    /// An empty alternate name produces an `alternate_names[0]` error.
    #[test]
    fn test_alternate_name_empty() {
        let mut thing = Thing::new("X");
        thing.alternate_names = vec![String::new()];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "alternate_names[0]"));
    }

    /// Multiple problems are all reported (validation does not short-circuit).
    #[test]
    fn test_multiple_validation_errors() {
        let mut thing = Thing::new("");
        thing.url = Some("bad-url".into());
        thing.additional_type = Some("Book".into());
        let errors = validate_thing(&thing);
        assert!(errors.len() >= 3, "Expected 3+ errors, got: {errors:?}");
    }

    /// Normalization trims leading/trailing whitespace from the name.
    #[test]
    fn test_normalize_trims_name() {
        let mut thing = Thing::new("  Pride and Prejudice  ");
        normalize_thing(&mut thing);
        assert_eq!(thing.name, "Pride and Prejudice");
    }

    /// Normalization lowercases the URL scheme but preserves host/path case.
    #[test]
    fn test_normalize_url_scheme_lowercase() {
        let mut thing = Thing::new("X");
        thing.url = Some("HTTPS://EXAMPLE.com/Path".into());
        normalize_thing(&mut thing);
        // Scheme is lowered; host/path preserved
        assert_eq!(thing.url.as_deref(), Some("https://EXAMPLE.com/Path"));
    }

    /// Normalization removes duplicate `same_as` entries.
    #[test]
    fn test_normalize_dedupes_same_as() {
        let mut thing = Thing::new("X");
        thing.same_as = vec![
            "https://example.com".into(),
            "https://example.com".into(),
            "https://other.com".into(),
        ];
        normalize_thing(&mut thing);
        assert_eq!(thing.same_as.len(), 2);
    }

    /// Normalization removes duplicate alternate names, preserving order.
    #[test]
    fn test_normalize_dedupes_alternate_names() {
        let mut thing = Thing::new("X");
        thing.alternate_names = vec!["A".into(), "A".into(), "B".into()];
        normalize_thing(&mut thing);
        assert_eq!(
            thing.alternate_names,
            vec!["A".to_string(), "B".to_string()]
        );
    }

    /// SEC-M1: an oversized scalar text field (`name`) yields a cap error.
    #[test]
    fn test_cap_oversized_scalar_text() {
        let thing = Thing::new(&"x".repeat(MAX_TEXT_LEN + 1));
        let errors = validate_thing(&thing);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "name" && e.message.contains("must not exceed")),
            "expected a name cap error, got: {errors:?}"
        );
    }

    /// SEC-M1: an over-long array (`same_as`) yields a cardinality error.
    #[test]
    fn test_cap_over_long_array() {
        let mut thing = Thing::new("X");
        // Valid http(s) URLs so only the cardinality cap fires.
        thing.same_as = vec!["https://example.com".to_string(); MAX_ARRAY_LEN + 1];
        let errors = validate_thing(&thing);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "same_as" && e.message.contains("must not exceed")),
            "expected a same_as cardinality error, got: {} errors",
            errors.len()
        );
    }

    /// SEC-M1: an oversized array entry yields an indexed cap error.
    #[test]
    fn test_cap_oversized_array_entry() {
        let mut thing = Thing::new("X");
        thing.alternate_names = vec!["ok".into(), "x".repeat(MAX_ITEM_LEN + 1)];
        let errors = validate_thing(&thing);
        assert!(
            errors
                .iter()
                .any(|e| e.field == "alternate_names[1]" && e.message.contains("must not exceed")),
            "expected an alternate_names[1] cap error, got: {errors:?}"
        );
    }

    /// SEC-M1: a record filled right up to the caps produces no cap errors.
    #[test]
    fn test_within_caps_large_record_ok() {
        let mut thing = Thing::new(&"x".repeat(MAX_TEXT_LEN));
        thing.alternate_names = vec!["a".repeat(MAX_ITEM_LEN); MAX_ARRAY_LEN];
        thing.same_as = vec!["https://example.com".to_string(); MAX_ARRAY_LEN];
        thing.images = vec!["https://example.com/i.png".to_string(); MAX_ARRAY_LEN];
        // SKU format is unconstrained; value within the per-item cap.
        thing.identifiers = vec![ThingIdentifier::sku(&"s".repeat(MAX_ITEM_LEN)); MAX_ARRAY_LEN];
        let errors = validate_thing(&thing);
        assert!(
            !errors.iter().any(|e| e.message.contains("must not exceed")),
            "expected no cap errors, got: {errors:?}"
        );
    }
}
