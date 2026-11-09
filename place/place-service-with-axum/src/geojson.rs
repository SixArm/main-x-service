//! `GeoJSON` (RFC 7946) rendering of a place (spec §13 T-7).
//!
//! A place becomes one `Feature`: its coordinates are the `Point`
//! geometry, and every other field is a property. A place with no
//! coordinates gets `"geometry": null`, which RFC 7946 §3.2 allows for an
//! unlocated feature, rather than a made-up point.
//!
//! Coordinates are copied from the place's own JSON, so they keep the
//! exact decimal digits stored (this crate builds `serde_json` with
//! `arbitrary_precision`) instead of passing through `f64`. The order is
//! longitude, latitude, then elevation when there is one (RFC 7946 §3.1.1).

use serde_json::{Map, Value, json};

use crate::models::place::Place;

/// The media type RFC 7946 §12 registers for `GeoJSON`.
pub const MEDIA_TYPE: &str = "application/geo+json";

/// Render `place` as a `GeoJSON` `Feature`.
///
/// # Errors
///
/// Returns the serialization error if the place cannot be rendered as
/// JSON, which a `Place` built by this crate never causes.
pub fn place_feature(place: &Place) -> Result<Value, serde_json::Error> {
    let Value::Object(mut properties) = serde_json::to_value(place)? else {
        return Ok(Value::Null);
    };
    properties.remove("id");
    let geometry = properties
        .remove("geo")
        .as_ref()
        .and_then(point)
        .unwrap_or(Value::Null);
    Ok(json!({
        "type": "Feature",
        "id": place.id.to_string(),
        "geometry": geometry,
        "properties": Value::Object(properties),
    }))
}

/// A `Point` from a place's serialized `geo`, or `None` when it has none.
fn point(geo: &Value) -> Option<Value> {
    let geo: &Map<String, Value> = geo.as_object()?;
    let mut position = vec![
        geo.get("longitude_as_decimal_degrees")?.clone(),
        geo.get("latitude_as_decimal_degrees")?.clone(),
    ];
    if let Some(elevation) = geo
        .get("elevation_as_decimal_metres")
        .filter(|v| !v.is_null())
    {
        position.push(elevation.clone());
    }
    Some(json!({ "type": "Point", "coordinates": position }))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;

    use super::*;
    use crate::models::geo::GeoCoordinates;

    fn located(lat: &str, lon: &str, elevation: Option<&str>) -> Place {
        let mut place = Place::new("Central Park");
        place.geo = Some(GeoCoordinates {
            latitude_as_decimal_degrees: BigDecimal::from_str(lat).unwrap(),
            longitude_as_decimal_degrees: BigDecimal::from_str(lon).unwrap(),
            elevation_as_decimal_metres: elevation.map(|e| BigDecimal::from_str(e).unwrap()),
        });
        place
    }

    #[test]
    fn a_located_place_is_a_point_feature_longitude_first() {
        let place = located("40.785091", "-73.968285", None);
        let feature = place_feature(&place).unwrap();
        assert_eq!(feature["type"], "Feature");
        assert_eq!(feature["id"], place.id.to_string());
        assert_eq!(feature["geometry"]["type"], "Point");
        assert_eq!(
            feature["geometry"]["coordinates"].to_string(),
            "[-73.968285,40.785091]"
        );
        assert_eq!(feature["properties"]["name"], "Central Park");
    }

    #[test]
    fn elevation_is_the_third_position_element() {
        let feature = place_feature(&located("1.5", "2.5", Some("42.25"))).unwrap();
        assert_eq!(
            feature["geometry"]["coordinates"].to_string(),
            "[2.5,1.5,42.25]"
        );
    }

    #[test]
    fn stored_digits_survive_without_rounding_through_f64() {
        let feature = place_feature(&located("51.50072919999999999", "-0.1246254", None)).unwrap();
        assert_eq!(
            feature["geometry"]["coordinates"].to_string(),
            "[-0.1246254,51.50072919999999999]"
        );
    }

    #[test]
    fn an_unlocated_place_has_null_geometry_not_a_made_up_point() {
        let feature = place_feature(&Place::new("Nowhere")).unwrap();
        assert!(feature["geometry"].is_null(), "{feature}");
        assert_eq!(feature["properties"]["name"], "Nowhere");
    }

    #[test]
    fn geometry_and_id_are_not_repeated_in_properties() {
        let feature = place_feature(&located("1", "2", None)).unwrap();
        let properties = feature["properties"].as_object().unwrap();
        assert!(!properties.contains_key("geo"));
        assert!(!properties.contains_key("id"));
    }
}
