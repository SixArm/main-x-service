//! HL7 FHIR R5 interop for the Person resource.
//!
//! Provides the bidirectional mapping between the internal domain
//! [`Person`](crate::models::Person) and the wire-level [`FhirPerson`](crate::api::fhir::FhirPerson): [`to_fhir_person`](crate::api::fhir::to_fhir_person) for
//! outbound responses and [`from_fhir_person`](crate::api::fhir::from_fhir_person) for inbound requests. The
//! FHIR resource shapes live in [`resources`](crate::api::fhir::resources), bundle handling in
//! [`bundle`](crate::api::fhir::bundle), search-parameter parsing in [`search_parameters`](crate::api::fhir::search_parameters), and
//! the Axum endpoints in [`handlers`](crate::api::fhir::handlers). The conversions are intentionally
//! lossy where the domain model has no equivalent field (noted by inline
//! `TODO`s).

use crate::Result;
use crate::models::{Address, Identifier, Person};

/// FHIR Bundle (search-set) construction.
pub mod bundle;
/// FHIR endpoint handlers.
pub mod handlers;
/// FHIR resource type definitions (Person, Identifier, …).
pub mod resources;
/// FHIR search-parameter parsing.
pub mod search_parameters;

pub use resources::{FhirOperationOutcome, FhirPerson};

/// Map a domain [`Address`] to a FHIR `Address` (joining `line1`/`line2`
/// into the FHIR `line` array). `use`/`type`/`text` are not modeled.
fn fhir_address(addr: &Address) -> resources::FhirAddress {
    let mut lines = Vec::new();
    if let Some(ref l1) = addr.line1 {
        lines.push(l1.clone());
    }
    if let Some(ref l2) = addr.line2 {
        lines.push(l2.clone());
    }
    resources::FhirAddress {
        use_: None,
        type_: None,
        text: None,
        line: (!lines.is_empty()).then_some(lines),
        city: addr.city.clone(),
        state: addr.state.clone(),
        postal_code: addr.postal_code.clone(),
        country: addr.country.clone(),
    }
}

/// Build the FHIR `identifier` array for a [`Person`].
fn fhir_identifiers(person: &Person) -> Vec<resources::FhirIdentifier> {
    use resources::{FhirCodeableConcept, FhirCoding, FhirIdentifier, FhirReference};
    person
        .identifiers
        .iter()
        .map(|id| FhirIdentifier {
            use_: id
                .use_type
                .as_ref()
                .map(|u| format!("{u:?}").to_lowercase()),
            type_: Some(FhirCodeableConcept {
                coding: Some(vec![FhirCoding {
                    system: Some(id.system.clone()),
                    code: Some(id.identifier_type.to_string()),
                    display: Some(id.identifier_type.to_string()),
                }]),
                text: Some(id.identifier_type.to_string()),
            }),
            system: Some(id.system.clone()),
            value: Some(id.value.clone()),
            assigner: id.assigner.as_ref().map(|a| FhirReference {
                reference: None,
                display: Some(a.clone()),
            }),
        })
        .collect()
}

/// Build the FHIR `name` array for a [`Person`]: the primary name
/// followed by any additional names. Empty given/prefix/suffix vectors
/// map to `None` per FHIR convention.
fn fhir_names(person: &Person) -> Vec<resources::FhirHumanName> {
    use crate::models::HumanName;
    use resources::FhirHumanName;

    let to_fhir = |name: &HumanName, text: String| FhirHumanName {
        use_: name
            .use_type
            .as_ref()
            .map(|u| format!("{u:?}").to_lowercase()),
        text: Some(text),
        family: Some(name.family.clone()),
        given: (!name.given.is_empty()).then(|| name.given.clone()),
        prefix: (!name.prefix.is_empty()).then(|| name.prefix.clone()),
        suffix: (!name.suffix.is_empty()).then(|| name.suffix.clone()),
    };

    let mut names = vec![to_fhir(&person.name, person.full_name())];
    names.extend(
        person
            .additional_names
            .iter()
            .map(|n| to_fhir(n, format!("{} {}", n.given.join(" "), n.family))),
    );
    names
}

/// Map an internal [`Person`] to its primary FHIR R5 **`Patient`**
/// resource ([`agents/share/fhir.md`](../../../../agents/share/fhir.md)
/// §3, `high` fidelity).
///
/// Copies identity, names (primary + additional), telecom, gender, birth
/// date, deceased status, addresses, marital status, multiple-birth,
/// links, and managing organization, populating FHIR `Reference`s where
/// the domain holds bare ids. Enum values are lowercased to FHIR codes.
/// The returned resource carries `resourceType: "Patient"`; use
/// [`to_fhir_person`] for the `"Person"` demographic alias.
#[must_use]
pub fn to_fhir_patient(person: &Person) -> FhirPerson {
    use resources::{
        FhirCodeableConcept, FhirCoding, FhirContactPoint, FhirDeceased, FhirMeta,
        FhirMultipleBirth, FhirPerson, FhirPersonLink, FhirReference,
    };

    let mut fhir_person = FhirPerson::new();

    // Basic fields
    fhir_person.id = Some(person.id.to_string());
    fhir_person.active = Some(person.active);

    // Meta
    fhir_person.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(person.updated_at.to_string()),
    });

    // Identifiers
    if !person.identifiers.is_empty() {
        fhir_person.identifier = Some(fhir_identifiers(person));
    }

    // Name (primary + additional)
    fhir_person.name = Some(fhir_names(person));

    // Telecom
    if !person.telecom.is_empty() {
        fhir_person.telecom = Some(
            person
                .telecom
                .iter()
                .map(|cp| FhirContactPoint {
                    system: Some(format!("{:?}", cp.system).to_lowercase()),
                    value: Some(cp.value.clone()),
                    use_: cp
                        .use_type
                        .as_ref()
                        .map(|u| format!("{u:?}").to_lowercase()),
                })
                .collect(),
        );
    }

    // Gender
    fhir_person.gender = Some(format!("{:?}", person.gender).to_lowercase());

    // Birth date
    fhir_person.birth_date = person.birth_date.map(|d| d.to_string());

    // Deceased
    if person.deceased {
        fhir_person.deceased = Some(if let Some(dt) = person.deceased_datetime {
            FhirDeceased::DateTime(dt.to_string())
        } else {
            FhirDeceased::Boolean(true)
        });
    }

    // Addresses
    if !person.addresses.is_empty() {
        fhir_person.address = Some(person.addresses.iter().map(fhir_address).collect());
    }

    // Marital status
    if let Some(ref status) = person.marital_status {
        fhir_person.marital_status = Some(FhirCodeableConcept {
            coding: Some(vec![FhirCoding {
                system: Some("http://terminology.hl7.org/CodeSystem/v3-MaritalStatus".to_string()),
                code: Some(status.clone()),
                display: Some(status.clone()),
            }]),
            text: Some(status.clone()),
        });
    }

    // Multiple birth
    if let Some(mb) = person.multiple_birth {
        fhir_person.multiple_birth = Some(FhirMultipleBirth::Boolean(mb));
    }

    // Links
    if !person.links.is_empty() {
        fhir_person.link = Some(
            person
                .links
                .iter()
                .map(|link| FhirPersonLink {
                    other: FhirReference {
                        reference: Some(format!("Person/{}", link.other_person_id)),
                        display: None,
                    },
                    type_: format!("{:?}", link.link_type).to_lowercase(),
                })
                .collect(),
        );
    }

    // Managing organization
    if let Some(ref org_id) = person.managing_organization {
        fhir_person.managing_organization = Some(FhirReference {
            reference: Some(format!("Organization/{org_id}")),
            display: None,
        });
    }

    fhir_person
}

/// Map an internal [`Person`] to the FHIR **`Person`** demographic alias
/// (`resourceType: "Person"`). Identical field content to
/// [`to_fhir_patient`], differing only in the resource-type discriminator
/// — the non-clinical demographic view backing `/fhir/Person`
/// ([`agents/share/fhir.md`](../../../../agents/share/fhir.md) §3).
#[must_use]
pub fn to_fhir_person(person: &Person) -> FhirPerson {
    let mut resource = to_fhir_patient(person);
    resource.resource_type = "Person".to_string();
    resource
}

/// Parse a FHIR `deceased[x]` choice into the domain
/// `(deceased, deceased_datetime)` pair.
fn parse_fhir_deceased(
    deceased: Option<&resources::FhirDeceased>,
) -> (bool, Option<chrono::DateTime<chrono::Utc>>) {
    use resources::FhirDeceased;
    match deceased {
        Some(FhirDeceased::Boolean(b)) => (*b, None),
        Some(FhirDeceased::DateTime(dt)) => {
            (true, dt.parse::<chrono::DateTime<chrono::Utc>>().ok())
        }
        None => (false, None),
    }
}

/// Map a FHIR `Address` back to a domain [`Address`], splitting the
/// FHIR `line` array into `line1` / `line2`.
fn address_from_fhir(faddr: &resources::FhirAddress) -> Address {
    let lines = faddr.line.clone().unwrap_or_default();
    Address {
        use_type: None,
        line1: lines.first().cloned(),
        line2: lines.get(1).cloned(),
        city: faddr.city.clone(),
        state: faddr.state.clone(),
        postal_code: faddr.postal_code.clone(),
        country: faddr.country.clone(),
    }
}

/// Parse one FHIR `ContactPoint` into the domain [`ContactPoint`](crate::models::ContactPoint),
/// dropping entries with an unknown/absent system or a missing value.
fn parse_fhir_contact_point(
    ftel: &resources::FhirContactPoint,
) -> Option<crate::models::ContactPoint> {
    use crate::models::{ContactPoint, ContactPointSystem, ContactPointUse};
    let system = ftel.system.as_ref().and_then(|s| match s.as_str() {
        "phone" => Some(ContactPointSystem::Phone),
        "fax" => Some(ContactPointSystem::Fax),
        "email" => Some(ContactPointSystem::Email),
        "pager" => Some(ContactPointSystem::Pager),
        "url" => Some(ContactPointSystem::Url),
        "sms" => Some(ContactPointSystem::Sms),
        "other" => Some(ContactPointSystem::Other),
        _ => None,
    })?;
    let value = ftel.value.clone()?;
    Some(ContactPoint {
        system,
        value,
        use_type: ftel.use_.as_ref().and_then(|u| match u.as_str() {
            "home" => Some(ContactPointUse::Home),
            "work" => Some(ContactPointUse::Work),
            "temp" => Some(ContactPointUse::Temp),
            "old" => Some(ContactPointUse::Old),
            "mobile" => Some(ContactPointUse::Mobile),
            _ => None,
        }),
    })
}

/// Map an inbound FHIR [`FhirPerson`] to the internal [`Person`].
///
/// Parses the id (generating a fresh UUID when absent), the first name
/// entry (required — errors otherwise), gender, birth date, deceased
/// flag, identifiers, addresses, and telecom. Fields the domain does not
/// yet round-trip (additional names, marital status, multiple birth,
/// org reference) are left at defaults.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] on an invalid UUID or when no
/// name entry is present.
pub fn from_fhir_person(fhir_person: &FhirPerson) -> Result<Person> {
    use crate::models::{Gender, HumanName, NameUse};
    use chrono::Utc;
    use uuid::Uuid;

    // Parse ID
    let id = if let Some(ref id_str) = fhir_person.id {
        Uuid::parse_str(id_str)
            .map_err(|e| crate::Error::Validation(format!("Invalid UUID: {e}")))?
    } else {
        Uuid::new_v4()
    };

    // Parse name (use first name)
    let first_name = fhir_person
        .name
        .as_ref()
        .and_then(|names| names.first())
        .ok_or_else(|| {
            crate::Error::Validation("Person must have at least one name".to_string())
        })?;
    let name = HumanName {
        use_type: first_name.use_.as_ref().and_then(|u| match u.as_str() {
            "usual" => Some(NameUse::Usual),
            "official" => Some(NameUse::Official),
            "temp" => Some(NameUse::Temp),
            "nickname" => Some(NameUse::Nickname),
            "anonymous" => Some(NameUse::Anonymous),
            "old" => Some(NameUse::Old),
            "maiden" => Some(NameUse::Maiden),
            _ => None,
        }),
        family: first_name.family.clone().unwrap_or_default(),
        given: first_name.given.clone().unwrap_or_default(),
        prefix: first_name.prefix.clone().unwrap_or_default(),
        suffix: first_name.suffix.clone().unwrap_or_default(),
    };

    // Parse gender
    let gender = if let Some(ref g) = fhir_person.gender {
        match g.as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            "other" => Gender::Other,
            _ => Gender::Unknown,
        }
    } else {
        Gender::Unknown
    };

    // Parse birth date
    let birth_date = fhir_person
        .birth_date
        .as_ref()
        .and_then(|d| d.parse::<chrono::NaiveDate>().ok());

    // Parse deceased
    let (deceased, deceased_datetime) = parse_fhir_deceased(fhir_person.deceased.as_ref());

    // Parse identifiers
    let identifiers = if let Some(ref ids) = fhir_person.identifier {
        ids.iter()
            .filter_map(|fid| {
                Some(Identifier::new(
                    crate::models::IdentifierType::Other, // TODO: Parse from coding
                    fid.system.clone()?,
                    fid.value.clone()?,
                ))
            })
            .collect()
    } else {
        vec![]
    };

    // Parse addresses
    let addresses = fhir_person
        .address
        .as_ref()
        .map(|addrs| addrs.iter().map(address_from_fhir).collect())
        .unwrap_or_default();

    // Parse telecom
    let telecom = fhir_person
        .telecom
        .as_ref()
        .map(|tels| tels.iter().filter_map(parse_fhir_contact_point).collect())
        .unwrap_or_default();

    Ok(Person {
        id,
        identifiers,
        active: fhir_person.active.unwrap_or(true),
        name,
        additional_names: vec![], // TODO: Parse additional names from FHIR
        telecom,
        gender,
        birth_date,
        deceased,
        deceased_datetime,
        addresses,
        tax_id: None,
        documents: vec![],
        emergency_contacts: vec![],
        marital_status: None, // TODO: Parse marital status
        multiple_birth: None, // TODO: Parse multiple birth
        photo: vec![],
        managing_organization: None, // TODO: Parse organization reference
        links: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}
