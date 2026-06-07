//! Amenity features of a [`Place`](crate::models::place::Place).
//!
//! [`AmenityFeature`] models
//! [schema.org/LocationFeatureSpecification](https://schema.org/LocationFeatureSpecification):
//! a named feature with an optional value. The value distinguishes a present
//! boolean amenity (`"Elevator"`, no value) from one with detail
//! (`"Parking"` → `"100 spaces"`).
//!
//! # Examples
//!
//! ```
//! use place_service::models::amenity::AmenityFeature;
//!
//! let wifi = AmenityFeature::new("WiFi");
//! assert!(wifi.value.is_none());
//!
//! let parking = AmenityFeature::with_value("Parking", "100 spaces");
//! assert_eq!(parking.value.as_deref(), Some("100 spaces"));
//! ```

use serde::{Deserialize, Serialize};

/// A named amenity feature with an optional descriptive value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AmenityFeature {
    /// The feature's name, e.g. `"WiFi"` or `"Parking"`.
    pub name: String,
    /// Optional detail for the feature, e.g. `"100 spaces"`. `None` for a
    /// plain present/absent amenity.
    pub value: Option<String>,
}

impl AmenityFeature {
    /// Constructs a value-less amenity feature with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::amenity::AmenityFeature;
    ///
    /// let a = AmenityFeature::new("Elevator");
    /// assert!(a.value.is_none());
    /// ```
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: None,
        }
    }

    /// Constructs an amenity feature with both a name and a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::amenity::AmenityFeature;
    ///
    /// let a = AmenityFeature::with_value("Restaurant", "Le Jules Verne");
    /// assert_eq!(a.value.as_deref(), Some("Le Jules Verne"));
    /// ```
    pub fn with_value(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: Some(value.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new` sets the name and leaves the value unset.
    #[test]
    fn test_amenity_new() {
        let a = AmenityFeature::new("WiFi");
        assert_eq!(a.name, "WiFi");
        assert!(a.value.is_none());
    }

    /// `with_value` sets both the name and the value.
    #[test]
    fn test_amenity_with_value() {
        let a = AmenityFeature::with_value("Parking", "100 spaces");
        assert_eq!(a.name, "Parking");
        assert_eq!(a.value.as_deref(), Some("100 spaces"));
    }
}
