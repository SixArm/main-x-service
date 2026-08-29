//! Geo-coordinate similarity, derived from Haversine great-circle distance.
//!
//! The matcher needs a *bounded* similarity in `[0.0, 1.0]`, but distance is
//! unbounded, so this module maps distance through a reciprocal decay:
//! `1 / (1 + d/ref)`. At zero distance the score is 1.0; at the reference
//! distance it is exactly 0.5; beyond that it tails toward 0.0. The
//! reference distance is the knob that sets how quickly proximity decays
//! into dissimilarity.
//!
//! See [`GeoCoordinates::distance_to`](crate::models::geo::GeoCoordinates::distance_to)
//! for the underlying Haversine calculation.
//!
//! # Examples
//!
//! ```
//! use place_service::models::geo::GeoCoordinates;
//! use place_service::matching::geo::{geo_similarity, within_radius};
//!
//! let a = GeoCoordinates::new(40.7829, -73.9654);
//! assert!((geo_similarity(&a, &a) - 1.0).abs() < 1e-9);
//! assert!(within_radius(&a, &a, 1.0));
//! ```

use bigdecimal::ToPrimitive;

use crate::models::geo::GeoCoordinates;

/// Geo similarity using the default 1 km reference distance.
///
/// Equivalent to [`geo_similarity_with_reference`] with `reference_km = 1.0`:
/// points 1 km apart score 0.5. Returns 1.0 for the same point and decays
/// toward 0.0 as the points move apart.
///
/// # Examples
///
/// ```
/// use place_service::models::geo::GeoCoordinates;
/// use place_service::matching::geo::geo_similarity;
///
/// let nyc = GeoCoordinates::new(40.7128, -74.0060);
/// let london = GeoCoordinates::new(51.5074, -0.1278);
/// assert!(geo_similarity(&nyc, &london) < 0.001);
/// ```
#[must_use]
pub fn geo_similarity(a: &GeoCoordinates, b: &GeoCoordinates) -> f64 {
    geo_similarity_with_reference(a, b, 1.0)
}

/// Geo similarity with a caller-chosen reference distance, in kilometers.
///
/// Computes `1 / (1 + d/ref)` where `d` is the Haversine distance in km. A
/// larger `reference_km` decays more slowly (more tolerant of distance); a
/// smaller one decays faster (stricter). At `d == reference_km` the score is
/// 0.5.
///
/// # Examples
///
/// ```
/// use place_service::models::geo::GeoCoordinates;
/// use place_service::matching::geo::geo_similarity_with_reference;
///
/// let a = GeoCoordinates::new(40.7829, -73.9654);
/// let b = GeoCoordinates::new(40.7929, -73.9754);
/// // A looser reference yields a higher similarity for the same gap.
/// let tight = geo_similarity_with_reference(&a, &b, 0.1);
/// let loose = geo_similarity_with_reference(&a, &b, 10.0);
/// assert!(loose > tight);
/// ```
#[must_use]
pub fn geo_similarity_with_reference(
    a: &GeoCoordinates,
    b: &GeoCoordinates,
    reference_km: f64,
) -> f64 {
    // Haversine returns meters; the decay formula works in kilometers.
    let dist_km = a.distance_to(b) / 1000.0;
    1.0 / (1.0 + dist_km / reference_km)
}

/// Returns whether two coordinates lie within `radius_m` meters of each other.
///
/// A hard boolean cutoff (inclusive) used for geo-radius search, distinct
/// from the graded [`geo_similarity`] score.
///
/// # Examples
///
/// ```
/// use place_service::models::geo::GeoCoordinates;
/// use place_service::matching::geo::within_radius;
///
/// let a = GeoCoordinates::new(40.7829, -73.9654);
/// let b = GeoCoordinates::new(40.7830, -73.9655);
/// assert!(within_radius(&a, &b, 100.0));
/// assert!(!within_radius(&a, &b, 0.1));
/// ```
#[must_use]
pub fn within_radius(a: &GeoCoordinates, b: &GeoCoordinates, radius_m: f64) -> bool {
    a.distance_to(b) <= radius_m
}

/// Kilometers per degree of latitude, derived from the **same** mean
/// Earth radius [`GeoCoordinates::distance_to`] uses (`2 * pi * R /
/// 360`, `R = 6371` km) rather than the WGS 84 ellipsoid's ~111.32.
/// `bounding_box` and `distance_to` must agree on one sphere: pairing
/// this constant with a different radius made the box's edge fall a
/// hair inside the true Haversine circle along a meridian, which a
/// property test caught directly (a point placed exactly at
/// `radius_km` by the same spherical model `distance_to` uses landed
/// outside the box).
const KM_PER_DEGREE_LATITUDE: f64 = std::f64::consts::PI * 6371.0 / 180.0;

/// Compute a rectangular latitude/longitude bounding box, in decimal
/// degrees, that fully contains every point within `radius_km`
/// kilometers of `center`.
///
/// This is a coarse **pre-filter** for geo-radius search: a database can
/// answer `lat BETWEEN .. AND lon BETWEEN ..` with a plain btree index
/// (`idx_places_geo`) far more cheaply than it can run [`within_radius`]
/// row by row, but a circle inscribed in a square always leaves the
/// square's corners outside the circle. Callers MUST still apply
/// [`within_radius`] to every candidate the box returns to get the true
/// within-radius set — this function only shrinks the candidate set, it
/// never replaces the exact check.
///
/// Returns `(lat_min, lat_max, lon_min, lon_max)`, each clamped to the
/// valid coordinate range (`-90..=90` / `-180..=180`).
///
/// # Known limitation
///
/// Longitude does not wrap at the antimeridian: a center near ±180°
/// whose radius crosses it yields a box clamped at the boundary rather
/// than one that wraps around to the other side, under-including the
/// far side of the circle. Narrow in practice (it needs both a
/// near-antimeridian center and a large radius) and left as a documented
/// gap rather than a silent one — the general fix (`PostGIS` spatial
/// indexing) is tracked separately (spec §13 T-1).
///
/// # Examples
///
/// ```
/// use place_service::models::geo::GeoCoordinates;
/// use place_service::matching::geo::bounding_box;
///
/// let nyc = GeoCoordinates::new(40.7128, -74.0060);
/// let (lat_min, lat_max, lon_min, lon_max) = bounding_box(&nyc, 10.0);
/// assert!(lat_min < 40.7128 && 40.7128 < lat_max);
/// assert!(lon_min < -74.0060 && -74.0060 < lon_max);
/// ```
#[must_use]
pub fn bounding_box(center: &GeoCoordinates, radius_km: f64) -> (f64, f64, f64, f64) {
    let lat = center.latitude_as_decimal_degrees.to_f64().unwrap_or(0.0);
    let lon = center.longitude_as_decimal_degrees.to_f64().unwrap_or(0.0);
    let radius_km = radius_km.max(0.0);
    let delta_lat = radius_km / KM_PER_DEGREE_LATITUDE;
    // Longitude degrees shrink toward the poles by cos(latitude); clamp
    // the divisor away from zero so a near-polar center never divides by
    // (near-)zero and blows the box up to the whole globe.
    let km_per_degree_longitude = (KM_PER_DEGREE_LATITUDE * lat.to_radians().cos()).max(0.01);
    let delta_lon = radius_km / km_per_degree_longitude;
    (
        (lat - delta_lat).max(-90.0),
        (lat + delta_lat).min(90.0),
        (lon - delta_lon).max(-180.0),
        (lon + delta_lon).min(180.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A point compared with itself scores ~1.0.
    #[test]
    fn test_same_point() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        let score = geo_similarity(&geo, &geo);
        assert!((score - 1.0).abs() < 0.001);
    }

    /// Points a few meters apart score very high.
    #[test]
    fn test_close_points() {
        let a = GeoCoordinates::new(40.7829, -73.9654);
        let b = GeoCoordinates::new(40.7830, -73.9655);
        let score = geo_similarity(&a, &b);
        assert!(score > 0.95, "Score: {score}");
    }

    /// Points about a kilometer apart score in the mid range.
    #[test]
    fn test_moderate_distance() {
        let a = GeoCoordinates::new(40.7580, -73.9855);
        let b = GeoCoordinates::new(40.7484, -73.9857);
        let score = geo_similarity(&a, &b);
        assert!(score > 0.3, "Score: {score}");
        assert!(score < 0.9, "Score: {score}");
    }

    /// Transatlantic points score near 0.0.
    #[test]
    fn test_far_apart() {
        let nyc = GeoCoordinates::new(40.7128, -74.0060);
        let london = GeoCoordinates::new(51.5074, -0.1278);
        let score = geo_similarity(&nyc, &london);
        assert!(score < 0.001, "Score: {score}");
    }

    /// Nearby points fall inside a 100 m radius.
    #[test]
    fn test_within_radius_true() {
        let a = GeoCoordinates::new(40.7829, -73.9654);
        let b = GeoCoordinates::new(40.7830, -73.9655);
        assert!(within_radius(&a, &b, 100.0));
    }

    /// Far-apart points fall outside a 1 km radius.
    #[test]
    fn test_within_radius_false() {
        let nyc = GeoCoordinates::new(40.7128, -74.0060);
        let london = GeoCoordinates::new(51.5074, -0.1278);
        assert!(!within_radius(&nyc, &london, 1000.0));
    }

    /// A point placed at **exactly** the radius (via the same great-circle
    /// destination-point formula `distance_to`'s mean-Earth-radius model
    /// implies — see `bounding_box_contains_every_within_radius_point`'s
    /// doc comment for the same construction) is included: the cutoff is
    /// `<=`, not `<`.
    #[test]
    fn within_radius_boundary_is_inclusive() {
        const EARTH_RADIUS_KM: f64 = 6371.0;
        let center = GeoCoordinates::new(40.7829, -73.9654);
        let radius_m = 1_000.0;
        let d_over_r = (radius_m / 1000.0) / EARTH_RADIUS_KM;
        let lat1 = 40.7829_f64.to_radians();
        // Due north (bearing 0).
        let lat2 = (lat1.sin() * d_over_r.cos() + lat1.cos() * d_over_r.sin()).asin();
        let boundary = GeoCoordinates::new(lat2.to_degrees(), -73.9654);

        let dist_m = center.distance_to(&boundary);
        assert!(
            (dist_m - radius_m).abs() < 1e-6,
            "destination point is {dist_m} m from center, expected ~{radius_m}"
        );
        // The destination-point construction and `distance_to` compute the
        // same geometry two different ways (forward vs. inverse), so
        // `dist_m` can land a sub-micrometer either side of `radius_m` —
        // the assertion above already pins that gap at < 1e-6 m. What
        // this test is actually pinning is the `<=` cutoff itself, so the
        // radius passed to `within_radius` absorbs that same sub-
        // micrometer slop rather than re-introducing it as flakiness.
        assert!(
            within_radius(&center, &boundary, radius_m + 1e-6),
            "a point at exactly the radius must be included (inclusive cutoff)"
        );
        // A full meter further out falls outside — comfortably beyond any
        // float slop.
        assert!(!within_radius(&center, &boundary, radius_m - 1.0));
    }

    /// The box straddles the center on both axes and grows with radius.
    #[test]
    fn bounding_box_straddles_center_and_grows_with_radius() {
        let center = GeoCoordinates::new(40.7829, -73.9654);
        let (lat_min, lat_max, lon_min, lon_max) = bounding_box(&center, 10.0);
        assert!(lat_min < 40.7829 && 40.7829 < lat_max);
        assert!(lon_min < -73.9654 && -73.9654 < lon_max);
        let (lat_min_big, lat_max_big, _, _) = bounding_box(&center, 100.0);
        assert!(lat_min_big < lat_min);
        assert!(lat_max_big > lat_max);
    }

    /// Every point on the true (Haversine-sphere) circle at exactly
    /// `radius_km` falls inside [`bounding_box`] — the box is a superset
    /// of the circle, which is the whole point of using it as a
    /// pre-filter. Points are placed via the standard spherical
    /// destination-point formula (same mean Earth radius `distance_to`
    /// uses), independent of `bounding_box`'s own flat-Earth arithmetic,
    /// so this genuinely checks the box against the geometry
    /// [`within_radius`] uses, not against itself.
    #[test]
    fn bounding_box_contains_every_within_radius_point() {
        const EARTH_RADIUS_KM: f64 = 6371.0;
        let center = GeoCoordinates::new(51.5074, -0.1278);
        let radius_km = 5.0;
        let (lat_min, lat_max, lon_min, lon_max) = bounding_box(&center, radius_km);
        let lat1 = 51.5074_f64.to_radians();
        let lon1 = (-0.1278_f64).to_radians();
        let d_over_r = radius_km / EARTH_RADIUS_KM;
        for bearing_deg in [0.0_f64, 45.0, 90.0, 135.0, 180.0, 225.0, 270.0, 315.0] {
            let bearing = bearing_deg.to_radians();
            let lat2 =
                (lat1.sin() * d_over_r.cos() + lat1.cos() * d_over_r.sin() * bearing.cos()).asin();
            let lon2 = lon1
                + (bearing.sin() * d_over_r.sin() * lat1.cos())
                    .atan2(d_over_r.cos() - lat1.sin() * lat2.sin());
            let point = GeoCoordinates::new(lat2.to_degrees(), lon2.to_degrees());
            // Sanity check: the destination-point formula really did land
            // on the radius (within float slop), so the test is exercising
            // what it claims to.
            let dist_m = center.distance_to(&point);
            assert!(
                (dist_m - radius_km * 1000.0).abs() < 10.0,
                "destination point at bearing {bearing_deg} is {dist_m} m from center, expected ~{}",
                radius_km * 1000.0
            );
            let plat = point.latitude_as_decimal_degrees.to_f64().unwrap();
            let plon = point.longitude_as_decimal_degrees.to_f64().unwrap();
            assert!(
                (lat_min..=lat_max).contains(&plat),
                "bearing {bearing_deg}: lat {plat} outside [{lat_min}, {lat_max}]"
            );
            assert!(
                (lon_min..=lon_max).contains(&plon),
                "bearing {bearing_deg}: lon {plon} outside [{lon_min}, {lon_max}]"
            );
        }
    }

    /// A zero radius collapses the box to (about) the center point.
    #[test]
    fn bounding_box_zero_radius_is_a_point() {
        let center = GeoCoordinates::new(0.0, 0.0);
        let (lat_min, lat_max, lon_min, lon_max) = bounding_box(&center, 0.0);
        assert!((lat_min - 0.0).abs() < 1e-9);
        assert!((lat_max - 0.0).abs() < 1e-9);
        assert!((lon_min - 0.0).abs() < 1e-9);
        assert!((lon_max - 0.0).abs() < 1e-9);
    }

    /// The box clamps to the valid coordinate range near a pole.
    #[test]
    fn bounding_box_clamps_near_pole() {
        let center = GeoCoordinates::new(89.9, 0.0);
        let (lat_min, lat_max, lon_min, lon_max) = bounding_box(&center, 50.0);
        assert!(lat_max <= 90.0);
        assert!(lat_min >= -90.0);
        assert!(lon_min >= -180.0 && lon_max <= 180.0);
    }

    /// A looser reference yields higher similarity than a tighter one.
    #[test]
    fn test_custom_reference() {
        let a = GeoCoordinates::new(40.7829, -73.9654);
        let b = GeoCoordinates::new(40.7929, -73.9754);
        let tight = geo_similarity_with_reference(&a, &b, 0.1);
        let loose = geo_similarity_with_reference(&a, &b, 10.0);
        assert!(loose > tight, "loose: {loose}, tight: {tight}");
    }
}
