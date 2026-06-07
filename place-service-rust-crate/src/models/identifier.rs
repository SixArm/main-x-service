//! External identifiers for a [`Place`](crate::models::place::Place).
//!
//! A place can carry many identifiers from different namespaces — a GS1
//! Global Location Number, a US Census FIPS code, a USGS GNIS feature ID, an
//! OpenStreetMap ID, or any custom scheme. Each is a
//! ([`IdentifierType`], value) pair. In matching, an exact type+value match
//! is a strong signal, and a GLN match in particular is treated as
//! deterministic (short-circuits the score to 1.0).
//!
//! # Examples
//!
//! ```
//! use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
//!
//! let gln = PlaceIdentifier::gln("1234567890123");
//! assert_eq!(gln.identifier_type, IdentifierType::GlobalLocationNumber);
//!
//! let fips = PlaceIdentifier::new(IdentifierType::Fips, "36061");
//! assert_eq!(fips.value, "36061");
//! ```

use serde::{Deserialize, Serialize};

/// The namespace / scheme an identifier value belongs to.
///
/// Derives `Eq` and `Hash` so identifiers can key maps/sets. The
/// [`Custom`](Self::Custom) variant names any scheme outside the fixed list
/// (e.g. an IATA airport code).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IdentifierType {
    /// GS1 Global Location Number (13 digits). Deterministic match signal.
    GlobalLocationNumber,
    /// Operator-defined branch / site code.
    BranchCode,
    /// US Census Bureau FIPS code.
    Fips,
    /// USGS Geographic Names Information System feature ID.
    Gnis,
    /// OpenStreetMap object ID.
    OpenStreetMap,
    /// Any other scheme, named by the inner string.
    Custom(String),
}

/// An external identifier: a scheme plus its value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaceIdentifier {
    /// Which namespace the [`value`](Self::value) belongs to.
    pub identifier_type: IdentifierType,
    /// The identifier string within that namespace.
    pub value: String,
}

impl PlaceIdentifier {
    /// Constructs an identifier of the given type and value.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
    ///
    /// let id = PlaceIdentifier::new(IdentifierType::Gnis, "975772");
    /// assert_eq!(id.value, "975772");
    /// ```
    pub fn new(identifier_type: IdentifierType, value: &str) -> Self {
        Self {
            identifier_type,
            value: value.to_string(),
        }
    }

    /// Convenience constructor for a Global Location Number identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
    ///
    /// let id = PlaceIdentifier::gln("1234567890123");
    /// assert_eq!(id.identifier_type, IdentifierType::GlobalLocationNumber);
    /// ```
    pub fn gln(value: &str) -> Self {
        Self::new(IdentifierType::GlobalLocationNumber, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `gln` builds a GLN-typed identifier carrying the given value.
    #[test]
    fn test_identifier_gln() {
        let id = PlaceIdentifier::gln("1234567890123");
        assert_eq!(id.identifier_type, IdentifierType::GlobalLocationNumber);
        assert_eq!(id.value, "1234567890123");
    }

    /// `new` with a `Custom` type stores the value verbatim.
    #[test]
    fn test_identifier_custom() {
        let id = PlaceIdentifier::new(IdentifierType::Custom("NPI".into()), "ABC123");
        assert_eq!(id.value, "ABC123");
    }

    /// An identifier survives a JSON serialization round-trip.
    #[test]
    fn test_identifier_serialization() {
        let id = PlaceIdentifier::gln("1234567890123");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: PlaceIdentifier = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
