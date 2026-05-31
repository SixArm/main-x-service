use crate::models::geo::GeoCoordinates;

/// Geo similarity based on Haversine distance.
/// Returns 1.0 for same point, decays toward 0.0 as distance increases.
pub fn geo_similarity(a: &GeoCoordinates, b: &GeoCoordinates) -> f64 {
    geo_similarity_with_reference(a, b, 1.0)
}

/// Geo similarity with configurable reference distance (km).
pub fn geo_similarity_with_reference(
    a: &GeoCoordinates,
    b: &GeoCoordinates,
    reference_km: f64,
) -> f64 {
    let dist_km = a.distance_to(b) / 1000.0;
    1.0 / (1.0 + dist_km / reference_km)
}

/// Check if two coordinates are within a given radius (meters).
pub fn within_radius(a: &GeoCoordinates, b: &GeoCoordinates, radius_m: f64) -> bool {
    a.distance_to(b) <= radius_m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_point() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        let score = geo_similarity(&geo, &geo);
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_close_points() {
        let a = GeoCoordinates::new(40.7829, -73.9654);
        let b = GeoCoordinates::new(40.7830, -73.9655);
        let score = geo_similarity(&a, &b);
        assert!(score > 0.95, "Score: {score}");
    }

    #[test]
    fn test_moderate_distance() {
        let a = GeoCoordinates::new(40.7580, -73.9855);
        let b = GeoCoordinates::new(40.7484, -73.9857);
        let score = geo_similarity(&a, &b);
        assert!(score > 0.3, "Score: {score}");
        assert!(score < 0.9, "Score: {score}");
    }

    #[test]
    fn test_far_apart() {
        let nyc = GeoCoordinates::new(40.7128, -74.0060);
        let london = GeoCoordinates::new(51.5074, -0.1278);
        let score = geo_similarity(&nyc, &london);
        assert!(score < 0.001, "Score: {score}");
    }

    #[test]
    fn test_within_radius_true() {
        let a = GeoCoordinates::new(40.7829, -73.9654);
        let b = GeoCoordinates::new(40.7830, -73.9655);
        assert!(within_radius(&a, &b, 100.0));
    }

    #[test]
    fn test_within_radius_false() {
        let nyc = GeoCoordinates::new(40.7128, -74.0060);
        let london = GeoCoordinates::new(51.5074, -0.1278);
        assert!(!within_radius(&nyc, &london, 1000.0));
    }

    #[test]
    fn test_custom_reference() {
        let a = GeoCoordinates::new(40.7829, -73.9654);
        let b = GeoCoordinates::new(40.7929, -73.9754);
        let tight = geo_similarity_with_reference(&a, &b, 0.1);
        let loose = geo_similarity_with_reference(&a, &b, 10.0);
        assert!(loose > tight, "loose: {loose}, tight: {tight}");
    }
}
