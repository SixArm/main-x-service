//! Domain models for the Thing Service.
//!
//! This module groups the registry's value objects and the core entity:
//!
//! - [`thing`](crate::models::thing) — the
//!   [`Thing`](crate::models::thing::Thing) entity, modeled on
//!   [schema.org/Thing](https://schema.org/Thing).
//! - [`identifier`](crate::models::identifier) — the
//!   [`ThingIdentifier`](crate::models::identifier::ThingIdentifier)
//!   typed identifier (schema.org `PropertyValue` shape) and its
//!   [`IdentifierType`](crate::models::identifier::IdentifierType) scheme
//!   enum.
//! - [`consent`](crate::models::consent) — the
//!   [`Consent`](crate::models::consent::Consent) record used for
//!   data-protection compliance (GDPR).
//!
//! These types are plain serde-serializable structs with no I/O
//! dependencies, so they can be constructed and round-tripped freely in
//! tests and at the API boundary.

/// Consent records for data-protection compliance.
pub mod consent;
/// Typed identifiers (DOI, ISBN, GTIN, …) in the schema.org `PropertyValue`
/// shape.
pub mod identifier;
/// The core [`Thing`](crate::models::thing::Thing) entity.
pub mod thing;
