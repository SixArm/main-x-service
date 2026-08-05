//! `seed_examples` task — load the repository's shared demo fixture
//! (`examples/data/persons.jsonl`) into a running service, for the
//! tutorials (EX-4 in the repo-root `tasks.md`).
//!
//! ```text
//! cargo loco task seed_examples
//! ```
//!
//! ## Why this bypasses `POST /api/persons`
//!
//! `examples/data/persons.jsonl` deliberately contains five duplicate
//! pairs (see `examples/data/README.md` "The duplicate pairs"), so the
//! dedup/match/merge tutorial has real duplicate data to walk through.
//! The normal create handler runs real-time duplicate detection and
//! returns `409` on the second half of every pair — EX-1 confirmed this
//! live against a running service — so seeding through it would
//! silently drop half the fixture. This task instead calls the
//! **model-layer create** ([`SeaOrmPersonRepository::create`]) directly.
//! That insert has no duplicate check, and — because no event publisher
//! or audit log is attached to the repository here — writes no audit
//! row and publishes no event either. This is **deliberate for a seed
//! task, not a defect to fix**: the tutorials that exercise duplicate
//! detection, audit, and events do so against records already present,
//! not against the act of seeding itself.
//!
//! ## Idempotency
//!
//! Before inserting anything the task counts the `persons` table. A
//! non-empty table means a previous run already seeded it (or the
//! database holds real data), so the task refuses to insert a second
//! copy of every row and returns cleanly — the model-layer create used
//! here has none of the duplicate detection that would otherwise catch
//! a re-run.

use std::path::Path;

use loco_rs::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};

use crate::db::models::persons;
use crate::db::repositories::{AuditContext, PersonRepository, SeaOrmPersonRepository};
use crate::models::Person;

/// The fixture's location, relative to this crate's manifest directory
/// (two levels below the repository root:
/// `person/person-service-with-loco`).
pub const FIXTURE_PATH: &str = "../../examples/data/persons.jsonl";

/// One JSONL line that failed to parse: its 1-based line number and the
/// `serde_json` error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    /// 1-based line number within the file.
    pub line: usize,
    /// The `serde_json` error, rendered.
    pub message: String,
}

/// Parse every non-blank line of a JSONL fixture into a [`Person`],
/// separating successes from per-line failures. Mirrors the bulk
/// import's per-row error contract
/// (`agents/share/bulk-import-export.md` §7) in spirit — one bad line
/// does not abort the rest — though a seed run has no downloadable
/// error-report artifact, just the returned list.
#[must_use]
pub fn parse_fixture(contents: &str) -> (Vec<Person>, Vec<ParseFailure>) {
    let mut persons = Vec::new();
    let mut failures = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match crate::bulk::jsonl::parse_line(line) {
            Ok(person) => persons.push(person),
            Err(error) => failures.push(ParseFailure {
                line: idx + 1,
                message: error.to_string(),
            }),
        }
    }
    (persons, failures)
}

/// Outcome of one [`seed`] run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SeedReport {
    /// Rows offered for insertion (parsed from the fixture).
    pub total: usize,
    /// Rows actually inserted this run.
    pub created: usize,
    /// Rows already in the table when the run started.
    pub existing: usize,
    /// Whether the run skipped inserting because the table was
    /// non-empty.
    pub skipped: bool,
}

/// Seed `persons` from `fixture`, unless the table already holds rows.
///
/// Inserts via [`SeaOrmPersonRepository::create`] — the model-layer
/// create, deliberately bypassing duplicate detection (see the module
/// docs) — with no event publisher and no audit log attached.
///
/// # Errors
///
/// When the row count query or an insert fails.
pub async fn seed(db: &DatabaseConnection, fixture: &[Person]) -> Result<SeedReport> {
    let existing = persons::Entity::find().count(db).await?;
    if existing > 0 {
        println!(
            "persons table is not empty ({existing} rows) — skipping seed; reset the database first if you want to reseed"
        );
        return Ok(SeedReport {
            total: fixture.len(),
            created: 0,
            existing: usize::try_from(existing).unwrap_or(usize::MAX),
            skipped: true,
        });
    }

    let repo = SeaOrmPersonRepository::new(db.clone());
    let mut created = 0usize;
    for person in fixture {
        // A CLI seed task has no authenticated caller — `AuditContext::default()`
        // ("system") is the legitimate actor here, not a gap SEC-B8 closes.
        let saved = repo
            .create(person, &AuditContext::default())
            .await
            .map_err(|e| Error::string(&format!("creating person: {e}")))?;
        created += 1;
        println!(
            "seeded person {created}/{}: {} ({})",
            fixture.len(),
            saved.full_name(),
            saved.id
        );
    }
    println!(
        "seeded {created} of {} persons (existing: 0)",
        fixture.len()
    );
    Ok(SeedReport {
        total: fixture.len(),
        created,
        existing: 0,
        skipped: false,
    })
}

/// The `seed_examples` CLI task.
pub struct SeedExamples;

#[async_trait]
impl Task for SeedExamples {
    /// Task metadata (name + one-line description) for the loco CLI.
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "seed_examples".to_string(),
            detail: "Load examples/data/persons.jsonl into the persons table for the tutorials (EX-4); no-op if the table is not empty.".to_string(),
        }
    }

    /// Read the fixture, parse it, and seed the database.
    ///
    /// # Errors
    ///
    /// When the fixture cannot be read, no row parses, the row count
    /// query fails, or an insert fails.
    async fn run(&self, ctx: &AppContext, _vars: &task::Vars) -> Result<()> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .map_err(|e| Error::string(&format!("reading {}: {e}", path.display())))?;
        let (fixture, failures) = parse_fixture(&contents);
        for failure in &failures {
            eprintln!(
                "skipping {} line {}: {}",
                path.display(),
                failure.line,
                failure.message
            );
        }
        if fixture.is_empty() {
            return Err(Error::string(&format!(
                "no rows parsed from {} ({} parse failures)",
                path.display(),
                failures.len()
            )));
        }
        seed(&ctx.db, &fixture).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{FIXTURE_PATH, Person, parse_fixture};
    use std::path::Path;

    /// The fixture's own wire type is `Person` — a real line must parse
    /// cleanly with no crate-side modification, catching wire-type /
    /// fixture drift immediately and without a database.
    #[test]
    fn parses_the_real_fixtures_first_line() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let first_line = contents
            .lines()
            .next()
            .expect("fixture has at least one line");
        let person: Person = crate::bulk::jsonl::parse_line(first_line)
            .unwrap_or_else(|e| panic!("fixture line 1 failed to parse as a Person: {e}"));
        assert!(!person.name.family.is_empty());
    }

    /// Every line of the real fixture parses, and the file holds the
    /// documented 50 rows (`examples/data/README.md`).
    #[test]
    fn parse_fixture_parses_every_line_of_the_real_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let (persons, failures) = parse_fixture(&contents);
        assert!(
            failures.is_empty(),
            "unexpected parse failures: {failures:?}"
        );
        assert_eq!(persons.len(), 50, "expected 50 persons in the fixture");
    }

    /// The task advertises the name operators type.
    #[test]
    fn task_is_named_seed_examples() {
        use loco_rs::task::Task;
        assert_eq!(super::SeedExamples.task().name, "seed_examples");
    }
}
