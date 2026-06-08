//! Geographic coordinates for a [`Place`](crate::models::place::Place).
//!
//! This module defines [`GeoCoordinates`], a WGS 84 latitude/longitude
//! (and optional elevation) value object modeled on
//! [schema.org/GeoCoordinates](https://schema.org/GeoCoordinates), plus a
//! [great-circle distance](https://en.wikipedia.org/wiki/Great-circle_distance)
//! calculation via the [Haversine formula](https://en.wikipedia.org/wiki/Haversine_formula).
//!
//! Geo distance feeds the geo component of the place-matching score (see
//! `crate::matching::geo`): two places that sit close together on the globe
//! are more likely to be the same real-world place.
//!
//! # Examples
//!
//! ```
//! use place_service::models::geo::GeoCoordinates;
//!
//! let nyc = GeoCoordinates::new(40.7128, -74.0060);
//! let lax = GeoCoordinates::new(33.9425, -118.4081);
//!
//! // Haversine distance is returned in meters.
//! let km = nyc.distance_to(&lax) / 1000.0;
//! assert!((km - 3944.0).abs() < 50.0);
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A geographic point in the [WGS 84](https://en.wikipedia.org/wiki/World_Geodetic_System)
/// reference system, modeled on [schema.org/GeoCoordinates](https://schema.org/GeoCoordinates).
///
/// Coordinates are stored as decimal degrees. Valid ranges are
/// `-90.0..=90.0` for [`latitude`](Self::latitude) and `-180.0..=180.0`
/// for [`longitude`](Self::longitude); enforcement of those ranges lives in
/// the validation layer (`crate::validation`), not in this value object, so a
/// `GeoCoordinates` can be constructed from raw input before it is validated.
///
/// # Examples
///
/// ```
/// use place_service::models::geo::GeoCoordinates;
///
/// // Summit of Mount Everest, including elevation in meters.
/// let everest = GeoCoordinates {
///     latitude: 27.9881,
///     longitude: 86.9250,
///     elevation: Some(8848.86),
/// };
/// assert_eq!(everest.elevation, Some(8848.86));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct GeoCoordinates {
    /// Latitude in decimal degrees. Positive is north of the equator,
    /// negative is south. Expected range: `-90.0..=90.0`.
    pub latitude: f64,
    /// Longitude in decimal degrees. Positive is east of the prime
    /// meridian, negative is west. Expected range: `-180.0..=180.0`.
    pub longitude: f64,
    /// Optional elevation above the WGS 84 ellipsoid, in meters. `None`
    /// when the source data did not supply an elevation; elevation does
    /// not participate in [`distance_to`](Self::distance_to).
    pub elevation: Option<f64>,
}

impl GeoCoordinates {
    /// Constructs a coordinate pair at sea level (elevation `None`).
    ///
    /// This is the common constructor; set [`elevation`](Self::elevation)
    /// directly via a struct literal when a third dimension is needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::geo::GeoCoordinates;
    ///
    /// let p = GeoCoordinates::new(40.7829, -73.9654);
    /// assert_eq!(p.latitude, 40.7829);
    /// assert!(p.elevation.is_none());
    /// ```
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            // Default to "unknown elevation" rather than 0.0, since 0.0 is a
            // valid sea-level reading and would be indistinguishable from
            // missing data.
            elevation: None,
        }
    }

    /// Returns the great-circle distance to `other`, in **meters**, using
    /// the [Haversine formula](https://en.wikipedia.org/wiki/Haversine_formula).
    ///
    /// The calculation treats the Earth as a sphere of radius
    /// 6,371,000 m (the mean radius). This is accurate to within ~0.5% for
    /// most place-matching purposes; it ignores the WGS 84 ellipsoidal
    /// flattening and any difference in [`elevation`](Self::elevation).
    ///
    /// The result is symmetric: `a.distance_to(&b) == b.distance_to(&a)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_service::models::geo::GeoCoordinates;
    ///
    /// let a = GeoCoordinates::new(51.5074, -0.1278);
    /// // One hundredth of a degree of latitude is ~1.11 km.
    /// let b = GeoCoordinates::new(51.5174, -0.1278);
    /// let meters = a.distance_to(&b);
    /// assert!((meters - 1112.0).abs() < 10.0);
    ///
    /// // Distance from a point to itself is zero.
    /// assert!(a.distance_to(&a).abs() < 0.01);
    /// ```
    pub fn distance_to(&self, other: &GeoCoordinates) -> f64 {
        // Mean Earth radius in meters; the Haversine formula assumes a sphere.
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        // The formula operates in radians, so convert every angle up front.
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        // Deltas between the two points along each axis.
        let dlat = (other.latitude - self.latitude).to_radians();
        let dlon = (other.longitude - self.longitude).to_radians();

        // `a` is the square of half the chord length between the points.
        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        // `c` is the angular distance in radians. Using asin(sqrt(a)) (the
        // half-angle form) is numerically stable for the small distances
        // that dominate place matching.
        let c = 2.0 * a.sqrt().asin();

        // Arc length = radius * central angle.
        EARTH_RADIUS_M * c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new` stores latitude/longitude verbatim and leaves elevation unset.
    #[test]
    fn test_geo_new() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        assert!((geo.latitude - 40.7829).abs() < f64::EPSILON);
        assert!((geo.longitude + 73.9654).abs() < f64::EPSILON);
        assert!(geo.elevation.is_none());
    }

    /// Elevation can be supplied via a struct literal and round-trips.
    #[test]
    fn test_geo_with_elevation() {
        let geo = GeoCoordinates {
            latitude: 27.9881,
            longitude: 86.9250,
            elevation: Some(8848.86),
        };
        assert_eq!(geo.elevation, Some(8848.86));
    }

    /// A point is zero distance from itself (degenerate Haversine case).
    #[test]
    fn test_haversine_same_point() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        let dist = geo.distance_to(&geo);
        assert!(dist.abs() < 0.01);
    }

    /// Cross-continental sanity check against a known reference distance.
    #[test]
    fn test_haversine_known_distance() {
        // New York to Los Angeles: ~3944 km
        let nyc = GeoCoordinates::new(40.7128, -74.0060);
        let lax = GeoCoordinates::new(33.9425, -118.4081);
        let dist_km = nyc.distance_to(&lax) / 1000.0;
        assert!((dist_km - 3944.0).abs() < 50.0, "NYC-LAX distance: {dist_km} km");
    }

    /// Sub-kilometer accuracy: 0.01° of latitude is ~1.11 km.
    #[test]
    fn test_haversine_short_distance() {
        let a = GeoCoordinates::new(51.5074, -0.1278);
        let b = GeoCoordinates::new(51.5174, -0.1278);
        let dist = a.distance_to(&b);
        assert!((dist - 1112.0).abs() < 10.0, "Short distance: {dist} m");
    }

    /// Antipodal points are ~half the Earth's circumference apart.
    #[test]
    fn test_haversine_antipodal() {
        let a = GeoCoordinates::new(0.0, 0.0);
        let b = GeoCoordinates::new(0.0, 180.0);
        let dist_km = a.distance_to(&b) / 1000.0;
        assert!((dist_km - 20015.0).abs() < 100.0, "Antipodal: {dist_km} km");
    }

    /// Coordinates survive a JSON serialization round-trip unchanged.
    #[test]
    fn test_geo_serialization() {
        let geo = GeoCoordinates::new(48.8566, 2.3522);
        let json = serde_json::to_string(&geo).unwrap();
        let deserialized: GeoCoordinates = serde_json::from_str(&json).unwrap();
        assert_eq!(geo, deserialized);
    }
}
