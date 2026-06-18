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
//! - [`merge`] — [`MergeRequest`](merge::MergeRequest)/response/record types
//!   for folding a duplicate place into a surviving main place.

/// [`PostalAddress`](address::PostalAddress) — structured postal address.
pub mod address;
/// [`AmenityFeature`](amenity::AmenityFeature) — named key/value features.
pub mod amenity;
/// GDPR [`Consent`](consent::Consent) records and their lifecycle.
pub mod consent;
/// [`GeoCoordinates`](geo::GeoCoordinates) with Haversine distance.
pub mod geo;
/// External [`PlaceIdentifier`](identifier::PlaceIdentifier)s (GLN, FIPS, GNIS, OSM, …).
pub mod identifier;
/// Duplicate-merge request/response/record types.
pub mod merge;
/// [`OpeningHoursSpecification`](opening_hours::OpeningHoursSpecification) per day.
pub mod opening_hours;
/// The central [`Place`](place::Place) aggregate.
pub mod place;
/// The [`PlaceType`](place_type::PlaceType) classification enum.
pub mod place_type;
