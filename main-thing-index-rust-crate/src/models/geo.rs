use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeoCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    pub elevation: Option<f64>,
}

impl GeoCoordinates {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            elevation: None,
        }
    }

    /// Haversine distance in meters between two coordinates.
    pub fn distance_to(&self, other: &GeoCoordinates) -> f64 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let dlat = (other.latitude - self.latitude).to_radians();
        let dlon = (other.longitude - self.longitude).to_radians();

        let a = (dlat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_M * c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_new() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        assert!((geo.latitude - 40.7829).abs() < f64::EPSILON);
        assert!((geo.longitude + 73.9654).abs() < f64::EPSILON);
        assert!(geo.elevation.is_none());
    }

    #[test]
    fn test_geo_with_elevation() {
        let geo = GeoCoordinates {
            latitude: 27.9881,
            longitude: 86.9250,
            elevation: Some(8848.86),
        };
        assert_eq!(geo.elevation, Some(8848.86));
    }

    #[test]
    fn test_haversine_same_point() {
        let geo = GeoCoordinates::new(40.7829, -73.9654);
        let dist = geo.distance_to(&geo);
        assert!(dist.abs() < 0.01);
    }

    #[test]
    fn test_haversine_known_distance() {
        // New York to Los Angeles: ~3944 km
        let nyc = GeoCoordinates::new(40.7128, -74.0060);
        let lax = GeoCoordinates::new(33.9425, -118.4081);
        let dist_km = nyc.distance_to(&lax) / 1000.0;
        assert!((dist_km - 3944.0).abs() < 50.0, "NYC-LAX distance: {dist_km} km");
    }

    #[test]
    fn test_haversine_short_distance() {
        let a = GeoCoordinates::new(51.5074, -0.1278);
        let b = GeoCoordinates::new(51.5174, -0.1278);
        let dist = a.distance_to(&b);
        assert!((dist - 1112.0).abs() < 10.0, "Short distance: {dist} m");
    }

    #[test]
    fn test_haversine_antipodal() {
        let a = GeoCoordinates::new(0.0, 0.0);
        let b = GeoCoordinates::new(0.0, 180.0);
        let dist_km = a.distance_to(&b) / 1000.0;
        assert!((dist_km - 20015.0).abs() < 100.0, "Antipodal: {dist_km} km");
    }

    #[test]
    fn test_geo_serialization() {
        let geo = GeoCoordinates::new(48.8566, 2.3522);
        let json = serde_json::to_string(&geo).unwrap();
        let deserialized: GeoCoordinates = serde_json::from_str(&json).unwrap();
        assert_eq!(geo, deserialized);
    }
}
