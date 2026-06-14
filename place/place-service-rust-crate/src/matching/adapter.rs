//! Adapter from the service's `Place` domain model to the `place-matcher`
//! library's `Place` matching input.
//!
//! The service stores a rich, schema.org-shaped `Place` (`PostalAddress`,
//! `GeoCoordinates`, `PlaceType`, `PlaceIdentifier` list, opening hours,
//! amenity features, audit timestamps). The `place-matcher` crate accepts a
//! flat, builder-shaped `Place` with `Address`/`PlaceCategory`/`PlaceId`,
//! latitude/longitude as bare `Option<f64>`, and `country`/`county`/
//! `postcode` (British English) instead of the service's
//! `address_country`/`address_region`/`postal_code` (schema.org English).
//!
//! [`to_matcher_place`] performs the projection so callers can use the
//! canonical algorithm without rewriting their domain model.
//!
//! See `agents/share/match.md` and the matcher crate's `spec.md §5–§7` for
//! the algorithm contract this adapter feeds.
//!
//! # Mapping
//!
//! | Service field | Matcher slot |
//! |---|---|
//! | `name` | `name` |
//! | `alternate_name` (Option) | one entry in `alternate_names` |
//! | `place_type` | `category` (mapped via [`map_place_type`]) |
//! | `address.street_address` | `address.line1` |
//! | `address.address_locality` | `address.city` |
//! | `address.address_region` | `address.county` |
//! | `address.postal_code` | `address.postcode` |
//! | `address.address_country` | `address.country` + `country_code_as_iso_3166_1_alpha_2` if 2-char |
//! | `geo.latitude` / `.longitude` / `.elevation` | `latitude`/`longitude`/`elevation_as_metre` |
//! | `telephone` | `phone` |
//! | `global_location_number` | `add_place_id(Other("GLN"), value)` |
//! | `branch_code` | `add_place_id(Other("BranchCode"), value)` |
//! | `identifiers[]` | mapped via [`map_identifier_scheme`] |
//! | `maximum_attendee_capacity` | `maximum_capacity_count` |

use place_matcher::{
    Address as MAddress, Place as MPlace, PlaceCategory as MCategory, PlaceId as MPlaceId,
    PlaceIdScheme as MScheme,
};

use crate::models::{
    address::PostalAddress, identifier::IdentifierType, place::Place, place_type::PlaceType,
};

/// Convert a service `Place` into a `place_matcher::Place` ready for
/// `MatchingEngine::match_places` / `deterministic_match`.
///
/// This projection is intentionally lossy — registry-only fields (`id`,
/// `is_deleted`, `created_at`, `keywords`, `amenity_features`,
/// `opening_hours`, `description`, `fax_number`, `url`, `public_access`,
/// `smoking_allowed`, …) are dropped because the matcher does not consume
/// them.
pub fn to_matcher_place(p: &Place) -> MPlace {
    let mut b = MPlace::builder().name(p.name.trim());

    if let Some(alt) = p
        .alternate_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        b = b.add_alternate_name(alt);
    }

    if let Some(t) = &p.place_type {
        b = b.category(map_place_type(t));
    }

    if let Some(addr) = p.address.as_ref().and_then(map_address) {
        b = b.address(addr);
    }

    if let Some(country) = p
        .address
        .as_ref()
        .and_then(|a| a.address_country.as_deref())
        .map(str::trim)
        .filter(|c| c.len() == 2)
    {
        b = b.country_code_as_iso_3166_1_alpha_2(country.to_ascii_uppercase());
    }

    if let Some(geo) = &p.geo {
        b = b.latitude(geo.latitude).longitude(geo.longitude);
        if let Some(e) = geo.elevation {
            b = b.elevation_as_metre(e);
        }
    }

    if let Some(t) = p
        .telephone
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        b = b.phone(t);
    }

    if let Some(gln) = p
        .global_location_number
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        && let Some(pid) = MPlaceId::new(MScheme::Other("GLN".into()), gln)
    {
        b = b.add_place_id(pid);
    }
    if let Some(bc) = p
        .branch_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        && let Some(pid) = MPlaceId::new(MScheme::Other("BranchCode".into()), bc)
    {
        b = b.add_place_id(pid);
    }

    for id in &p.identifiers {
        if let Some(pid) =
            MPlaceId::new(map_identifier_scheme(&id.identifier_type), id.value.trim())
        {
            b = b.add_place_id(pid);
        }
    }

    if let Some(cap) = p.maximum_attendee_capacity {
        b = b.maximum_capacity_count(cap);
    }

    b.build()
}

/// Map the service's `PlaceType` enum to the matcher's `PlaceCategory`.
///
/// The matcher's vocabulary is broader (34 variants vs. the service's 12);
/// service-side `Other(s)` flows through as `MCategory::Other(s)`.
#[must_use]
pub fn map_place_type(t: &PlaceType) -> MCategory {
    match t {
        PlaceType::LocalBusiness => MCategory::Other("LocalBusiness".into()),
        PlaceType::CivicStructure => MCategory::Other("CivicStructure".into()),
        PlaceType::AdministrativeArea => MCategory::Other("AdministrativeArea".into()),
        PlaceType::Landform => MCategory::Mountain,
        PlaceType::Park => MCategory::Park,
        PlaceType::Airport => MCategory::Airport,
        PlaceType::Hospital => MCategory::Hospital,
        PlaceType::School => MCategory::School,
        PlaceType::Library => MCategory::Library,
        PlaceType::Museum => MCategory::Museum,
        PlaceType::Restaurant => MCategory::Restaurant,
        PlaceType::Hotel => MCategory::Hotel,
        PlaceType::Other(s) => MCategory::Other(s.clone()),
    }
}

/// Map the service's `IdentifierType` enum to a matcher `PlaceIdScheme`.
///
/// The matcher uses its enumerated `PlaceIdScheme` (Google, OSM nodes,
/// Wikidata, …) plus `Other(String)` for everything else; service-side
/// schemes that have no direct enum variant fall through as `Other(name)`.
#[must_use]
pub fn map_identifier_scheme(t: &IdentifierType) -> MScheme {
    match t {
        IdentifierType::GlobalLocationNumber => MScheme::Other("GLN".into()),
        IdentifierType::BranchCode => MScheme::Other("BranchCode".into()),
        IdentifierType::Fips => MScheme::Other("FIPS".into()),
        IdentifierType::Gnis => MScheme::Other("GNIS".into()),
        IdentifierType::OpenStreetMap => MScheme::OsmNode,
        IdentifierType::Custom(s) => MScheme::Other(s.clone()),
    }
}

/// Project the service `PostalAddress` (schema.org English field names) onto
/// the matcher's `Address` (British English field names).
///
/// Returns `None` when the source address is entirely empty, so the caller
/// can skip attaching an address slot rather than send a hollow one (which
/// would otherwise pull the matcher's address component toward a spurious
/// score). The field renames are: `street_address` → `line1`,
/// `address_locality` → `city`, `address_region` → `county`, `postal_code`
/// → `postcode`, `address_country` → `country`.
fn map_address(a: &PostalAddress) -> Option<MAddress> {
    // Bail out unless at least one source field is populated; an all-`None`
    // address carries no matching signal and must not become an empty slot.
    let any = a.street_address.is_some()
        || a.address_locality.is_some()
        || a.address_region.is_some()
        || a.address_country.is_some()
        || a.postal_code.is_some();
    if !any {
        return None;
    }
    let mut m = MAddress::new();
    if let Some(v) = a.street_address.as_deref() {
        m = m.with_line1(v);
    }
    if let Some(v) = a.address_locality.as_deref() {
        m = m.with_city(v);
    }
    if let Some(v) = a.address_region.as_deref() {
        // Field rename: the matcher's `county` slot holds the service's
        // `address_region` (state/province/region) value verbatim.
        m = m.with_county(v);
    }
    if let Some(v) = a.postal_code.as_deref() {
        m = m.with_postcode(v);
    }
    if let Some(v) = a.address_country.as_deref() {
        m = m.with_country(v);
    }
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::identifier::PlaceIdentifier;

    /// Pins that `name` carries through verbatim and `geo.latitude` /
    /// `geo.longitude` route to the matcher's bare `latitude` / `longitude`.
    #[test]
    fn round_trip_basic_name_and_geo() {
        use crate::models::geo::GeoCoordinates;
        let mut svc = Place::new("Central Park");
        svc.geo = Some(GeoCoordinates {
            latitude: 40.7829,
            longitude: -73.9654,
            elevation: None,
        });
        let m = to_matcher_place(&svc);
        assert_eq!(m.name.as_deref(), Some("Central Park"));
        assert_eq!(m.latitude, Some(40.7829));
        assert_eq!(m.longitude, Some(-73.9654));
    }

    /// Pins the address field renames (`locality`→`city`,
    /// `region`→`county`, `postal_code`→`postcode`) and that a 2-char
    /// `address_country` also populates `country_code_as_iso_3166_1_alpha_2`.
    #[test]
    fn address_renames_locality_region_to_city_county() {
        let mut svc = Place::new("Test");
        svc.address = Some(PostalAddress {
            street_address: Some("1 Broadway".into()),
            address_locality: Some("New York".into()),
            address_region: Some("NY".into()),
            address_country: Some("US".into()),
            postal_code: Some("10004".into()),
        });
        let m = to_matcher_place(&svc);
        let a = m.address.as_ref().unwrap();
        assert_eq!(a.line1.as_deref(), Some("1 Broadway"));
        assert_eq!(a.city.as_deref(), Some("New York"));
        assert_eq!(a.county.as_deref(), Some("NY"));
        assert_eq!(a.postcode.as_deref(), Some("10004"));
        assert_eq!(m.country_code_as_iso_3166_1_alpha_2.as_deref(), Some("US"));
    }

    /// Pins that the top-level `global_location_number` field routes to a
    /// single `PlaceId` under the `Other("GLN")` scheme with its value intact.
    #[test]
    fn gln_routed_to_place_id_with_gln_scheme() {
        let mut svc = Place::new("Test");
        svc.global_location_number = Some("0614141999996".into());
        let m = to_matcher_place(&svc);
        assert_eq!(m.place_ids.len(), 1);
        assert_eq!(m.place_ids[0].scheme, MScheme::Other("GLN".into()));
        assert_eq!(m.place_ids[0].value, "0614141999996");
    }

    /// Pins that an `OpenStreetMap` identifier maps to the matcher's
    /// enumerated `OsmNode` scheme (not a generic `Other`).
    #[test]
    fn place_identifier_osm_to_osm_node() {
        let mut svc = Place::new("Test");
        svc.identifiers.push(PlaceIdentifier::new(
            IdentifierType::OpenStreetMap,
            "node:42",
        ));
        let m = to_matcher_place(&svc);
        assert_eq!(m.place_ids[0].scheme, MScheme::OsmNode);
    }

    /// Pins that a `PlaceType` maps to the corresponding matcher
    /// `PlaceCategory` variant (here `Museum`).
    #[test]
    fn place_type_maps_to_category() {
        let mut svc = Place::new("Test");
        svc.place_type = Some(PlaceType::Museum);
        let m = to_matcher_place(&svc);
        assert_eq!(m.category, Some(MCategory::Museum));
    }
}
