//! Domain models module.
//!
//! This consumer crate keeps **no** local SeaORM entities — every
//! domain object (Patient, Place, Worker, Thing/Folder, Event/Move)
//! lives in an external Main-X service and is fetched over HTTP. The
//! module exists only because Loco expects a `models` module (and a
//! nested `models::_entities`) to be present.

/// Placeholder for SeaORM entity modules; intentionally empty here.
pub mod _entities;
