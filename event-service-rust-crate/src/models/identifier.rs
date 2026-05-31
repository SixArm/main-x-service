//! External identifiers for events.
//!
//! An [`Identifier`] is keyed by `(identifier_type, system, value)` —
//! the system is a URI namespace (e.g. an issuing-system OID) and the
//! value is the identifier itself.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// An external identifier issued by some other system.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Identifier {
    pub use_type: Option<IdentifierUse>,
    pub identifier_type: IdentifierType,
    /// URI naming the issuing system (e.g. "urn:oid:1.2.840.…").
    pub system: String,
    pub value: String,
    /// Free-text name of the issuing authority, if not captured in `system`.
    pub assigner: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierUse {
    Usual,
    Official,
    Temp,
    Secondary,
    Old,
}

/// Categories of event identifiers.
///
/// `BookingNumber`, `ConfirmationCode`, and `TicketNumber` are the
/// common reservation/ticketing forms. `EncounterId` is the clinical
/// encounter number. `TransactionId` covers sales / payment refs.
/// `ExternalRef` is the catch-all for opaque IDs from other systems.
/// `TAX` is reserved for billable events that carry a tax/invoice ref.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "UPPERCASE")]
pub enum IdentifierType {
    /// Reservation / booking number (hotels, restaurants, …).
    BookingNumber,
    /// Confirmation code (typically alphanumeric, human-readable).
    ConfirmationCode,
    /// Ticket number (specific ticket within a sale).
    TicketNumber,
    /// Healthcare encounter ID (Encounter resource).
    EncounterId,
    /// Sale / payment / order transaction ID.
    TransactionId,
    /// Opaque external system reference.
    ExternalRef,
    /// Tax / invoice reference for billable events.
    #[serde(rename = "TAX")]
    Tax,
    /// Any other identifier type.
    #[serde(other)]
    Other,
}

impl std::fmt::Display for IdentifierType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IdentifierType::BookingNumber => "BOOKING_NUMBER",
            IdentifierType::ConfirmationCode => "CONFIRMATION_CODE",
            IdentifierType::TicketNumber => "TICKET_NUMBER",
            IdentifierType::EncounterId => "ENCOUNTER_ID",
            IdentifierType::TransactionId => "TRANSACTION_ID",
            IdentifierType::ExternalRef => "EXTERNAL_REF",
            IdentifierType::Tax => "TAX",
            IdentifierType::Other => "OTHER",
        };
        f.write_str(s)
    }
}

impl Identifier {
    /// Construct an identifier with the given type, system, and value.
    pub fn new(identifier_type: IdentifierType, system: String, value: String) -> Self {
        Self {
            use_type: None,
            identifier_type,
            system,
            value,
            assigner: None,
        }
    }

    /// Convenience: build a confirmation-code identifier under a given system.
    pub fn confirmation_code(system: String, value: String) -> Self {
        Self::new(IdentifierType::ConfirmationCode, system, value)
    }

    /// Convenience: build a booking-number identifier under a given system.
    pub fn booking_number(system: String, value: String) -> Self {
        Self::new(IdentifierType::BookingNumber, system, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format() {
        assert_eq!(IdentifierType::BookingNumber.to_string(), "BOOKING_NUMBER");
        assert_eq!(IdentifierType::Tax.to_string(), "TAX");
        assert_eq!(IdentifierType::Other.to_string(), "OTHER");
    }

    #[test]
    fn roundtrip_unknown_falls_back_to_other() {
        let id: Identifier = serde_json::from_str(
            r#"{"identifier_type":"NEWLY_INVENTED","system":"s","value":"v"}"#,
        )
        .unwrap();
        assert_eq!(id.identifier_type, IdentifierType::Other);
    }
}
