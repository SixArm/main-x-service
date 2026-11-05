//! Structured postal address for a [`Place`](crate::models::place::Place).
//!
//! [`PostalAddress`] mirrors
//! [schema.org/PostalAddress](https://schema.org/PostalAddress): every
//! component is optional because source data is frequently partial (a place
//! may carry only a country, or only a postal code). The validation layer
//! requires at least one of locality / postal code / country; the matching
//! layer compares only the fields present in *both* addresses.
//!
//! # Examples
//!
//! ```
//! use place_service::models::address::PostalAddress;
//!
//! let addr = PostalAddress {
//!     street_address: Some("14 E 60th St".into()),
//!     address_locality: Some("New York".into()),
//!     address_region: Some("NY".into()),
//!     address_country: Some("US".into()),
//!     postal_code: Some("10022".into()),
//! };
//! assert_eq!(addr.address_locality.as_deref(), Some("New York"));
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A structured postal address (schema.org/PostalAddress).
///
/// All fields are optional. Field names follow schema.org spelling
/// (`address_locality`, `address_region`, `postal_code`) rather than the
/// British vocabulary used by the `place-matcher` crate; the matching
/// adapter renames them at the boundary.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct PostalAddress {
    /// Street address line, e.g. `"14 E 60th St"`.
    pub street_address: Option<String>,
    /// City / town / locality, e.g. `"New York"`.
    pub address_locality: Option<String>,
    /// State / province / region, e.g. `"NY"`.
    pub address_region: Option<String>,
    /// Country, typically an ISO 3166-1 code or country name, e.g. `"US"`.
    pub address_country: Option<String>,
    /// Postal / ZIP code, e.g. `"10022"`.
    pub postal_code: Option<String>,
}

impl PostalAddress {
    /// Constructs an empty address with every field set to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::address::PostalAddress;
    ///
    /// let addr = PostalAddress::new();
    /// assert!(addr.postal_code.is_none());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            street_address: None,
            address_locality: None,
            address_region: None,
            address_country: None,
            postal_code: None,
        }
    }
}

impl Default for PostalAddress {
    /// Equivalent to [`PostalAddress::new`] — an all-`None` address.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new` produces an address with no fields populated.
    #[test]
    fn test_postal_address_new() {
        let addr = PostalAddress::new();
        assert!(addr.street_address.is_none());
        assert!(addr.address_locality.is_none());
    }

    /// A fully-populated address exposes each field verbatim.
    #[test]
    fn test_postal_address_with_fields() {
        let addr = PostalAddress {
            street_address: Some("123 Main St".into()),
            address_locality: Some("Springfield".into()),
            address_region: Some("IL".into()),
            address_country: Some("US".into()),
            postal_code: Some("62701".into()),
        };
        assert_eq!(addr.street_address.as_deref(), Some("123 Main St"));
        assert_eq!(addr.postal_code.as_deref(), Some("62701"));
    }

    /// An address survives a JSON serialization round-trip unchanged.
    #[test]
    fn test_postal_address_serialization() {
        let addr = PostalAddress {
            street_address: Some("456 Oak Ave".into()),
            address_locality: Some("Portland".into()),
            address_region: Some("OR".into()),
            address_country: Some("US".into()),
            postal_code: Some("97201".into()),
        };
        let json = serde_json::to_string(&addr).unwrap();
        let deserialized: PostalAddress = serde_json::from_str(&json).unwrap();
        assert_eq!(addr, deserialized);
    }

    /// A partial address (only some fields set) is valid and preserved.
    #[test]
    fn test_postal_address_partial() {
        let addr = PostalAddress {
            street_address: None,
            address_locality: Some("London".into()),
            address_region: None,
            address_country: Some("GB".into()),
            postal_code: None,
        };
        assert!(addr.street_address.is_none());
        assert_eq!(addr.address_country.as_deref(), Some("GB"));
    }
}
