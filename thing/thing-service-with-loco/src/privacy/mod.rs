//! Privacy controls for [`Thing`](crate::models::thing::Thing) records.
//!
//! Two operations support data-protection regimes:
//!
//! - [`mask_thing`](crate::privacy::mask_thing) returns a copy with sensitive
//!   fields obscured, for privacy-conscious display (e.g. the masked-view
//!   endpoint). Most schema.org/Thing properties are public bibliographic
//!   data, so only `owner` and identifier values/URLs are touched.
//! - [`gdpr_export`](crate::privacy::gdpr_export) returns the record's full
//!   JSON, unmodified, for a
//!   GDPR Article 15 data-portability request.
//!
//! # Examples
//!
//! ```
//! use thing_service::models::identifier::ThingIdentifier;
//! use thing_service::models::thing::Thing;
//! use thing_service::privacy::mask_thing;
//!
//! let mut thing = Thing::new("Private Diary");
//! thing.owner = Some("Jane Doe".into());
//! thing.identifiers = vec![ThingIdentifier::serial_number("SN-1234567890")];
//!
//! let masked = mask_thing(&thing);
//! assert_eq!(masked.owner.as_deref(), Some("[owner withheld]"));
//! assert!(masked.identifiers[0].value.starts_with("****"));
//! ```

use crate::models::thing::Thing;
use serde_json::Value;

/// Mask sensitive fields in a Thing for privacy-conscious display.
///
/// Most schema.org/Thing properties are public bibliographic data. The
/// fields considered sensitive here are:
///
/// - `owner` — may identify a person; replaced with `"[owner withheld]"`
/// - `identifiers` — each `value` is truncated to its last four
///   characters; any per-identifier `url` is cleared. The
///   `property_id` is preserved.
#[must_use]
pub fn mask_thing(thing: &Thing) -> Thing {
    let mut masked = thing.clone();
    if masked.owner.is_some() {
        masked.owner = Some("[owner withheld]".to_string());
    }
    for id in &mut masked.identifiers {
        id.value = mask_value(&id.value);
        id.url = None;
    }
    masked
}

/// Mask an identifier value, keeping only its last four characters behind a
/// `****` prefix. Values of four characters or fewer are fully redacted to
/// `"****"` so nothing identifying leaks from short strings.
fn mask_value(v: &str) -> String {
    if v.len() <= 4 {
        return "****".to_string();
    }
    let tail = &v[v.len() - 4..];
    format!("****{tail}")
}

/// GDPR Article 15 export — full Thing JSON, unmodified.
#[must_use]
pub fn gdpr_export(thing: &Thing) -> Value {
    serde_json::to_value(thing).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::ThingIdentifier;

    /// A present `owner` is replaced with the withheld placeholder.
    #[test]
    fn test_mask_owner() {
        let mut thing = Thing::new("Private Diary");
        thing.owner = Some("Jane Doe".into());
        let masked = mask_thing(&thing);
        assert_eq!(masked.owner.as_deref(), Some("[owner withheld]"));
    }

    /// An identifier value keeps only its last four characters.
    #[test]
    fn test_mask_identifier_value() {
        let mut thing = Thing::new("Test");
        thing.identifiers = vec![ThingIdentifier::serial_number("ABCD1234567890")];
        let masked = mask_thing(&thing);
        let v = &masked.identifiers[0].value;
        assert!(v.starts_with("****"));
        assert!(v.ends_with("7890"));
    }

    /// A per-identifier `url` is cleared during masking.
    #[test]
    fn test_mask_identifier_url_cleared() {
        let mut thing = Thing::new("Test");
        let mut id = ThingIdentifier::isbn("9780141439518");
        id.url = Some("https://worldcat.org/isbn/9780141439518".into());
        thing.identifiers = vec![id];
        let masked = mask_thing(&thing);
        assert!(masked.identifiers[0].url.is_none());
    }

    /// A short (≤4 char) identifier value is fully redacted to `****`.
    #[test]
    fn test_mask_short_identifier() {
        let mut thing = Thing::new("Test");
        thing.identifiers = vec![ThingIdentifier::sku("AB")];
        let masked = mask_thing(&thing);
        assert_eq!(masked.identifiers[0].value, "****");
    }

    /// The public `name` is left untouched by masking.
    #[test]
    fn test_mask_preserves_name() {
        let thing = Thing::new("Pride and Prejudice");
        let masked = mask_thing(&thing);
        assert_eq!(masked.name, "Pride and Prejudice");
    }

    /// Masking preserves each identifier's `property_id` (only the value/url change).
    #[test]
    fn test_mask_preserves_property_id() {
        let mut thing = Thing::new("Test");
        thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
        let masked = mask_thing(&thing);
        assert_eq!(
            masked.identifiers[0].property_id,
            thing.identifiers[0].property_id
        );
    }

    /// Masking a record with no sensitive fields is a no-op.
    #[test]
    fn test_mask_no_sensitive_fields() {
        let thing = Thing::new("Public Thing");
        let masked = mask_thing(&thing);
        assert!(masked.owner.is_none());
        assert!(masked.identifiers.is_empty());
    }

    /// GDPR export carries field values through unmodified.
    #[test]
    fn test_gdpr_export_preserves_fields() {
        let mut thing = Thing::new("Export Test");
        thing.description = Some("A test thing".into());
        let export = gdpr_export(&thing);
        assert_eq!(export["name"], "Export Test");
        assert_eq!(export["description"], "A test thing");
    }

    /// GDPR export includes every top-level field for data portability.
    #[test]
    fn test_gdpr_export_has_all_top_level_fields() {
        let thing = Thing::new("Full Export");
        let export = gdpr_export(&thing);
        for key in [
            "id",
            "name",
            "alternate_names",
            "description",
            "disambiguating_description",
            "additional_type",
            "url",
            "identifiers",
            "images",
            "main_entity_of_page",
            "owner",
            "same_as",
            "subject_of",
            "potential_action",
            "is_deleted",
            "created_at",
            "updated_at",
        ] {
            assert!(export.get(key).is_some(), "missing field: {key}");
        }
    }
}
