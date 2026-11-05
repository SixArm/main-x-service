//! Person's declared bulk-import **stable key**
//! (`agents/share/bulk-import-export.md` §10.1).
//!
//! The stable key is what drives idempotent upsert: on import, a row
//! whose stable key matches an existing record updates it in place rather
//! than creating a duplicate, so re-running the same file is a no-op.
//!
//! Person's precedence (most-to-least specific):
//!
//! 1. A **scheme-scoped national/official identifier** — the first
//!    [`Identifier`] of a strong type (SSN, TAX, NPI, PPN) with a
//!    non-empty value. This is the same class of identifier the person
//!    matcher treats as a short-circuit signal.
//! 2. The **`tax_id`** convenience field, under a synthetic system URN,
//!    when no typed strong identifier is present.
//! 3. The record **`pid`** (`Person::id`) as the always-available
//!    fallback — this is what makes re-importing an *export* (which
//!    carries pids) idempotent.

use crate::models::{IdentifierType, Person};
use uuid::Uuid;

/// Synthetic identifier system for the person `tax_id` convenience field
/// when it is promoted to a stable key.
pub const TAX_ID_SYSTEM: &str = "urn:mxi:person:tax_id";

/// A resolved stable key for one import row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StableKey {
    /// Match on the record's own pid.
    Pid(Uuid),
    /// Match on a scheme-scoped identifier `(system, value)`.
    Identifier {
        /// Issuing-namespace URI.
        system: String,
        /// The identifier value.
        value: String,
    },
}

/// Identifier types treated as strong, scheme-scoped upsert keys, in
/// preference order.
const STRONG_TYPES: [IdentifierType; 4] = [
    IdentifierType::SSN,
    IdentifierType::TAX,
    IdentifierType::NPI,
    IdentifierType::PPN,
];

/// Resolve the stable key for `person` per the precedence above.
///
/// Always returns a key: the `pid` fallback guarantees one exists even
/// for a record with no identifiers.
#[must_use]
pub fn resolve_stable_key(person: &Person) -> StableKey {
    for strong in STRONG_TYPES {
        if let Some(id) = person
            .identifiers
            .iter()
            .find(|i| i.identifier_type == strong && !i.value.trim().is_empty())
        {
            return StableKey::Identifier {
                system: id.system.clone(),
                value: id.value.clone(),
            };
        }
    }

    if let Some(tax) = person
        .tax_id
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return StableKey::Identifier {
            system: TAX_ID_SYSTEM.to_string(),
            value: tax.to_string(),
        };
    }

    StableKey::Pid(person.id)
}

/// Whether a raw JSONL row carried an **explicit, non-null `id`**.
///
/// `Person::id` deserializes with `#[serde(default = "Uuid::new_v4")]`, so
/// after parsing there is no way to tell "this row named its own pid" from
/// "no id was given, and parsing handed it a random one" — [`Person`]
/// alone has already lost that distinction. This answers the question
/// from the wire bytes, before the `Person` exists, so [`is_keyless`] can
/// tell a real upsert target from `resolve_stable_key`'s placeholder
/// fallback. Malformed JSON reads as "no explicit id" — the row will fail
/// to parse into a `Person` moments later and land in the error report.
///
/// The CSV codec answers the equivalent question directly from its own
/// column cell ([`crate::bulk::csv::decode`]'s `had_explicit_id`), since a
/// CSV row has no single JSON text to inspect.
#[must_use]
pub fn row_has_explicit_id(raw_line: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(raw_line)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .is_some_and(|id| !id.is_null())
}

/// Whether `person`'s row is **keyless** (`agents/share/bulk-import-export.md`
/// §6): no strong identifier, no `tax_id`, and no explicit `id` of its own.
///
/// A keyless row cannot be idempotently upserted — [`resolve_stable_key`]'s
/// [`StableKey::Pid`] fallback is only a freshly-generated placeholder, not
/// a real record to find — so the import pipeline routes it through
/// duplicate detection instead of a blind create.
#[must_use]
pub fn is_keyless(person: &Person, row_has_explicit_id: bool) -> bool {
    match resolve_stable_key(person) {
        StableKey::Identifier { .. } => false,
        StableKey::Pid(_) => !row_has_explicit_id,
    }
}

#[cfg(test)]
mod tests {
    use super::{StableKey, TAX_ID_SYSTEM, is_keyless, resolve_stable_key, row_has_explicit_id};
    use crate::models::{Gender, HumanName, Identifier, IdentifierType, Person};

    fn base() -> Person {
        Person::new(
            HumanName {
                use_type: None,
                family: "Doe".to_string(),
                given: vec!["Jane".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Female,
        )
    }

    #[test]
    fn prefers_strong_identifier() {
        let mut p = base();
        p.tax_id = Some("TAX-999".to_string());
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        ));
        assert_eq!(
            resolve_stable_key(&p),
            StableKey::Identifier {
                system: "http://hl7.org/fhir/sid/us-ssn".to_string(),
                value: "123-45-6789".to_string(),
            }
        );
    }

    #[test]
    fn falls_back_to_tax_id() {
        let mut p = base();
        p.tax_id = Some("TAX-999".to_string());
        assert_eq!(
            resolve_stable_key(&p),
            StableKey::Identifier {
                system: TAX_ID_SYSTEM.to_string(),
                value: "TAX-999".to_string(),
            }
        );
    }

    #[test]
    fn falls_back_to_pid() {
        let p = base();
        assert_eq!(resolve_stable_key(&p), StableKey::Pid(p.id));
    }

    #[test]
    fn ignores_blank_identifier_value() {
        let mut p = base();
        p.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "   ".to_string(),
        ));
        assert_eq!(resolve_stable_key(&p), StableKey::Pid(p.id));
    }

    #[test]
    fn row_has_explicit_id_detects_a_present_non_null_id() {
        assert!(row_has_explicit_id(
            r#"{"id":"11111111-1111-4111-8111-111111111111","name":{"family":"Doe","given":[]}}"#
        ));
        assert!(!row_has_explicit_id(r#"{"name":{"family":"Doe"}}"#));
        assert!(!row_has_explicit_id(
            r#"{"id":null,"name":{"family":"Doe"}}"#
        ));
        assert!(!row_has_explicit_id("not json"));
    }

    #[test]
    fn keyless_only_when_no_strong_key_and_no_explicit_id() {
        // Strong identifier ⇒ never keyless, explicit id or not.
        let mut with_id = base();
        with_id.identifiers.push(Identifier::new(
            IdentifierType::SSN,
            "http://hl7.org/fhir/sid/us-ssn".to_string(),
            "123-45-6789".to_string(),
        ));
        assert!(!is_keyless(&with_id, false));
        assert!(!is_keyless(&with_id, true));

        // No strong key, tax_id, or explicit id ⇒ keyless.
        let bare = base();
        assert!(is_keyless(&bare, false));

        // No strong key, but the row named its own pid ⇒ not keyless (the
        // `Pid` fallback is a real upsert target, not a placeholder).
        assert!(!is_keyless(&bare, true));

        // tax_id alone is a real key even with no explicit id.
        let mut with_tax = base();
        with_tax.tax_id = Some("TAX-999".to_string());
        assert!(!is_keyless(&with_tax, false));
    }
}
