//! `seed_examples` task — load the repository's shared demo fixture
//! (`examples/data/organizations.jsonl`) into a running service, for
//! the tutorials (EX-4 in the repo-root `tasks.md`).
//!
//! ```text
//! cargo loco task seed_examples
//! ```
//!
//! ## Why this bypasses `POST /api/organizations`
//!
//! This task calls the **model-layer create**
//! ([`organizations::Model::create`]) directly rather than the HTTP
//! create handler. That insert has no duplicate check, and — because it
//! is called with the pooled connection rather than through
//! `streaming::create_and_emit` — writes no audit row and publishes no
//! event either. This is **deliberate for a seed task, not a defect to
//! fix**: `examples/data/organizations.jsonl` carries no duplicate
//! organizations (unlike the person fixture), so nothing here is
//! *dropped* by skipping duplicate detection, but the same model-layer
//! path is used for consistency with the person and case seed tasks
//! (EX-4), and so a future edit that adds a duplicate pair to this
//! fixture does not silently start losing rows the way `POST
//! /api/organizations` would. The tutorials that exercise duplicate
//! detection, audit, and events do so against records already present,
//! not against the act of seeding itself.
//!
//! ## Idempotency
//!
//! Before inserting anything the task counts the `organizations` table.
//! A non-empty table means a previous run already seeded it (or the
//! database holds real data), so the task refuses to insert a second
//! copy of every row and returns cleanly.

use std::path::Path;

use loco_rs::prelude::*;
use organization_matcher::Organization;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};

use crate::models::_entities::organizations;
use crate::models::organizations::Model as OrgModel;

/// The fixture's location, relative to this crate's manifest directory
/// (two levels below the repository root:
/// `organization/organization-service-with-loco`).
pub const FIXTURE_PATH: &str = "../../examples/data/organizations.jsonl";

/// One JSONL line that failed to parse: its 1-based line number and the
/// error message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    /// 1-based line number within the file.
    pub line: usize,
    /// The parse error, rendered.
    pub message: String,
}

/// Parse every non-blank line of a JSONL fixture into an [`Organization`]
/// via the crate's own [`crate::bulk::jsonl::parse_line`] (the same
/// parser BLK-5 import uses), separating successes from per-line
/// failures. The fixture carries no `pid` column, so every row is
/// keyless — `had_explicit_pid`/`pid` are discarded here.
#[must_use]
pub fn parse_fixture(contents: &str) -> (Vec<Organization>, Vec<ParseFailure>) {
    let mut orgs = Vec::new();
    let mut failures = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match crate::bulk::jsonl::parse_line(line) {
            Ok((_had_explicit_pid, _pid, org)) => orgs.push(org),
            Err(message) => failures.push(ParseFailure {
                line: idx + 1,
                message,
            }),
        }
    }
    (orgs, failures)
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

/// Seed `organizations` from `fixture`, unless the table already holds
/// rows.
///
/// # Errors
///
/// When the row count query or an insert fails.
pub async fn seed(db: &DatabaseConnection, fixture: &[Organization]) -> Result<SeedReport> {
    let existing = organizations::Entity::find().count(db).await?;
    if existing > 0 {
        println!(
            "organizations table is not empty ({existing} rows) — skipping seed; reset the database first if you want to reseed"
        );
        return Ok(SeedReport {
            total: fixture.len(),
            created: 0,
            existing: usize::try_from(existing).unwrap_or(usize::MAX),
            skipped: true,
        });
    }

    let mut created = 0usize;
    for org in fixture {
        let saved = OrgModel::create(db, org).await?;
        created += 1;
        println!(
            "seeded organization {created}/{}: {} ({})",
            fixture.len(),
            saved.name,
            saved.pid
        );
    }
    println!(
        "seeded {created} of {} organizations (existing: 0)",
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
            detail: "Load examples/data/organizations.jsonl into the organizations table for the tutorials (EX-4); no-op if the table is not empty.".to_string(),
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
    use super::{FIXTURE_PATH, Organization, parse_fixture};
    use std::path::Path;

    /// The fixture's own wire type is `Organization` — a real line must
    /// parse cleanly via the crate's own bulk-import parser, catching
    /// wire-type / fixture drift immediately and without a database.
    #[test]
    fn parses_the_real_fixtures_first_line() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let first_line = contents
            .lines()
            .next()
            .expect("fixture has at least one line");
        let (had_explicit_pid, pid, org): (bool, Option<uuid::Uuid>, Organization) =
            crate::bulk::jsonl::parse_line(first_line)
                .unwrap_or_else(|e| panic!("fixture line 1 failed to parse as Organization: {e}"));
        assert!(!org.name.is_empty());
        assert!(!had_explicit_pid);
        assert!(pid.is_none());
    }

    /// Every line of the real fixture parses, and the file holds the
    /// documented 20 rows (`examples/data/README.md`).
    #[test]
    fn parse_fixture_parses_every_line_of_the_real_file() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let (orgs, failures) = parse_fixture(&contents);
        assert!(
            failures.is_empty(),
            "unexpected parse failures: {failures:?}"
        );
        assert_eq!(orgs.len(), 20, "expected 20 organizations in the fixture");
    }

    /// The task advertises the name operators type.
    #[test]
    fn task_is_named_seed_examples() {
        use loco_rs::task::Task;
        assert_eq!(super::SeedExamples.task().name, "seed_examples");
    }
}
