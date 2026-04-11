//! HL7 FHIR R5 API implementation

use crate::models::{Event, Address, ContactPoint, Identifier};
use crate::Result;

pub mod resources;
pub mod bundle;
pub mod search_parameters;
pub mod handlers;

pub use resources::{FhirEvent, FhirOperationOutcome};

/// Convert internal Event model to FHIR Event resource
pub fn to_fhir_event(event: &Event) -> FhirEvent {
    use resources::*;

    let mut fhir_event = FhirEvent::new();

    // Basic fields
    fhir_event.id = Some(event.id.to_string());
    fhir_event.active = Some(event.active);

    // Meta
    fhir_event.meta = Some(FhirMeta {
        version_id: None,
        last_updated: Some(event.updated_at.to_rfc3339()),
    });

    // Identifiers
    if !event.identifiers.is_empty() {
        fhir_event.identifier = Some(
            event
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
        use_: event.name.use_type.as_ref().map(|u| format!("{:?}", u).to_lowercase()),
        text: Some(event.full_name()),
        family: Some(event.name.family.clone()),
        given: if event.name.given.is_empty() {
            None
        } else {
            Some(event.name.given.clone())
        },
        prefix: if event.name.prefix.is_empty() {
            None
        } else {
            Some(event.name.prefix.clone())
        },
        suffix: if event.name.suffix.is_empty() {
            None
        } else {
            Some(event.name.suffix.clone())
        },
    }];

    // Additional names
    for add_name in &event.additional_names {
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
    fhir_event.name = Some(names);

    // Telecom
    if !event.telecom.is_empty() {
        fhir_event.telecom = Some(
            event
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
    fhir_event.gender = Some(format!("{:?}", event.gender).to_lowercase());

    // Birth date
    fhir_event.birth_date = event.birth_date.map(|d| d.to_string());

    // Deceased
    if event.deceased {
        fhir_event.deceased = Some(if let Some(dt) = event.deceased_datetime {
            FhirDeceased::DateTime(dt.to_rfc3339())
        } else {
            FhirDeceased::Boolean(true)
        });
    }

    // Addresses
    if !event.addresses.is_empty() {
        fhir_event.address = Some(
            event
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
    if let Some(ref status) = event.marital_status {
        fhir_event.marital_status = Some(FhirCodeableConcept {
            coding: Some(vec![FhirCoding {
                system: Some("http://terminology.hl7.org/CodeSystem/v3-MaritalStatus".to_string()),
                code: Some(status.clone()),
                display: Some(status.clone()),
            }]),
            text: Some(status.clone()),
        });
    }

    // Multiple birth
    if let Some(mb) = event.multiple_birth {
        fhir_event.multiple_birth = Some(FhirMultipleBirth::Boolean(mb));
    }

    // Links
    if !event.links.is_empty() {
        fhir_event.link = Some(
            event
                .links
                .iter()
                .map(|link| FhirEventLink {
                    other: FhirReference {
                        reference: Some(format!("Event/{}", link.other_event_id)),
                        display: None,
                    },
                    type_: format!("{:?}", link.link_type).to_lowercase(),
                })
                .collect(),
        );
    }

    // Managing organization
    if let Some(ref org_id) = event.managing_organization {
        fhir_event.managing_organization = Some(FhirReference {
            reference: Some(format!("Organization/{}", org_id)),
            display: None,
        });
    }

    fhir_event
}

/// Convert FHIR Event resource to internal Event model
pub fn from_fhir_event(fhir_event: &FhirEvent) -> Result<Event> {
    use crate::models::{HumanName, NameUse, Gender, ContactPointSystem, ContactPointUse};
    use crate::api::fhir::resources::FhirDeceased;
    use uuid::Uuid;
    use chrono::Utc;

    // Parse ID
    let id = if let Some(ref id_str) = fhir_event.id {
        Uuid::parse_str(id_str).map_err(|e| crate::Error::Validation(format!("Invalid UUID: {}", e)))?
    } else {
        Uuid::new_v4()
    };

    // Parse name (use first name)
    let name = if let Some(ref names) = fhir_event.name {
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
            return Err(crate::Error::Validation("Event must have at least one name".to_string()));
        }
    } else {
        return Err(crate::Error::Validation("Event must have at least one name".to_string()));
    };

    // Parse gender
    let gender = if let Some(ref g) = fhir_event.gender {
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
    let birth_date = fhir_event.birth_date.as_ref().and_then(|d| {
        chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
    });

    // Parse deceased
    let (deceased, deceased_datetime) = match &fhir_event.deceased {
        Some(FhirDeceased::Boolean(b)) => (*b, None),
        Some(FhirDeceased::DateTime(dt)) => {
            let parsed_dt = chrono::DateTime::parse_from_rfc3339(dt).ok()
                .map(|d| d.with_timezone(&Utc));
            (true, parsed_dt)
        }
        None => (false, None),
    };

    // Parse identifiers
    let identifiers = if let Some(ref ids) = fhir_event.identifier {
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
    let addresses = if let Some(ref addrs) = fhir_event.address {
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
    let telecom = if let Some(ref tels) = fhir_event.telecom {
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

    Ok(Event {
        id,
        identifiers,
        active: fhir_event.active.unwrap_or(true),
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
