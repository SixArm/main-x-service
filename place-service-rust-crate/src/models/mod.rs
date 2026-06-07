//! Domain models for the place registry, based on
//! [schema.org/Place](https://schema.org/Place).
//!
//! These are plain, serializable value objects and aggregates with no
//! database, HTTP, or matching dependencies — they form the shared
//! vocabulary every other layer (matching, validation, privacy, the API
//! tier) operates on.
//!
//! - [`place`] — the central [`Place`](place::Place) aggregate.
//! - [`address`] — [`PostalAddress`](address::PostalAddress).
//! - [`geo`] — [`GeoCoordinates`](geo::GeoCoordinates) with Haversine distance.
//! - [`place_type`] — the [`PlaceType`](place_type::PlaceType) classification enum.
//! - [`identifier`] — external identifiers (GLN, FIPS, GNIS, OSM, …).
//! - [`amenity`] — [`AmenityFeature`](amenity::AmenityFeature) key/value features.
//! - [`opening_hours`] — [`OpeningHoursSpecification`](opening_hours::OpeningHoursSpecification).
//! - [`consent`] — GDPR [`Consent`](consent::Consent) records.

pub mod address;
pub mod amenity;
pub mod consent;
pub mod geo;
pub mod identifier;
pub mod opening_hours;
pub mod place;
pub mod place_type;
