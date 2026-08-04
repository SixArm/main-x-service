//! CLI tasks, registered in [`crate::app::App::register_tasks`] and run
//! with `cargo loco task <name>`.

/// `search_reindex` — rebuild the full-text index from the database.
pub mod search;
/// `seed_examples` — load `examples/data/organizations.jsonl` for the
/// tutorials.
pub mod seed_examples;
