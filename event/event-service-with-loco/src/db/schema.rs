//! Database schema definitions (SeaORM).
//!
//! Historically this module held entity declarations; the live SeaORM
//! entities now live in [`super::models`]. This module is retained for
//! backward compatibility and as the documented home for any future
//! schema-level helpers.
//!
//! To regenerate entities from a live database:
//! `sea-orm-cli generate entity -o src/db/entity`.

// SeaORM entities are defined in the models module.
// This module is kept for backward compatibility and re-exports.
