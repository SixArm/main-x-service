//! Search query builders (reserved).
//!
//! This module is a placeholder for higher-level, reusable Tantivy
//! query constructors (term/boolean/fuzzy/phonetic builders). Today the
//! queries are built inline where they are used: see
//! [`SearchEngine`](crate::search::SearchEngine) for fuzzy and phrase queries
//! and [`PersonIndex`](crate::search::index::PersonIndex) for the schema field
//! handles those queries target. Extracting them here is future work.

// Query construction for Tantivy searches
