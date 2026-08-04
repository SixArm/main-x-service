//! `seed_examples` task — load the repository's shared demo fixture
//! (`examples/data/cases.jsonl`) into a running service, for the
//! tutorials (EX-4 in the repo-root `tasks.md`).
//!
//! ```text
//! cargo loco task seed_examples
//! ```
//!
//! ## Why this bypasses `POST /api/cases`
//!
//! This task calls the **model-layer create** ([`cases::Model::create`])
//! directly rather than the HTTP create handler. That insert has no
//! duplicate check, and — because it is called with the pooled
//! connection rather than through `streaming::create_and_emit` — writes
//! no audit row and publishes no event either. This is **deliberate for
//! a seed task, not a defect to fix**: the tutorials that exercise
//! duplicate detection, audit, and events do so against records already
//! present, not against the act of seeding itself.
//!
//! Once all ten cases are seeded, the `subject_of` links to person
//! records documented in `examples/data/case-subject-links.md` still
//! need to be created separately via `POST /api/cases/{pid}/links` —
//! the case pids are not knowable until after this task (and the
//! person `seed_examples` task) have run, so that step is not automated
//! here.
//!
//! ## Idempotency
//!
//! Before inserting anything the task counts the `cases` table. A
//! non-empty table means a previous run already seeded it (or the
//! database holds real data), so the task refuses to insert a second
//! copy of every row and returns cleanly.

use std::path::Path;

use case_matcher::Case;
use loco_rs::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};

use crate::models::cases::{self, Model as CaseModel};

/// The fixture's location, relative to this crate's manifest directory
/// (two levels below the repository root: `case/case-service-with-loco`).
pub const FIXTURE_PATH: &str = "../../examples/data/cases.jsonl";

/// One JSONL line that failed to parse: its 1-based line number and the
/// `serde_json` error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    /// 1-based line number within the file.
    pub line: usize,
    /// The `serde_json` error, rendered.
    pub message: String,
}

/// Parse every non-blank line of a JSONL fixture into a [`Case`] via the
/// crate's own [`crate::bulk::jsonl::parse_line`] (the same parser
/// bulk import uses), separating successes from per-line failures. The
/// fixture carries no `pid` column, so every row is keyless — the
/// [`crate::bulk::row::BulkCaseRow::pid`] is discarded here.
#[must_use]
pub fn parse_fixture(contents: &str) -> (Vec<Case>, Vec<ParseFailure>) {
    let mut cases = Vec::new();
    let mut failures = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match crate::bulk::jsonl::parse_line(line) {
            Ok(row) => cases.push(row.case),
            Err(error) => failures.push(ParseFailure {
                line: idx + 1,
                message: error.to_string(),
            }),
        }
    }
    (cases, failures)
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

/// Seed `cases` from `fixture`, unless the table already holds rows.
///
/// # Errors
///
/// When the row count query or an insert fails.
pub async fn seed(db: &DatabaseConnection, fixture: &[Case]) -> Result<SeedReport> {
    let existing = cases::Entity::find().count(db).await?;
    if existing > 0 {
        println!(
            "cases table is not empty ({existing} rows) — skipping seed; reset the database first if you want to reseed"
        );
        return Ok(SeedReport {
            total: fixture.len(),
            created: 0,
            existing: usize::try_from(existing).unwrap_or(usize::MAX),
            skipped: true,
        });
    }

    let mut created = 0usize;
    for case in fixture {
        let saved = CaseModel::create(db, case).await?;
        created += 1;
        println!(
            "seeded case {created}/{}: {} ({})",
            fixture.len(),
            saved.title,
            saved.pid
        );
    }
    println!("seeded {created} of {} cases (existing: 0)", fixture.len());
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
            detail: "Load examples/data/cases.jsonl into the cases table for the tutorials (EX-4); no-op if the table is not empty.".to_string(),
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
    use super::{Case, FIXTURE_PATH, parse_fixture};
    use crate::bulk::row::BulkCaseRow;
    use std::path::Path;

    /// The fixture's own wire type is `Case` (via `BulkCaseRow`'s
    /// flattened shape) — a real line must parse cleanly via the
    /// crate's own bulk-import parser, catching wire-type / fixture
    /// drift immediately and without a database.
    #[test]
    fn parses_the_real_fixtures_first_line() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let first_line = contents
            .lines()
            .next()
            .expect("fixture has at least one line");
        let row: BulkCaseRow = crate::bulk::jsonl::parse_line(first_line)
            .unwrap_or_else(|e| panic!("fixture line 1 failed to parse as a Case: {e}"));
        let case: Case = row.case;
        assert!(!case.title.is_empty());
        assert!(row.pid.is_none());
    }

    /// Every line of the real fixture parses, and the file holds the
    /// documented 10 rows (`examples/data/README.md`).
    #[test]
    fn parse_fixture_parses_every_line_of_the_real_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let (cases, failures) = parse_fixture(&contents);
        assert!(
            failures.is_empty(),
            "unexpected parse failures: {failures:?}"
        );
        assert_eq!(cases.len(), 10, "expected 10 cases in the fixture");
    }

    /// The task advertises the name operators type.
    #[test]
    fn task_is_named_seed_examples() {
        use loco_rs::task::Task;
        assert_eq!(super::SeedExamples.task().name, "seed_examples");
    }
}
