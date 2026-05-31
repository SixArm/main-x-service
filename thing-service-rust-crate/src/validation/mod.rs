use crate::models::identifier::{IdentifierType, ThingIdentifier};
use crate::models::thing::Thing;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

/// Validate a Thing, returning all errors. An empty Vec means valid.
pub fn validate_thing(thing: &Thing) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if thing.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "name is required and must not be empty".into(),
        });
    }

    check_optional_http_url(&thing.url, "url", &mut errors);
    check_optional_http_url(&thing.additional_type, "additional_type", &mut errors);
    check_optional_http_url(&thing.main_entity_of_page, "main_entity_of_page", &mut errors);
    check_optional_http_url(&thing.subject_of, "subject_of", &mut errors);

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

fn check_optional_http_url(value: &Option<String>, field: &str, errors: &mut Vec<ValidationError>) {
    if let Some(v) = value
        && !is_http_url(v)
    {
        errors.push(ValidationError {
            field: field.to_string(),
            message: format!("{field} must be an http(s) URL"),
        });
    }
}

fn is_http_url(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    t.starts_with("http://") || t.starts_with("https://")
}

fn normalize_url(s: &str) -> String {
    let t = s.trim();
    let lower_scheme = t
        .find("://")
        .map(|i| (&t[..i], &t[i..]))
        .map(|(scheme, rest)| format!("{}{rest}", scheme.to_lowercase()));
    lower_scheme.unwrap_or_else(|| t.to_string())
}

fn dedupe(iter: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    iter.filter(|s| !s.is_empty() && seen.insert(s.clone())).collect()
}

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

fn validate_isbn(v: &str) -> Result<(), String> {
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
                .map(|c| c.is_ascii_digit() || c == 'X')
                .unwrap_or(false)
    } else {
        digits.chars().all(|c| c.is_ascii_digit())
    };
    if !valid {
        return Err("ISBN contains invalid characters".into());
    }
    Ok(())
}

fn validate_issn(v: &str) -> Result<(), String> {
    let s: String = v.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    if s.len() != 8 {
        return Err(format!("ISSN must be 8 digits, got {}", s.len()));
    }
    let head_ok = s.chars().take(7).all(|c| c.is_ascii_digit());
    let tail_ok = s
        .chars()
        .last()
        .map(|c| c.is_ascii_digit() || c == 'X')
        .unwrap_or(false);
    if !head_ok || !tail_ok {
        return Err("ISSN contains invalid characters".into());
    }
    Ok(())
}

fn validate_doi(v: &str) -> Result<(), String> {
    if !v.starts_with("10.") || !v.contains('/') {
        return Err("DOI must start with '10.' and contain a '/'".into());
    }
    Ok(())
}

fn validate_gtin(v: &str) -> Result<(), String> {
    let digits: String = v.chars().filter(|c| c.is_ascii_digit()).collect();
    if !matches!(digits.len(), 8 | 12 | 13 | 14) {
        return Err(format!(
            "GTIN must be 8, 12, 13, or 14 digits, got {}",
            digits.len()
        ));
    }
    Ok(())
}

fn validate_uuid(v: &str) -> Result<(), String> {
    uuid::Uuid::parse_str(v)
        .map(|_| ())
        .map_err(|e| format!("invalid UUID: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::ThingIdentifier;

    #[test]
    fn test_valid_thing() {
        let thing = Thing::new("Pride and Prejudice");
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_empty_name() {
        let thing = Thing::new("");
        let errors = validate_thing(&thing);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[test]
    fn test_whitespace_name() {
        let thing = Thing::new("   ");
        let errors = validate_thing(&thing);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "name");
    }

    #[test]
    fn test_valid_url() {
        let mut thing = Thing::new("X");
        thing.url = Some("https://example.com".into());
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_invalid_url() {
        let mut thing = Thing::new("X");
        thing.url = Some("not-a-url".into());
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "url"));
    }

    #[test]
    fn test_valid_additional_type() {
        let mut thing = Thing::new("X");
        thing.additional_type = Some("https://schema.org/Book".into());
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_invalid_additional_type() {
        let mut thing = Thing::new("X");
        thing.additional_type = Some("Book".into());
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "additional_type"));
    }

    #[test]
    fn test_invalid_image_url() {
        let mut thing = Thing::new("X");
        thing.images = vec!["data:image/png;base64,AAAA".into()];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "images[0]"));
    }

    #[test]
    fn test_invalid_same_as_url() {
        let mut thing = Thing::new("X");
        thing.same_as = vec!["wikidata://Q42".into()];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "same_as[0]"));
    }

    #[test]
    fn test_valid_isbn_10() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::isbn("0-141-43951-9")];
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_valid_isbn_13() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_invalid_isbn_length() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::isbn("123")];
        assert!(!validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_valid_doi() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::doi("10.1000/xyz123")];
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_invalid_doi() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::doi("99.1000/xyz123")];
        assert!(!validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_valid_gtin_13() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::gtin("0012345600012")];
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_invalid_gtin_length() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::gtin("1234")];
        assert!(!validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_valid_uuid() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::uuid("550e8400-e29b-41d4-a716-446655440000")];
        assert!(validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_invalid_uuid() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::uuid("not-a-uuid")];
        assert!(!validate_thing(&thing).is_empty());
    }

    #[test]
    fn test_identifier_empty_value() {
        let mut thing = Thing::new("X");
        thing.identifiers = vec![ThingIdentifier::sku("")];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "identifiers[0].value"));
    }

    #[test]
    fn test_alternate_name_empty() {
        let mut thing = Thing::new("X");
        thing.alternate_names = vec!["".into()];
        let errors = validate_thing(&thing);
        assert!(errors.iter().any(|e| e.field == "alternate_names[0]"));
    }

    #[test]
    fn test_multiple_validation_errors() {
        let mut thing = Thing::new("");
        thing.url = Some("bad-url".into());
        thing.additional_type = Some("Book".into());
        let errors = validate_thing(&thing);
        assert!(errors.len() >= 3, "Expected 3+ errors, got: {errors:?}");
    }

    #[test]
    fn test_normalize_trims_name() {
        let mut thing = Thing::new("  Pride and Prejudice  ");
        normalize_thing(&mut thing);
        assert_eq!(thing.name, "Pride and Prejudice");
    }

    #[test]
    fn test_normalize_url_scheme_lowercase() {
        let mut thing = Thing::new("X");
        thing.url = Some("HTTPS://EXAMPLE.com/Path".into());
        normalize_thing(&mut thing);
        // Scheme is lowered; host/path preserved
        assert_eq!(thing.url.as_deref(), Some("https://EXAMPLE.com/Path"));
    }

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

    #[test]
    fn test_normalize_dedupes_alternate_names() {
        let mut thing = Thing::new("X");
        thing.alternate_names = vec!["A".into(), "A".into(), "B".into()];
        normalize_thing(&mut thing);
        assert_eq!(thing.alternate_names, vec!["A".to_string(), "B".to_string()]);
    }
}
