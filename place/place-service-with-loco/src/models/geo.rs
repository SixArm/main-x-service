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

use bigdecimal::{BigDecimal, ToPrimitive};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Convert an `f64` coordinate literal into the decimal it denotes.
///
/// Rust's `Display` for `f64` emits the **shortest decimal string that
/// round-trips**, so `40.7829_f64` formats as exactly `"40.7829"` — the
/// number the caller wrote. That is deliberately *not*
/// `BigDecimal::from_f64`, which expands the binary approximation to
/// `40.7828999999999979308995534665882587432861328125`: forty-six digits
/// of representation noise, worse than the `f64` it replaced and past any
/// sane scale cap.
///
/// # Panics
///
/// Panics if `value` is not finite — `NaN` and infinity have no decimal
/// form. See [`GeoCoordinates::new`] for why that is acceptable here.
fn decimal_from_f64(value: f64, field: &str) -> BigDecimal {
    assert!(value.is_finite(), "{field} must be finite, got {value}");
    format!("{value}")
        .parse()
        .expect("a finite f64 always formats as a parseable decimal")
}

/// A geographic point in the [WGS 84](https://en.wikipedia.org/wiki/World_Geodetic_System)
/// reference system, modeled on [schema.org/GeoCoordinates](https://schema.org/GeoCoordinates).
///
/// Coordinates are **exact decimal degrees** ([`BigDecimal`]), not binary
/// floats. A coordinate is a decimal quantity: an `f64` cannot hold
/// `40.7829` — it holds `40.78289999999999793…` — and cannot distinguish
/// `40.7829` from `40.78290000000000001` at all. Stored as `NUMERIC`, a
/// coordinate round-trips as the digits the caller sent.
///
/// Two consequences worth knowing:
///
/// - **The wire format is a JSON number, not a string.** `BigDecimal`'s
///   default serde representation is a quoted string; these fields opt
///   into `bigdecimal::impl_serde::arbitrary_precision` so the JSON is
///   unchanged from when they were `f64`.
/// - **Distance is still floating-point.** [`distance_to`](Self::distance_to)
///   converts at its own boundary, because Haversine is trigonometry.
///   Exactness is a property of what is stored and returned, not of the
///   distance score.
///
/// Valid ranges are `-90..=90` for [`latitude`](Self::latitude) and
/// `-180..=180` for [`longitude`](Self::longitude); enforcement of those
/// ranges lives in the validation layer (`crate::validation`), not in this
/// value object, so a `GeoCoordinates` can be constructed from raw input
/// before it is validated. The type does, however, make a non-finite
/// coordinate *unrepresentable* — a decimal has no `NaN` — which closes a
/// hole the `f64` version had: `NaN` compares false against both range
/// bounds, so a `NaN` latitude passed validation unnoticed.
///
/// # Examples
///
/// ```
/// use bigdecimal::BigDecimal;
/// use place_service::models::geo::GeoCoordinates;
///
/// // Summit of Mount Everest, including elevation in meters.
/// let everest = GeoCoordinates {
///     latitude: "27.9881".parse().unwrap(),
///     longitude: "86.9250".parse().unwrap(),
///     elevation: Some("8848.86".parse().unwrap()),
/// };
/// assert_eq!(everest.elevation, Some("8848.86".parse::<BigDecimal>().unwrap()));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct GeoCoordinates {
    /// Latitude in decimal degrees. Positive is north of the equator,
    /// negative is south. Expected range: `-90..=90`.
    #[serde(with = "bigdecimal::impl_serde::arbitrary_precision")]
    #[schema(value_type = f64)]
    pub latitude: BigDecimal,
    /// Longitude in decimal degrees. Positive is east of the prime
    /// meridian, negative is west. Expected range: `-180..=180`.
    #[serde(with = "bigdecimal::impl_serde::arbitrary_precision")]
    #[schema(value_type = f64)]
    pub longitude: BigDecimal,
    /// Optional elevation above the WGS 84 ellipsoid, in meters. `None`
    /// when the source data did not supply an elevation; elevation does
    /// not participate in [`distance_to`](Self::distance_to).
    #[serde(default, with = "bigdecimal::impl_serde::arbitrary_precision_option")]
    #[schema(value_type = Option<f64>)]
    pub elevation: Option<BigDecimal>,
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
    /// use bigdecimal::BigDecimal;
    /// use place_service::models::geo::GeoCoordinates;
    ///
    /// // The stored value is the decimal the literal denotes, exactly.
    /// let p = GeoCoordinates::new(40.7829, -73.9654);
    /// assert_eq!(p.latitude, "40.7829".parse::<BigDecimal>().unwrap());
    /// assert!(p.elevation.is_none());
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if either argument is not finite. `NaN` and infinity have no
    /// decimal form, and this constructor is a convenience over literal
    /// coordinates — untrusted input reaches this type through
    /// deserialization or the database, never through here. (`std` takes
    /// the same line with `Duration::from_secs_f64`.)
    #[must_use]
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude: decimal_from_f64(latitude, "latitude"),
            longitude: decimal_from_f64(longitude, "longitude"),
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
    /// Returns `NaN` if either coordinate is too large to represent as an
    /// `f64`. Callers compare the result against a radius, and every
    /// comparison against `NaN` is false, so an unrepresentable coordinate
    /// fails closed — no false proximity match — rather than producing a
    /// plausible wrong distance.
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
    #[must_use]
    pub fn distance_to(&self, other: &GeoCoordinates) -> f64 {
        // Mean Earth radius in meters; the Haversine formula assumes a sphere.
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        // Coordinates are exact decimals; Haversine is trigonometry, so
        // convert at this boundary only.
        let (Some(self_lat), Some(self_lon), Some(other_lat), Some(other_lon)) = (
            self.latitude.to_f64(),
            self.longitude.to_f64(),
            other.latitude.to_f64(),
            other.longitude.to_f64(),
        ) else {
            return f64::NAN;
        };

        // The formula operates in radians, so convert every angle up front.
        let lat1 = self_lat.to_radians();
        let lat2 = other_lat.to_radians();
        // Deltas between the two points along each axis.
        let dlat = (other_lat - self_lat).to_radians();
        let dlon = (other_lon - self_lon).to_radians();

        // `a` is the square of half the chord length between the points.
        let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
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
        // Exact, not epsilon-tolerant: the point of the decimal is that
        // `40.7829` stores as `40.7829`, not `40.78289999999999793…`.
        assert_eq!(geo.latitude, "40.7829".parse::<BigDecimal>().unwrap());
        assert_eq!(geo.longitude, "-73.9654".parse::<BigDecimal>().unwrap());
        assert!(geo.elevation.is_none());
    }

    /// Elevation can be supplied via a struct literal and round-trips.
    #[test]
    fn test_geo_with_elevation() {
        let geo = GeoCoordinates {
            latitude: "27.9881".parse().unwrap(),
            longitude: "86.9250".parse().unwrap(),
            elevation: Some("8848.86".parse().unwrap()),
        };
        assert_eq!(
            geo.elevation,
            Some("8848.86".parse::<BigDecimal>().unwrap())
        );
    }

    /// `new` records the decimal the caller wrote, not the binary
    /// approximation of it.
    ///
    /// `BigDecimal::from_f64(40.7829)` would store
    /// `40.7828999999999979308995534665882587432861328125`. Going through
    /// the shortest round-tripping string keeps the intended value — and
    /// keeps it inside the validator's scale cap.
    #[test]
    fn new_stores_the_decimal_the_literal_denotes() {
        for (lat, lon) in [(40.7829, -73.9654), (0.1, 0.2), (-90.0, 180.0)] {
            let geo = GeoCoordinates::new(lat, lon);
            assert_eq!(geo.latitude.to_string(), format!("{lat}"));
            assert_eq!(geo.longitude.to_string(), format!("{lon}"));
        }
    }

    /// Coordinates stay JSON **numbers** on the wire.
    ///
    /// `BigDecimal`'s default serde impl emits a quoted string; this type
    /// opts into `arbitrary_precision` precisely so the representation is
    /// unchanged from when these were `f64`. A client parsing `latitude`
    /// as a number must keep working.
    #[test]
    fn coordinates_serialize_as_json_numbers() {
        let geo = GeoCoordinates {
            latitude: "27.9881".parse().unwrap(),
            longitude: "86.9250".parse().unwrap(),
            elevation: Some("8848.86".parse().unwrap()),
        };
        let json = serde_json::to_string(&geo).unwrap();
        assert!(json.contains(r#""latitude":27.9881"#), "{json}");
        assert!(json.contains(r#""longitude":86.9250"#), "{json}");
        assert!(json.contains(r#""elevation":8848.86"#), "{json}");
        assert!(
            !json.contains(r#""27.9881""#),
            "quoted, not a number: {json}"
        );
        assert_eq!(serde_json::from_str::<GeoCoordinates>(&json).unwrap(), geo);
    }

    /// A coordinate round-trips as the exact decimal it was given —
    /// including one an `f64` could not distinguish from its neighbour.
    #[test]
    fn coordinates_round_trip_exactly() {
        for lat in ["40.7829", "0.1", "40.78290000000000001", "-90"] {
            let geo = GeoCoordinates {
                latitude: lat.parse().unwrap(),
                longitude: "0".parse().unwrap(),
                elevation: None,
            };
            let json = serde_json::to_string(&geo).unwrap();
            assert!(json.contains(&format!(r#""latitude":{lat}"#)), "{json}");
            let back: GeoCoordinates = serde_json::from_str(&json).unwrap();
            assert_eq!(back.latitude.to_string(), lat);
        }
    }

    /// An absent elevation stays `null`, and reads back as `None` whether
    /// the key is `null` or missing — the behaviour `Option<f64>` had.
    #[test]
    fn absent_elevation_is_null_not_omitted() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        let json = serde_json::to_string(&geo).unwrap();
        assert!(json.contains(r#""elevation":null"#), "{json}");
        assert_eq!(serde_json::from_str::<GeoCoordinates>(&json).unwrap(), geo);

        let missing = r#"{"latitude":40.7829,"longitude":-73.9654}"#;
        assert_eq!(
            serde_json::from_str::<GeoCoordinates>(missing).unwrap(),
            geo
        );
    }

    /// `new` refuses a non-finite coordinate rather than inventing one.
    ///
    /// A decimal has no `NaN`. The `f64` version silently accepted one —
    /// and `NaN` compares false against both range bounds, so it also
    /// passed validation.
    #[test]
    #[should_panic(expected = "latitude must be finite")]
    fn new_rejects_non_finite_latitude() {
        let _ = GeoCoordinates::new(f64::NAN, 0.0);
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
        assert!(
            (dist_km - 3944.0).abs() < 50.0,
            "NYC-LAX distance: {dist_km} km"
        );
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
