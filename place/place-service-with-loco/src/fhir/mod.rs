//! HL7 FHIR R5 interop for the `Location` resource.
//!
//! Adopts the family FHIR contract
//! ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)) — copied from
//! the `organization` reference implementation and adapted to the place
//! service's own domain [`Place`](crate::models::place::Place) DTO (this
//! crate stores a rich normalized `Place`, not a matcher type as JSONB).
//! It provides the bidirectional mapping between the stored `Place` and the
//! wire-level [`resources::FhirLocation`]: [`to_fhir_location`] for outbound
//! responses and [`from_fhir_location`] for inbound requests. Resource
//! shapes live in [`resources`], search-parameter parsing in [`search`], and
//! the mounted Axum endpoints in [`crate::controllers::fhir`].
//!
//! Conversions are **lossy where the DTO has no FHIR home** — documented
//! inline and gathered in [`from_fhir_location`]'s doc — never silent.

/// FHIR resource + envelope wire types (`Location`, `OperationOutcome`,
/// `Bundle`).
pub mod resources;
/// FHIR search-parameter parsing + the in-memory match predicate.
pub mod search;

use crate::models::address::PostalAddress;
use crate::models::geo::GeoCoordinates;
use crate::models::identifier::{IdentifierType, PlaceIdentifier};
use crate::models::place::Place;
use crate::models::place_type::PlaceType;
use resources::{
    FhirAddress, FhirCodeableConcept, FhirCoding, FhirContactPoint, FhirIdentifier, FhirLocation,
    FhirMeta, FhirPosition,
};

/// The FHIR `identifier.system` URI for a GS1 Global Location Number.
const GLN_SYSTEM: &str = "https://www.gs1.org/gln";
/// The FHIR `identifier.system` URI for a place branch / site code.
const BRANCH_SYSTEM: &str = "urn:mxi:place:branchcode";
/// The code-system URI backing `Location.type` codings.
const TYPE_SYSTEM: &str = "urn:mxi:place:type";

/// Map a [`PlaceIdentifier`] scheme to its FHIR `identifier.system` URI.
/// The well-known GLN registry uses its canonical namespace; the rest use a
/// family `urn:mxi:place:*` namespace. [`system_to_identifier_type`] is the
/// exact inverse, so a scheme round-trips through FHIR unchanged.
#[must_use]
pub fn identifier_type_to_system(kind: &IdentifierType) -> String {
    match kind {
        IdentifierType::GlobalLocationNumber => GLN_SYSTEM.to_string(),
        IdentifierType::BranchCode => BRANCH_SYSTEM.to_string(),
        IdentifierType::Fips => "urn:mxi:place:fips".to_string(),
        IdentifierType::Gnis => "urn:mxi:place:gnis".to_string(),
        IdentifierType::OpenStreetMap => "https://www.openstreetmap.org".to_string(),
        IdentifierType::Custom(label) => format!("urn:mxi:place:custom:{label}"),
    }
}

/// Map a FHIR `identifier.system` URI back to a [`PlaceIdentifier`] scheme —
/// the inverse of [`identifier_type_to_system`]. An unrecognised system is
/// preserved as `Custom(system)` so an inbound identifier from a foreign
/// namespace is never dropped.
#[must_use]
pub fn system_to_identifier_type(system: &str) -> IdentifierType {
    match system {
        GLN_SYSTEM => IdentifierType::GlobalLocationNumber,
        BRANCH_SYSTEM => IdentifierType::BranchCode,
        "urn:mxi:place:fips" => IdentifierType::Fips,
        "urn:mxi:place:gnis" => IdentifierType::Gnis,
        "https://www.openstreetmap.org" => IdentifierType::OpenStreetMap,
        other => other.strip_prefix("urn:mxi:place:custom:").map_or_else(
            || IdentifierType::Custom(other.to_string()),
            |label| IdentifierType::Custom(label.to_string()),
        ),
    }
}

/// Render a [`PlaceType`] as a `Location.type` `CodeableConcept`.
fn place_type_to_concept(pt: &PlaceType) -> FhirCodeableConcept {
    let label = pt.to_string();
    FhirCodeableConcept {
        coding: vec![FhirCoding {
            system: Some(TYPE_SYSTEM.to_string()),
            code: Some(label.clone()),
            display: Some(label.clone()),
        }],
        text: Some(label),
    }
}

/// Recover a [`PlaceType`] from a `Location.type` `CodeableConcept`, reading
/// the first coding's `code` (falling back to `text`). Known variant names
/// map to their unit variant; anything else becomes `Other(code)`.
fn concept_to_place_type(concept: &FhirCodeableConcept) -> Option<PlaceType> {
    let code = concept
        .coding
        .first()
        .and_then(|c| c.code.clone())
        .or_else(|| concept.text.clone())?;
    Some(code_to_place_type(&code))
}

/// Map a place-type code string to a [`PlaceType`] variant (the inverse of
/// [`PlaceType`]'s `Display`).
fn code_to_place_type(code: &str) -> PlaceType {
    match code {
        "LocalBusiness" => PlaceType::LocalBusiness,
        "CivicStructure" => PlaceType::CivicStructure,
        "AdministrativeArea" => PlaceType::AdministrativeArea,
        "Landform" => PlaceType::Landform,
        "Park" => PlaceType::Park,
        "Airport" => PlaceType::Airport,
        "Hospital" => PlaceType::Hospital,
        "School" => PlaceType::School,
        "Library" => PlaceType::Library,
        "Museum" => PlaceType::Museum,
        "Restaurant" => PlaceType::Restaurant,
        "Hotel" => PlaceType::Hotel,
        other => PlaceType::Other(other.to_string()),
    }
}

/// Map a domain [`PostalAddress`] to a FHIR `Address` (splitting
/// `street_address` on newlines into the `line` array).
fn to_fhir_address(addr: &PostalAddress) -> FhirAddress {
    let line = addr
        .street_address
        .as_deref()
        .map(|s| s.lines().map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    FhirAddress {
        line,
        city: addr.address_locality.clone(),
        state: addr.address_region.clone(),
        postal_code: addr.postal_code.clone(),
        country: addr.address_country.clone(),
    }
}

/// Map a FHIR `Address` back to a domain [`PostalAddress`] (joining the
/// `line` array with newlines).
fn from_fhir_address(addr: &FhirAddress) -> PostalAddress {
    PostalAddress {
        street_address: (!addr.line.is_empty()).then(|| addr.line.join("\n")),
        address_locality: addr.city.clone(),
        address_region: addr.state.clone(),
        address_country: addr.country.clone(),
        postal_code: addr.postal_code.clone(),
    }
}

/// Render a stored [`Place`] as a FHIR R5 [`FhirLocation`].
///
/// `id` is the record's UUID; `status` is `active`/`inactive` from the
/// soft-delete flag; `last_updated` comes from the row. `identifier`
/// carries the scalar `global_location_number` (GLN system) and
/// `branch_code` (branch system) first, then every [`PlaceIdentifier`] in
/// `identifiers` (scheme → system). `telecom` carries `telephone`/
/// `fax_number`/`url`; `position` carries geo longitude/latitude/altitude;
/// `type` carries the place type.
///
/// **Fidelity gaps** (no FHIR `Location` home): `keywords`,
/// `amenity_features`, `opening_hours`, `contained_in_place`,
/// `is_accessible_for_free`, `public_access`, `smoking_allowed`, and
/// `maximum_attendee_capacity` are not emitted (documented gaps).
#[must_use]
pub fn to_fhir_location(place: &Place, last_updated: Option<String>) -> FhirLocation {
    let mut fhir = FhirLocation::new();
    fhir.id = Some(place.id.to_string());
    fhir.status = Some(if place.is_deleted { "inactive" } else { "active" }.to_string());
    fhir.meta = last_updated.map(|lu| FhirMeta {
        version_id: None,
        last_updated: Some(lu),
    });
    fhir.name = Some(place.name.clone());
    if let Some(ref alt) = place.alternate_name {
        fhir.alias = vec![alt.clone()];
    }
    fhir.description.clone_from(&place.description);

    let mut identifier = Vec::new();
    if let Some(ref gln) = place.global_location_number {
        identifier.push(FhirIdentifier {
            system: Some(GLN_SYSTEM.to_string()),
            value: Some(gln.clone()),
        });
    }
    if let Some(ref branch) = place.branch_code {
        identifier.push(FhirIdentifier {
            system: Some(BRANCH_SYSTEM.to_string()),
            value: Some(branch.clone()),
        });
    }
    for id in &place.identifiers {
        identifier.push(FhirIdentifier {
            system: Some(identifier_type_to_system(&id.identifier_type)),
            value: Some(id.value.clone()),
        });
    }
    fhir.identifier = identifier;

    if let Some(ref pt) = place.place_type {
        fhir.type_ = vec![place_type_to_concept(pt)];
    }

    let mut telecom = Vec::new();
    if let Some(ref phone) = place.telephone {
        telecom.push(FhirContactPoint {
            system: Some("phone".to_string()),
            value: Some(phone.clone()),
        });
    }
    if let Some(ref fax) = place.fax_number {
        telecom.push(FhirContactPoint {
            system: Some("fax".to_string()),
            value: Some(fax.clone()),
        });
    }
    if let Some(ref url) = place.url {
        telecom.push(FhirContactPoint {
            system: Some("url".to_string()),
            value: Some(url.clone()),
        });
    }
    fhir.telecom = telecom;

    if let Some(ref addr) = place.address {
        fhir.address = Some(to_fhir_address(addr));
    }

    if let Some(ref geo) = place.geo {
        fhir.position = Some(FhirPosition {
            longitude: geo.longitude,
            latitude: geo.latitude,
            altitude: geo.elevation,
        });
    }

    fhir
}

/// Parse an inbound [`FhirLocation`] into a stored [`Place`].
///
/// `name` is required (a FHIR `Location` with no `name` is a `400`).
/// `alias` (first) → `alternate_name`; `telecom` phone/fax/url → the scalar
/// fields; `position` → `geo`; the first `type` concept → `place_type`.
/// `identifier` is routed by system: the GLN system → the scalar
/// `global_location_number`, the branch system → `branch_code`, and every
/// other system → the `identifiers` vec (system → scheme).
///
/// **Fidelity gaps** (the DTO field has no FHIR `Location` home, or the
/// reverse is ambiguous): only the first `alias` is kept as
/// `alternate_name`; only the first `type` concept is read; a
/// `PlaceType::Other("Park")` round-trips to the unit `Park`; and the
/// FHIR-absent fields listed in [`to_fhir_location`] default to empty.
///
/// # Errors
///
/// Returns the missing-`name` diagnostic string when the resource has no
/// non-empty `name`.
pub fn from_fhir_location(fhir: &FhirLocation) -> Result<Place, String> {
    let name = fhir
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| "Location.name is required".to_string())?;

    let mut place = Place::new(name);
    place.alternate_name = fhir.alias.first().cloned();
    place.description.clone_from(&fhir.description);

    for id in &fhir.identifier {
        let Some(value) = id.value.clone() else {
            continue;
        };
        match id.system.as_deref() {
            Some(GLN_SYSTEM) => place.global_location_number = Some(value),
            Some(BRANCH_SYSTEM) => place.branch_code = Some(value),
            Some(system) => place.identifiers.push(PlaceIdentifier {
                identifier_type: system_to_identifier_type(system),
                value,
            }),
            None => place.identifiers.push(PlaceIdentifier {
                identifier_type: IdentifierType::Custom(String::new()),
                value,
            }),
        }
    }

    place.place_type = fhir.type_.first().and_then(concept_to_place_type);

    for cp in &fhir.telecom {
        let (Some(system), Some(value)) = (cp.system.as_deref(), cp.value.clone()) else {
            continue;
        };
        match system {
            "phone" => place.telephone = Some(value),
            "fax" => place.fax_number = Some(value),
            "url" => place.url = Some(value),
            _ => {}
        }
    }

    if let Some(addr) = fhir.address.as_ref() {
        place.address = Some(from_fhir_address(addr));
    }

    if let Some(pos) = fhir.position.as_ref() {
        place.geo = Some(GeoCoordinates {
            latitude: pos.latitude,
            longitude: pos.longitude,
            elevation: pos.altitude,
        });
    }

    Ok(place)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every identifier scheme round-trips through the FHIR `system` URI
    /// unchanged (`type → system → type` is the identity).
    #[test]
    fn identifier_type_system_round_trips() {
        let kinds = [
            IdentifierType::GlobalLocationNumber,
            IdentifierType::BranchCode,
            IdentifierType::Fips,
            IdentifierType::Gnis,
            IdentifierType::OpenStreetMap,
            IdentifierType::Custom("iata".to_string()),
        ];
        for kind in kinds {
            let system = identifier_type_to_system(&kind);
            assert_eq!(system_to_identifier_type(&system), kind, "round-trip {kind:?}");
        }
    }

    /// An unknown inbound `system` is preserved as `Custom(system)`, never
    /// dropped.
    #[test]
    fn unknown_system_becomes_custom() {
        assert_eq!(
            system_to_identifier_type("https://example.org/foo"),
            IdentifierType::Custom("https://example.org/foo".to_string())
        );
    }

    /// The losslessly-reversible fields survive `DTO → FHIR → DTO`.
    #[test]
    fn dto_fhir_round_trip_preserves_core_fields() {
        let mut place = Place::new("Central Park");
        place.alternate_name = Some("The Park".to_string());
        place.description = Some("Urban park".to_string());
        place.global_location_number = Some("1234567890123".to_string());
        place.branch_code = Some("NYC-01".to_string());
        place.place_type = Some(PlaceType::Park);
        place.identifiers = vec![
            PlaceIdentifier::new(IdentifierType::Fips, "36061"),
            PlaceIdentifier::new(IdentifierType::OpenStreetMap, "R12345"),
        ];
        place.telephone = Some("+1-212-310-6600".to_string());
        place.fax_number = Some("+1-212-310-6601".to_string());
        place.url = Some("https://www.centralparknyc.org".to_string());
        place.address = Some(PostalAddress {
            street_address: Some("14 E 60th St".to_string()),
            address_locality: Some("New York".to_string()),
            address_region: Some("NY".to_string()),
            address_country: Some("US".to_string()),
            postal_code: Some("10022".to_string()),
        });
        place.geo = Some(GeoCoordinates {
            latitude: 40.7829,
            longitude: -73.9654,
            elevation: Some(10.0),
        });

        let fhir = to_fhir_location(&place, None);
        assert_eq!(fhir.resource_type, "Location");
        assert_eq!(fhir.id.as_deref(), Some(place.id.to_string().as_str()));
        assert_eq!(fhir.status.as_deref(), Some("active"));

        let back = from_fhir_location(&fhir).expect("valid resource");
        assert_eq!(back.name, place.name);
        assert_eq!(back.alternate_name, place.alternate_name);
        assert_eq!(back.description, place.description);
        assert_eq!(back.global_location_number, place.global_location_number);
        assert_eq!(back.branch_code, place.branch_code);
        assert_eq!(back.place_type, place.place_type);
        assert_eq!(back.identifiers, place.identifiers);
        assert_eq!(back.telephone, place.telephone);
        assert_eq!(back.fax_number, place.fax_number);
        assert_eq!(back.url, place.url);
        assert_eq!(back.address, place.address);
        assert_eq!(back.geo, place.geo);
    }

    /// A soft-deleted place renders `status: "inactive"`.
    #[test]
    fn soft_deleted_place_is_inactive() {
        let mut place = Place::new("Closed Site");
        place.soft_delete();
        let fhir = to_fhir_location(&place, None);
        assert_eq!(fhir.status.as_deref(), Some("inactive"));
    }

    /// A resource with no `name` is rejected (maps to a `400`).
    #[test]
    fn missing_name_is_rejected() {
        let fhir = FhirLocation::new();
        assert!(from_fhir_location(&fhir).is_err());
    }
}
