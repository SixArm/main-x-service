//! External identifiers for events.
//!
//! An [`Identifier`] is keyed by `(identifier_type, system, value)` —
//! the system is a URI namespace (e.g. an issuing-system OID) and the
//! value is the identifier itself. A few [`IdentifierType`] variants
//! are "strong" (booking / confirmation / ticket / encounter /
//! transaction): an exact match on one of those short-circuits the
//! matcher to a perfect score.
//!
//! # Examples
//!
//! ```
//! use event_service::models::{Identifier, IdentifierType};
//!
//! let id = Identifier::confirmation_code("eventbrite".into(), "ABC123".into());
//! assert_eq!(id.identifier_type, IdentifierType::ConfirmationCode);
//! assert_eq!(id.value, "ABC123");
//! ```

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// An external identifier issued by some other system.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Identifier {
    /// How this identifier is used (official / temporary / …).
    pub use_type: Option<IdentifierUse>,
    /// The category of identifier (booking, ticket, encounter, …).
    pub identifier_type: IdentifierType,
    /// URI naming the issuing system (e.g. "urn:oid:1.2.840.…").
    pub system: String,
    /// The identifier value itself, stored verbatim.
    pub value: String,
    /// Free-text name of the issuing authority, if not captured in `system`.
    pub assigner: Option<String>,
}

/// How an [`Identifier`] is used. Mirrors the FHIR identifier-use value set.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum IdentifierUse {
    /// The common, everyday identifier.
    Usual,
    /// The officially registered identifier.
    Official,
    /// A temporary identifier.
    Temp,
    /// A secondary / alternate identifier.
    Secondary,
    /// A former identifier no longer in use.
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
///
/// On the wire each serializes in `UPPERCASE` (`BOOKINGNUMBER`) and is also
/// read in its Rust name (`BookingNumber`, what the front-end sends) and its
/// `Display` form (`BOOKING_NUMBER`). Storage uses the `Display` form, the
/// one the `event_identifiers` CHECK constraint names.
#[serde(rename_all = "UPPERCASE")]
pub enum IdentifierType {
    /// Reservation / booking number (hotels, restaurants, …).
    #[serde(alias = "BookingNumber", alias = "BOOKING_NUMBER")]
    BookingNumber,
    /// Confirmation code (typically alphanumeric, human-readable).
    #[serde(alias = "ConfirmationCode", alias = "CONFIRMATION_CODE")]
    ConfirmationCode,
    /// Ticket number (specific ticket within a sale).
    #[serde(alias = "TicketNumber", alias = "TICKET_NUMBER")]
    TicketNumber,
    /// Healthcare encounter ID (Encounter resource).
    #[serde(alias = "EncounterId", alias = "ENCOUNTER_ID")]
    EncounterId,
    /// Sale / payment / order transaction ID.
    #[serde(alias = "TransactionId", alias = "TRANSACTION_ID")]
    TransactionId,
    /// Opaque external system reference.
    #[serde(alias = "ExternalRef", alias = "EXTERNAL_REF")]
    ExternalRef,
    /// Tax / invoice reference for billable events.
    #[serde(rename = "TAX", alias = "Tax")]
    Tax,
    /// Any other identifier type.
    #[serde(other)]
    Other,
}

impl std::fmt::Display for IdentifierType {
    /// Renders the type as its `SCREAMING_SNAKE_CASE` wire form (e.g.
    /// `BOOKING_NUMBER`). This is distinct from the serde representation
    /// and is used in human-facing strings and the search index.
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

impl IdentifierType {
    /// Every variant, in declaration order.
    pub const ALL: [IdentifierType; 8] = [
        IdentifierType::BookingNumber,
        IdentifierType::ConfirmationCode,
        IdentifierType::TicketNumber,
        IdentifierType::EncounterId,
        IdentifierType::TransactionId,
        IdentifierType::ExternalRef,
        IdentifierType::Tax,
        IdentifierType::Other,
    ];

    /// The variant whose [`Display`](std::fmt::Display) form is `s`, the
    /// form stored in `event_identifiers.identifier_type`.
    #[must_use]
    pub fn from_stored(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.to_string() == s)
    }
}

impl Identifier {
    /// Construct an identifier with the given type, system, and value.
    /// `use_type` and `assigner` are left unset.
    ///
    /// # Examples
    ///
    /// ```
    /// use event_service::models::{Identifier, IdentifierType};
    ///
    /// let id = Identifier::new(IdentifierType::TicketNumber, "sys".into(), "T-1".into());
    /// assert_eq!(id.value, "T-1");
    /// assert!(id.assigner.is_none());
    /// ```
    #[must_use]
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
    #[must_use]
    pub fn confirmation_code(system: String, value: String) -> Self {
        Self::new(IdentifierType::ConfirmationCode, system, value)
    }

    /// Convenience: build a booking-number identifier under a given system.
    #[must_use]
    pub fn booking_number(system: String, value: String) -> Self {
        Self::new(IdentifierType::BookingNumber, system, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Display` renders the `SCREAMING_SNAKE_CASE` wire form per variant.
    #[test]
    fn display_format() {
        assert_eq!(IdentifierType::BookingNumber.to_string(), "BOOKING_NUMBER");
        assert_eq!(IdentifierType::Tax.to_string(), "TAX");
        assert_eq!(IdentifierType::Other.to_string(), "OTHER");
    }

    /// An unrecognized wire value deserializes to `Other` (via
    /// `#[serde(other)]`) rather than failing.
    #[test]
    fn roundtrip_unknown_falls_back_to_other() {
        let id: Identifier = serde_json::from_str(
            r#"{"identifier_type":"NEWLY_INVENTED","system":"s","value":"v"}"#,
        )
        .unwrap();
        assert_eq!(id.identifier_type, IdentifierType::Other);
    }

    /// The stored form round-trips for every variant, and is the form the
    /// `event_identifiers` CHECK constraint lists.
    #[test]
    fn every_identifier_type_round_trips_through_its_stored_form() {
        for t in IdentifierType::ALL {
            assert_eq!(IdentifierType::from_stored(&t.to_string()), Some(t));
        }
        assert_eq!(IdentifierType::BookingNumber.to_string(), "BOOKING_NUMBER");
        assert_eq!(IdentifierType::from_stored("BOOKINGNUMBER"), None);
    }

    /// The front-end sends the Rust name; it used to fall through to
    /// `Other` and lose the type.
    #[test]
    fn identifier_types_are_read_in_all_three_spellings() {
        for (text, want) in [
            ("\"BOOKINGNUMBER\"", IdentifierType::BookingNumber),
            ("\"BookingNumber\"", IdentifierType::BookingNumber),
            ("\"BOOKING_NUMBER\"", IdentifierType::BookingNumber),
            ("\"ExternalRef\"", IdentifierType::ExternalRef),
            ("\"EXTERNAL_REF\"", IdentifierType::ExternalRef),
            ("\"Tax\"", IdentifierType::Tax),
            ("\"TAX\"", IdentifierType::Tax),
            ("\"something else\"", IdentifierType::Other),
        ] {
            let got: IdentifierType = serde_json::from_str(text).unwrap();
            assert_eq!(got, want, "{text}");
        }
        assert_eq!(
            serde_json::to_string(&IdentifierType::BookingNumber).unwrap(),
            "\"BOOKINGNUMBER\"",
            "what the API sends is unchanged"
        );
    }
}
