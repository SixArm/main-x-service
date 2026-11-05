//! HL7 FHIR R5 interop for the Person resource.
//!
//! Provides the bidirectional mapping between the internal domain
//! [`Person`](crate::models::Person) and the wire-level [`FhirPerson`](crate::api::fhir::FhirPerson): [`to_fhir_person`](crate::api::fhir::to_fhir_person) for
//! outbound responses and [`from_fhir_person`](crate::api::fhir::from_fhir_person) for inbound requests. The
//! FHIR resource shapes live in [`resources`](crate::api::fhir::resources), bundle handling in
//! [`bundle`](crate::api::fhir::bundle), search-parameter parsing in [`search_parameters`](crate::api::fhir::search_parameters), and
//! the Axum endpoints in [`handlers`](crate::api::fhir::handlers). Per
//! `agents/share/fhir.md`'s "lossless where the model allows, explicit
//! where it does not" principle, [`from_fhir_person`](crate::api::fhir::from_fhir_person) parses every
//! field FHIR carries an equivalent for (name array, marital status,
//! multiple-birth, managing-organization reference, identifier-type
//! coding); the one field it cannot parse with confidence —
//! `managingOrganization` present but not a literal
//! `"Organization/<uuid>"` reference — is rejected with an
//! `OperationOutcome` rather than silently dropped (see
//! [`parse_managing_organization`]). See `spec/14-implementation-status.md`
//! §14.2 for the remaining, genuine model gap (multiple-birth order).

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

/// Parse one FHIR `HumanName` entry into a domain [`crate::models::HumanName`].
///
/// Shared by the primary-name parse and the `additional_names` parse in
/// [`from_fhir_person`], so a FHIR `Patient.name[]` entry beyond the
/// first round-trips through the same rules as the first.
fn human_name_from_fhir(fname: &resources::FhirHumanName) -> crate::models::HumanName {
    use crate::models::{HumanName, NameUse};
    HumanName {
        use_type: fname.use_.as_ref().and_then(|u| match u.as_str() {
            "usual" => Some(NameUse::Usual),
            "official" => Some(NameUse::Official),
            "temp" => Some(NameUse::Temp),
            "nickname" => Some(NameUse::Nickname),
            "anonymous" => Some(NameUse::Anonymous),
            "old" => Some(NameUse::Old),
            "maiden" => Some(NameUse::Maiden),
            _ => None,
        }),
        family: fname.family.clone().unwrap_or_default(),
        given: fname.given.clone().unwrap_or_default(),
        prefix: fname.prefix.clone().unwrap_or_default(),
        suffix: fname.suffix.clone().unwrap_or_default(),
    }
}

/// Parse a FHIR `Identifier.type` `CodeableConcept` back into the domain
/// [`crate::models::IdentifierType`].
///
/// Prefers the first coding's `code` (the shape [`fhir_identifiers`]
/// writes: `code` == the type's [`Display`](std::fmt::Display) form,
/// e.g. `"MRN"`), falling back to the concept's free-text `text` for a
/// resource built by hand. An unrecognised or absent code maps to
/// [`IdentifierType::Other`](crate::models::IdentifierType::Other) —
/// consistent with that type's own documented "unknown deserializes to
/// `Other` rather than failing" contract, not a silent drop.
fn identifier_type_from_fhir(fid: &resources::FhirIdentifier) -> crate::models::IdentifierType {
    use crate::models::IdentifierType;
    let code = fid.type_.as_ref().and_then(|tc| {
        tc.coding
            .as_ref()
            .and_then(|codings| codings.first())
            .and_then(|c| c.code.clone())
            .or_else(|| tc.text.clone())
    });
    code.and_then(|c| serde_json::from_value(serde_json::Value::String(c)).ok())
        .unwrap_or(IdentifierType::Other)
}

/// Parse a FHIR `Patient.maritalStatus` `CodeableConcept` back into the
/// domain's free-text/coded `marital_status` string.
///
/// Prefers the first coding's `code` (the shape [`to_fhir_patient`]
/// writes — `code` == the original domain string), falling back to the
/// concept's `text` for a resource built by hand.
fn parse_fhir_marital_status(cc: Option<&resources::FhirCodeableConcept>) -> Option<String> {
    cc.and_then(|c| {
        c.coding
            .as_ref()
            .and_then(|codings| codings.first())
            .and_then(|coding| coding.code.clone())
            .or_else(|| c.text.clone())
    })
}

/// Parse a FHIR `multipleBirth[x]` choice back into the domain's
/// `Option<bool>` `multiple_birth` flag.
///
/// `multipleBirthBoolean` maps directly. `multipleBirthInteger` (birth
/// order, `1..`, per the FHIR `positiveInt` constraint on that element)
/// has no domain field to carry the order in — but its mere presence is
/// unambiguous evidence the person *was* part of a multiple birth, so it
/// maps to `Some(true)` rather than being dropped. The order itself is a
/// genuine, documented model gap (spec §14.2), not a parsing ambiguity.
fn parse_fhir_multiple_birth(mb: Option<&resources::FhirMultipleBirth>) -> Option<bool> {
    use resources::FhirMultipleBirth;
    match mb {
        Some(FhirMultipleBirth::Boolean(b)) => Some(*b),
        Some(FhirMultipleBirth::Integer(_)) => Some(true),
        None => None,
    }
}

/// Parse a FHIR `Patient.managingOrganization` `Reference` back into the
/// domain's `Option<Uuid>` `managing_organization` field.
///
/// The only shape [`to_fhir_patient`] ever writes is a literal
/// `"Organization/<uuid>"` reference, so that is the only shape parsed
/// with confidence. A reference present but **not** in that shape (a
/// display-only reference with no literal `reference`, a reference to a
/// different resource type, or a malformed UUID) is **rejected**
/// (`crate::Error::Validation`, surfaced as a `400 invalid`
/// `OperationOutcome` by the handlers) rather than silently discarded —
/// per `agents/share/fhir.md`'s "explicit where [the model] does not
/// [carry the data]" principle.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] when `reference` is present but
/// is not a well-formed `"Organization/<uuid>"` literal reference.
fn parse_managing_organization(
    reference: Option<&resources::FhirReference>,
) -> Result<Option<uuid::Uuid>> {
    let Some(r) = reference else {
        return Ok(None);
    };
    let Some(ref_str) = r.reference.as_deref() else {
        return Err(crate::Error::Validation(
            "managingOrganization must carry a literal `reference` (e.g. \
             \"Organization/<uuid>\"); a display-only reference cannot be represented"
                .to_string(),
        ));
    };
    let Some(id_str) = ref_str.strip_prefix("Organization/") else {
        return Err(crate::Error::Validation(format!(
            "managingOrganization.reference must be of the form \
             \"Organization/<uuid>\", got: {ref_str}"
        )));
    };
    uuid::Uuid::parse_str(id_str).map(Some).map_err(|e| {
        crate::Error::Validation(format!(
            "managingOrganization.reference has an invalid UUID: {e}"
        ))
    })
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
/// Parses the id (generating a fresh UUID when absent), the name array
/// (the first entry required — errors otherwise — with any further
/// entries mapped into `additional_names`), gender, birth date, deceased
/// flag, identifiers (including identifier-type coding), addresses,
/// telecom, marital status, multiple-birth, and the managing-organization
/// reference.
///
/// # Errors
///
/// Returns [`crate::Error::Validation`] on an invalid UUID, when no name
/// entry is present, or when `managingOrganization` is present but is
/// not a well-formed `"Organization/<uuid>"` literal reference (see
/// [`parse_managing_organization`]).
pub fn from_fhir_person(fhir_person: &FhirPerson) -> Result<Person> {
    use crate::models::Gender;
    use chrono::Utc;
    use uuid::Uuid;

    // Parse ID
    let id = if let Some(ref id_str) = fhir_person.id {
        Uuid::parse_str(id_str)
            .map_err(|e| crate::Error::Validation(format!("Invalid UUID: {e}")))?
    } else {
        Uuid::new_v4()
    };

    // Parse names: the first is the primary name, any further entries
    // are additional names/aliases (FHIR `Patient.name[]` beyond index 0).
    let names = fhir_person
        .name
        .as_ref()
        .filter(|names| !names.is_empty())
        .ok_or_else(|| {
            crate::Error::Validation("Person must have at least one name".to_string())
        })?;
    let name = human_name_from_fhir(&names[0]);
    let additional_names = names[1..].iter().map(human_name_from_fhir).collect();

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

    // Parse identifiers, including the identifier-type coding.
    let identifiers = if let Some(ref ids) = fhir_person.identifier {
        ids.iter()
            .filter_map(|fid| {
                Some(Identifier::new(
                    identifier_type_from_fhir(fid),
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

    // Parse marital status, multiple birth, and the managing-organization
    // reference (the latter fallible — see `parse_managing_organization`).
    let marital_status = parse_fhir_marital_status(fhir_person.marital_status.as_ref());
    let multiple_birth = parse_fhir_multiple_birth(fhir_person.multiple_birth.as_ref());
    let managing_organization =
        parse_managing_organization(fhir_person.managing_organization.as_ref())?;

    Ok(Person {
        id,
        identifiers,
        active: fhir_person.active.unwrap_or(true),
        name,
        additional_names,
        telecom,
        gender,
        birth_date,
        deceased,
        deceased_datetime,
        addresses,
        tax_id: None,
        documents: vec![],
        emergency_contacts: vec![],
        marital_status,
        multiple_birth,
        photo: vec![],
        managing_organization,
        links: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}
