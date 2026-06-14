//! Generated SeaORM entity modules.
//!
//! In a typical Loco app this directory holds `sea-orm-codegen`-
//! generated entity files. This consumer crate has no local tables, so
//! there are none — the module is kept only to satisfy Loco's expected
//! `models::_entities` layout.

// All domain entities live in external services now (Patient, Place,
// Worker, Thing, Event). This crate keeps no SeaORM entities of its
// own, but Loco wants the `models` and `models::_entities` modules to
// exist.

/// Re-exports of the generated entities; intentionally empty here.
pub mod prelude;
