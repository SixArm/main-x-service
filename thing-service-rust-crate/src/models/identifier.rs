//! Typed identifiers for a [`Thing`](crate::models::thing::Thing).
//!
//! A [`ThingIdentifier`](crate::models::identifier::ThingIdentifier) follows
//! the schema.org [PropertyValue](https://schema.org/PropertyValue) shape: a
//! scheme ([`IdentifierType`](crate::models::identifier::IdentifierType)) plus
//! a value, with optional human-readable name and authoritative URL.
//! Identifiers are the strongest matching signal — a shared globally-unique
//! identifier (see
//! [`ThingIdentifier::is_deterministic`](crate::models::identifier::ThingIdentifier::is_deterministic))
//! short-circuits matching to a perfect score.
//!
//! # Examples
//!
//! ```
//! use thing_service::models::identifier::{IdentifierType, ThingIdentifier};
//!
//! let isbn = ThingIdentifier::isbn("9780141439518");
//! assert_eq!(isbn.property_id, IdentifierType::Isbn);
//! assert!(isbn.is_deterministic());
//!
//! // A vendor SKU is not globally unique.
//! assert!(!ThingIdentifier::sku("WIDGET-42").is_deterministic());
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Canonical identifier scheme. Mirrors the values commonly found in
/// the `propertyID` attribute of a schema.org
/// [PropertyValue](https://schema.org/PropertyValue).
///
/// Most variants are globally unique by construction and therefore
/// "deterministic" for matching purposes; [`Sku`](Self::Sku),
/// [`Uri`](Self::Uri), and [`Custom`](Self::Custom) are not. See
/// [`ThingIdentifier::is_deterministic`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub enum IdentifierType {
    /// Digital Object Identifier (`10.<registrant>/<suffix>`).
    Doi,
    /// International Standard Book Number (10- or 13-digit).
    Isbn,
    /// International Standard Serial Number (8 characters).
    Issn,
    /// Global Trade Item Number (8/12/13/14-digit; UPC/EAN/JAN).
    Gtin,
    /// Stock-Keeping Unit — vendor-scoped, *not* globally unique.
    Sku,
    /// Manufacturer Part Number.
    Mpn,
    /// Manufacturer serial number.
    SerialNumber,
    /// Generic URI — *not* treated as globally unique for matching.
    Uri,
    /// UUID (RFC 4122).
    Uuid,
    /// Any other scheme, carrying its free-text label (e.g.
    /// `Custom("OpenLibrary")`). *Not* globally unique.
    Custom(String),
}

/// A schema.org [PropertyValue](https://schema.org/PropertyValue)
/// identifier.
///
/// `property_id` corresponds to `propertyID`; `value` to `value`;
/// `name` is an optional human-readable label; `url` is an optional
/// authoritative URL for this identifier.
///
/// # Examples
///
/// ```
/// use thing_service::models::identifier::{IdentifierType, ThingIdentifier};
///
/// let mut id = ThingIdentifier::doi("10.1038/nature12373");
/// id.name = Some("Nature article".into());
/// assert_eq!(id.property_id, IdentifierType::Doi);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ThingIdentifier {
    /// The identifier scheme — schema.org `propertyID`.
    pub property_id: IdentifierType,
    /// The identifier value within that scheme — schema.org `value`.
    pub value: String,
    /// Optional human-readable label — schema.org `name`.
    pub name: Option<String>,
    /// Optional authoritative URL for this identifier — schema.org `url`.
    /// Cleared by [`mask_thing`](crate::privacy::mask_thing).
    pub url: Option<String>,
}

impl ThingIdentifier {
    /// Constructs an identifier from an explicit scheme and value, with no
    /// `name` or `url`. Prefer the per-scheme helpers (e.g. [`isbn`](Self::isbn))
    /// where one exists; use this for [`IdentifierType::Custom`].
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_service::models::identifier::{IdentifierType, ThingIdentifier};
    ///
    /// let id = ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W");
    /// assert_eq!(id.value, "OL1394865W");
    /// ```
    pub fn new(property_id: IdentifierType, value: &str) -> Self {
        Self {
            property_id,
            value: value.to_string(),
            name: None,
            url: None,
        }
    }

    /// Constructs a [`Doi`](IdentifierType::Doi) identifier.
    pub fn doi(value: &str) -> Self {
        Self::new(IdentifierType::Doi, value)
    }
    /// Constructs an [`Isbn`](IdentifierType::Isbn) identifier.
    pub fn isbn(value: &str) -> Self {
        Self::new(IdentifierType::Isbn, value)
    }
    /// Constructs an [`Issn`](IdentifierType::Issn) identifier.
    pub fn issn(value: &str) -> Self {
        Self::new(IdentifierType::Issn, value)
    }
    /// Constructs a [`Gtin`](IdentifierType::Gtin) identifier.
    pub fn gtin(value: &str) -> Self {
        Self::new(IdentifierType::Gtin, value)
    }
    /// Constructs a [`Sku`](IdentifierType::Sku) identifier.
    pub fn sku(value: &str) -> Self {
        Self::new(IdentifierType::Sku, value)
    }
    /// Constructs an [`Mpn`](IdentifierType::Mpn) identifier.
    pub fn mpn(value: &str) -> Self {
        Self::new(IdentifierType::Mpn, value)
    }
    /// Constructs a [`SerialNumber`](IdentifierType::SerialNumber) identifier.
    pub fn serial_number(value: &str) -> Self {
        Self::new(IdentifierType::SerialNumber, value)
    }
    /// Constructs a [`Uri`](IdentifierType::Uri) identifier.
    pub fn uri(value: &str) -> Self {
        Self::new(IdentifierType::Uri, value)
    }
    /// Constructs a [`Uuid`](IdentifierType::Uuid) identifier.
    pub fn uuid(value: &str) -> Self {
        Self::new(IdentifierType::Uuid, value)
    }

    /// Returns `true` if this identifier's scheme is globally unique by
    /// construction (DOI, ISBN, ISSN, GTIN, MPN, serial number, or UUID).
    ///
    /// A match on any deterministic identifier short-circuits scoring to
    /// 1.0. `Sku`, `Uri`, and `Custom` are excluded because they are not
    /// globally unique.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_service::models::identifier::ThingIdentifier;
    ///
    /// assert!(ThingIdentifier::isbn("9780141439518").is_deterministic());
    /// assert!(!ThingIdentifier::sku("WIDGET-42").is_deterministic());
    /// ```
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

    /// Each per-scheme helper sets the matching `property_id`.
    #[test]
    fn test_identifier_constructors() {
        assert_eq!(
            ThingIdentifier::isbn("9780141439518").property_id,
            IdentifierType::Isbn
        );
        assert_eq!(
            ThingIdentifier::doi("10.1000/xyz123").property_id,
            IdentifierType::Doi
        );
        assert_eq!(
            ThingIdentifier::gtin("00012345600012").property_id,
            IdentifierType::Gtin
        );
        assert_eq!(
            ThingIdentifier::sku("WIDGET-42").property_id,
            IdentifierType::Sku
        );
    }

    /// The `Custom` variant carries its label and stores the value.
    #[test]
    fn test_identifier_custom() {
        let id = ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1234");
        assert_eq!(id.value, "OL1234");
    }

    /// `is_deterministic` is true for globally-unique schemes only.
    #[test]
    fn test_is_deterministic() {
        assert!(ThingIdentifier::isbn("9780141439518").is_deterministic());
        assert!(ThingIdentifier::doi("10.1000/abc").is_deterministic());
        assert!(ThingIdentifier::serial_number("SN-001").is_deterministic());
        assert!(!ThingIdentifier::sku("WIDGET-42").is_deterministic());
        assert!(!ThingIdentifier::uri("urn:example:1").is_deterministic());
        assert!(
            !ThingIdentifier::new(IdentifierType::Custom("Internal".into()), "X")
                .is_deterministic()
        );
    }

    /// An identifier survives a JSON serialization round-trip.
    #[test]
    fn test_identifier_serialization() {
        let id = ThingIdentifier::isbn("9780141439518");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ThingIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    /// Optional `name` and `url` PropertyValue fields round-trip.
    #[test]
    fn test_identifier_with_name_and_url() {
        let mut id = ThingIdentifier::isbn("9780141439518");
        id.name = Some("Penguin Classics paperback".into());
        id.url = Some("https://www.worldcat.org/isbn/9780141439518".into());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: ThingIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
        assert_eq!(
            deserialized.url.as_deref().unwrap(),
            "https://www.worldcat.org/isbn/9780141439518"
        );
    }
}
