// All domain entities live in external services now (Patient, Place,
// Worker, Thing, Event). This crate keeps no SeaORM entities of its
// own, but Loco wants the `models` and `models::_entities` modules to
// exist.
pub mod prelude;
