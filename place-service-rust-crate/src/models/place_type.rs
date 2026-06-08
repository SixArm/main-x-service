//! Classification of a [`Place`](crate::models::place::Place), drawn from the
//! [schema.org/Place](https://schema.org/Place) subtype hierarchy.
//!
//! [`PlaceType`] is a closed set of common categories plus an open-ended
//! [`Other`](PlaceType::Other) escape hatch for anything not enumerated. In
//! matching it contributes a binary signal: equal types score 1.0, unequal
//! types score 0.0.
//!
//! # Examples
//!
//! ```
//! use place_service::models::place_type::PlaceType;
//!
//! assert_eq!(PlaceType::Park.to_string(), "Park");
//! assert_eq!(PlaceType::Other("Marina".into()).to_string(), "Marina");
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The kind of place, modeled on schema.org/Place subtypes.
///
/// Derives `Eq` and `Hash` so place types can be used as map/set keys. The
/// [`Other`](Self::Other) variant carries a free-form label for categories
/// outside the fixed list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub enum PlaceType {
    /// A commercial establishment (shop, office, …).
    LocalBusiness,
    /// A civic / public structure (monument, government building, …).
    CivicStructure,
    /// A governed geographic area (country, state, municipality, …).
    AdministrativeArea,
    /// A natural geographic feature (mountain, lake, …).
    Landform,
    /// A park.
    Park,
    /// An airport.
    Airport,
    /// A hospital.
    Hospital,
    /// A school.
    School,
    /// A library.
    Library,
    /// A museum.
    Museum,
    /// A restaurant.
    Restaurant,
    /// A hotel.
    Hotel,
    /// Any category not covered above, carrying its own label.
    Other(String),
}

impl std::fmt::Display for PlaceType {
    /// Formats the type as a bare label: the inner string for
    /// [`Other`](Self::Other), otherwise the variant name (via `Debug`).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Unwrap the custom label rather than printing `Other("…")`.
            PlaceType::Other(s) => write!(f, "{s}"),
            // For unit variants, Debug already yields the plain variant name.
            _ => write!(f, "{self:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Display` yields the variant name, and the inner label for `Other`.
    #[test]
    fn test_place_type_display() {
        assert_eq!(PlaceType::Park.to_string(), "Park");
        assert_eq!(PlaceType::Other("Marina".into()).to_string(), "Marina");
    }

    /// Equal variants compare equal; distinct variants do not.
    #[test]
    fn test_place_type_equality() {
        assert_eq!(PlaceType::Hospital, PlaceType::Hospital);
        assert_ne!(PlaceType::School, PlaceType::Library);
    }

    /// A unit variant survives a JSON serialization round-trip.
    #[test]
    fn test_place_type_serialization() {
        let pt = PlaceType::Restaurant;
        let json = serde_json::to_string(&pt).unwrap();
        let deserialized: PlaceType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, deserialized);
    }

    /// The `Other` variant round-trips with its label intact.
    #[test]
    fn test_place_type_other_serialization() {
        let pt = PlaceType::Other("GolfCourse".into());
        let json = serde_json::to_string(&pt).unwrap();
        let deserialized: PlaceType = serde_json::from_str(&json).unwrap();
        assert_eq!(pt, deserialized);
    }
}
