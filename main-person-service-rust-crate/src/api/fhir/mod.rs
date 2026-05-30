//! HL7 FHIR R5 API implementation

use crate::models::{Person, Address, ContactPoint, Identifier};
use crate::Result;

pub mod resources;
pub mod bundle;
pub mod search_parameters;
pub mod handlers;

pub use resources::{FhirPerson, FhirOperationOutcome};

/// Convert internal Person model to FHIR Person resource
pub fn to_fhir_person(person: &Person) -> FhirPerson {
    use resources::*;

    let mut fhir_person = FhirPerson::new();

    // Basic fields
    fhir_person.id = Some(person.id.to_string());
    fhir_person.active = Some(person.active);

    // Meta
    fhir_person.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(person.updated_at.to_rfc3339()),
    });

    // Identifiers
    if !person.identifiers.is_empty() {
        fhir_person.identifier = Some(
            person
                .identifiers
                .iter()
                .map(|id| FhirIdentifier {
                    use_: id.use_type.as_ref().map(|u| format!("{:?}", u).to_lowercase()),
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
                .collect(),
        );
    }

    // Name
    let mut names = vec![FhirHumanName {
        use_: person.name.use_type.as_ref().map(|u| format!("{:?}", u).to_lowercase()),
        text: Some(person.full_name()),
        family: Some(person.name.family.clone()),
        given: if person.name.given.is_empty() {
            None
        } else {
            Some(person.name.given.clone())
        },
        prefix: if person.name.prefix.is_empty() {
            None
        } else {
            Some(person.name.prefix.clone())
        },
        suffix: if person.name.suffix.is_empty() {
            None
        } else {
            Some(person.name.suffix.clone())
        },
    }];

    // Additional names
    for add_name in &person.additional_names {
        names.push(FhirHumanName {
            use_: add_name.use_type.as_ref().map(|u| format!("{:?}", u).to_lowercase()),
            text: Some(format!("{} {}", add_name.given.join(" "), add_name.family)),
            family: Some(add_name.family.clone()),
            given: if add_name.given.is_empty() {
                None
            } else {
                Some(add_name.given.clone())
            },
            prefix: if add_name.prefix.is_empty() {
                None
            } else {
                Some(add_name.prefix.clone())
            },
            suffix: if add_name.suffix.is_empty() {
                None
            } else {
                Some(add_name.suffix.clone())
            },
        });
    }
    fhir_person.name = Some(names);

    // Telecom
    if !person.telecom.is_empty() {
        fhir_person.telecom = Some(
            person
                .telecom
                .iter()
                .map(|cp| FhirContactPoint {
                    system: Some(format!("{:?}", cp.system).to_lowercase()),
                    value: Some(cp.value.clone()),
                    use_: cp.use_type.as_ref().map(|u| format!("{:?}", u).to_lowercase()),
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
            FhirDeceased::DateTime(dt.to_rfc3339())
        } else {
            FhirDeceased::Boolean(true)
        });
    }

    // Addresses
    if !person.addresses.is_empty() {
        fhir_person.address = Some(
            person
                .addresses
                .iter()
                .map(|addr| {
                    let mut lines = Vec::new();
                    if let Some(ref l1) = addr.line1 {
                        lines.push(l1.clone());
                    }
                    if let Some(ref l2) = addr.line2 {
                        lines.push(l2.clone());
                    }

                    FhirAddress {
                        use_: None, // Not stored in our model
                        type_: None, // Not stored in our model
                        text: None, // Not stored in our model
                        line: if lines.is_empty() { None } else { Some(lines) },
                        city: addr.city.clone(),
                        state: addr.state.clone(),
                        postal_code: addr.postal_code.clone(),
                        country: addr.country.clone(),
                    }
                })
                .collect(),
        );
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
            reference: Some(format!("Organization/{}", org_id)),
            display: None,
        });
    }

    fhir_person
}

/// Convert FHIR Person resource to internal Person model
pub fn from_fhir_person(fhir_person: &FhirPerson) -> Result<Person> {
    use crate::models::{HumanName, NameUse, Gender, ContactPointSystem, ContactPointUse};
    use crate::api::fhir::resources::FhirDeceased;
    use uuid::Uuid;
    use chrono::Utc;

    // Parse ID
    let id = if let Some(ref id_str) = fhir_person.id {
        Uuid::parse_str(id_str).map_err(|e| crate::Error::Validation(format!("Invalid UUID: {}", e)))?
    } else {
        Uuid::new_v4()
    };

    // Parse name (use first name)
    let name = if let Some(ref names) = fhir_person.name {
        if let Some(first_name) = names.first() {
            HumanName {
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
            }
        } else {
            return Err(crate::Error::Validation("Person must have at least one name".to_string()));
        }
    } else {
        return Err(crate::Error::Validation("Person must have at least one name".to_string()));
    };

    // Parse gender
    let gender = if let Some(ref g) = fhir_person.gender {
        match g.as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            "other" => Gender::Other,
            "unknown" => Gender::Unknown,
            _ => Gender::Unknown,
        }
    } else {
        Gender::Unknown
    };

    // Parse birth date
    let birth_date = fhir_person.birth_date.as_ref().and_then(|d| {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
    });

    // Parse deceased
    let (deceased, deceased_datetime) = match &fhir_person.deceased {
        Some(FhirDeceased::Boolean(b)) => (*b, None),
        Some(FhirDeceased::DateTime(dt)) => {
            let parsed_dt = chrono::DateTime::parse_from_rfc3339(dt).ok()
                .map(|d| d.with_timezone(&Utc));
            (true, parsed_dt)
        }
        None => (false, None),
    };

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
    let addresses = if let Some(ref addrs) = fhir_person.address {
        addrs.iter()
            .map(|faddr| {
                let lines = faddr.line.clone().unwrap_or_default();
                Address {
                    use_type: None,
                    line1: lines.get(0).cloned(),
                    line2: lines.get(1).cloned(),
                    city: faddr.city.clone(),
                    state: faddr.state.clone(),
                    postal_code: faddr.postal_code.clone(),
                    country: faddr.country.clone(),
                }
            })
            .collect()
    } else {
        vec![]
    };

    // Parse telecom
    let telecom = if let Some(ref tels) = fhir_person.telecom {
        tels.iter()
            .filter_map(|ftel| {
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
            })
            .collect()
    } else {
        vec![]
    };

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
