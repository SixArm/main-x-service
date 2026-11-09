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

/// Render `places` as a `GeoJSON` `FeatureCollection`, one `Feature` each.
///
/// # Errors
///
/// As [`place_feature`].
pub fn feature_collection(places: &[Place]) -> Result<Value, serde_json::Error> {
    let features = places
        .iter()
        .map(place_feature)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "type": "FeatureCollection", "features": features }))
}

/// A two-dimensional `GeoJSON` bounding box (RFC 7946 §5), in degrees.
///
/// `west > east` is a box that crosses the antimeridian (§5.2), not an
/// error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// Westernmost longitude, `-180..=180`.
    pub west: f64,
    /// Southernmost latitude, `-90..=90`.
    pub south: f64,
    /// Easternmost longitude, `-180..=180`.
    pub east: f64,
    /// Northernmost latitude, `-90..=90`, at least `south`.
    pub north: f64,
}

impl BoundingBox {
    /// Parse `west,south,east,north`, the RFC 7946 §5 order.
    ///
    /// # Errors
    ///
    /// Says what is wrong: not exactly four finite numbers, a latitude or
    /// longitude out of range, or `south` above `north`. A three-dimensional
    /// box, with six numbers, is refused rather than read as a flat one.
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        let numbers = text
            .split(',')
            .map(|part| part.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
            .collect::<Option<Vec<f64>>>()
            .ok_or("bbox must be four comma-separated finite numbers")?;
        let [west, south, east, north] = numbers[..] else {
            return Err("bbox must be four comma-separated numbers: west,south,east,north");
        };
        if ![west, east]
            .iter()
            .all(|lon| (-180.0..=180.0).contains(lon))
        {
            return Err("bbox longitudes must be in -180..=180");
        }
        if ![south, north]
            .iter()
            .all(|lat| (-90.0..=90.0).contains(lat))
        {
            return Err("bbox latitudes must be in -90..=90");
        }
        if south > north {
            return Err("bbox south must not be above north");
        }
        Ok(Self {
            west,
            south,
            east,
            north,
        })
    }
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

    #[test]
    fn a_feature_collection_holds_one_feature_per_place() {
        let places = [located("1", "2", None), Place::new("Nowhere")];
        let collection = feature_collection(&places).unwrap();
        assert_eq!(collection["type"], "FeatureCollection");
        let features = collection["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0]["id"], places[0].id.to_string());
        assert!(features[1]["geometry"].is_null());
    }

    #[test]
    fn an_empty_collection_is_still_a_collection() {
        let collection = feature_collection(&[]).unwrap();
        assert_eq!(
            collection.to_string(),
            r#"{"features":[],"type":"FeatureCollection"}"#
        );
    }

    #[test]
    fn a_bbox_is_west_south_east_north() {
        let bbox = BoundingBox::parse("-74.1, 40.6,-73.8,40.9").unwrap();
        assert_eq!(
            bbox,
            BoundingBox {
                west: -74.1,
                south: 40.6,
                east: -73.8,
                north: 40.9
            }
        );
    }

    #[test]
    fn a_bbox_across_the_antimeridian_is_accepted() {
        let bbox = BoundingBox::parse("170,-20,-170,-10").unwrap();
        assert!(bbox.west > bbox.east);
    }

    #[test]
    fn a_malformed_bbox_is_refused() {
        for bad in [
            "",
            "1,2,3",
            "1,2,3,4,5,6",
            "a,2,3,4",
            "NaN,2,3,4",
            "inf,2,3,4",
            "-181,0,0,1",
            "0,-91,1,0",
            "0,10,1,5",
        ] {
            assert!(
                BoundingBox::parse(bad).is_err(),
                "{bad:?} should be refused"
            );
        }
    }
}
