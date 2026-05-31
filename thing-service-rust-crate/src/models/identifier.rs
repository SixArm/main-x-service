use serde::{Deserialize, Serialize};

/// Canonical identifier scheme. Mirrors the values commonly found in
/// the `propertyID` attribute of a schema.org
/// [PropertyValue](https://schema.org/PropertyValue).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IdentifierType {
    Doi,
    Isbn,
    Issn,
    Gtin,
    Sku,
    Mpn,
    SerialNumber,
    Uri,
    Uuid,
    Custom(String),
}

/// A schema.org [PropertyValue](https://schema.org/PropertyValue)
/// identifier.
///
/// `property_id` corresponds to `propertyID`; `value` to `value`;
/// `name` is an optional human-readable label; `url` is an optional
/// authoritative URL for this identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThingIdentifier {
    pub property_id: IdentifierType,
    pub value: String,
    pub name: Option<String>,
    pub url: Option<String>,
}

impl ThingIdentifier {
    pub fn new(property_id: IdentifierType, value: &str) -> Self {
        Self {
            property_id,
            value: value.to_string(),
            name: None,
            url: None,
        }
    }

    pub fn doi(value: &str) -> Self { Self::new(IdentifierType::Doi, value) }
    pub fn isbn(value: &str) -> Self { Self::new(IdentifierType::Isbn, value) }
    pub fn issn(value: &str) -> Self { Self::new(IdentifierType::Issn, value) }
    pub fn gtin(value: &str) -> Self { Self::new(IdentifierType::Gtin, value) }
    pub fn sku(value: &str) -> Self { Self::new(IdentifierType::Sku, value) }
    pub fn mpn(value: &str) -> Self { Self::new(IdentifierType::Mpn, value) }
    pub fn serial_number(value: &str) -> Self { Self::new(IdentifierType::SerialNumber, value) }
    pub fn uri(value: &str) -> Self { Self::new(IdentifierType::Uri, value) }
    pub fn uuid(value: &str) -> Self { Self::new(IdentifierType::Uuid, value) }

    /// Identifiers in this set are treated as deterministically globally
    /// unique: a match on any of them short-circuits scoring to 1.0.
    pub fn is_deterministic(&self) -> bool {
        matches!(
            self.property_id,
            IdentifierType::Doi
                | IdentifierType::Isbn
                | IdentifierType::Issn
                | IdentifierType::Gtin
                | IdentifierType::Mpn
                | IdentifierType::SerialNumber
                | IdentifierType::Uuid
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_constructors() {
        assert_eq!(ThingIdentifier::isbn("9780141439518").property_id, IdentifierType::Isbn);
        assert_eq!(ThingIdentifier::doi("10.1000/xyz123").property_id, IdentifierType::Doi);
        assert_eq!(ThingIdentifier::gtin("00012345600012").property_id, IdentifierType::Gtin);
        assert_eq!(ThingIdentifier::sku("WIDGET-42").property_id, IdentifierType::Sku);
    }

    #[test]
    fn test_identifier_custom() {
        let id = ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1234");
        assert_eq!(id.value, "OL1234");
    }

    #[test]
    fn test_is_deterministic() {
        assert!(ThingIdentifier::isbn("9780141439518").is_deterministic());
        assert!(ThingIdentifier::doi("10.1000/abc").is_deterministic());
        assert!(ThingIdentifier::serial_number("SN-001").is_deterministic());
        assert!(!ThingIdentifier::sku("WIDGET-42").is_deterministic());
        assert!(!ThingIdentifier::uri("urn:example:1").is_deterministic());
        assert!(!ThingIdentifier::new(IdentifierType::Custom("Internal".into()), "X").is_deterministic());
    }

    #[test]
    fn test_identifier_serialization() {
        let id = ThingIdentifier::isbn("9780141439518");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ThingIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_identifier_with_name_and_url() {
        let mut id = ThingIdentifier::isbn("9780141439518");
        id.name = Some("Penguin Classics paperback".into());
        id.url = Some("https://www.worldcat.org/isbn/9780141439518".into());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ThingIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
        assert_eq!(deserialized.url.as_deref().unwrap(), "https://www.worldcat.org/isbn/9780141439518");
    }
}
