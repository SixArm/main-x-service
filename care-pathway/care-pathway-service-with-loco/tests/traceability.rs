//! IEC 62304 §5.2–§5.5 — **machine-checked** requirement→test traceability.
//!
//! A traceability matrix that lives only in a document rots: a test gets
//! renamed, a requirement quietly loses its verification, and nobody
//! notices until an audit. This suite makes `compliance/traceability.tsv`
//! executable — it fails the build when
//!
//! - a row is malformed or a field is blank,
//! - a requirement id is duplicated,
//! - a requirement names no test, or
//! - a named test does not exist anywhere in the crate.
//!
//! It deliberately does **not** claim whole-spec traceability. The matrix
//! covers the compliance requirements and the safety- or security-relevant
//! pre-existing ones; asserting coverage we do not have would defeat the
//! purpose of having the check at all.
//!
//! These tests need no database, so they run on the default `cargo test`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// The matrix, embedded so the test cannot read a different file than the
/// one the repository ships.
const MATRIX: &str = include_str!("../compliance/traceability.tsv");

/// One parsed requirement row.
struct Requirement {
    id: String,
    statement: String,
    tests: Vec<String>,
}

/// Parse the matrix, skipping blank lines and `#` comments.
fn requirements() -> Vec<Requirement> {
    MATRIX
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                3,
                "malformed row (expected 3 tab-separated fields): {line:?}"
            );
            Requirement {
                id: fields[0].trim().to_string(),
                statement: fields[1].trim().to_string(),
                tests: fields[2]
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect(),
            }
        })
        .collect()
}

/// Every `.rs` file under a directory, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Every function name defined anywhere in `src/` or `tests/`.
///
/// Scanning the source is cruder than reflecting over the test harness,
/// but it is the only approach that works across unit tests (in `src/`)
/// and integration tests (in `tests/`) without a build-script or a nightly
/// feature — and it catches exactly the failure the matrix exists to
/// prevent: a renamed or deleted test.
fn defined_functions() -> HashSet<String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    assert!(!files.is_empty(), "found no Rust sources to scan");

    let mut names = HashSet::new();
    for file in files {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        for line in text.lines() {
            let trimmed = line.trim_start();
            let Some(rest) = trimmed
                .strip_prefix("fn ")
                .or_else(|| trimmed.strip_prefix("async fn "))
                .or_else(|| trimmed.strip_prefix("pub fn "))
                .or_else(|| trimmed.strip_prefix("pub async fn "))
            else {
                continue;
            };
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                names.insert(name);
            }
        }
    }
    names
}

/// The matrix parses, and every row is complete.
#[test]
fn matrix_is_well_formed() {
    let rows = requirements();
    assert!(
        rows.len() >= 30,
        "matrix looks truncated: {} rows",
        rows.len()
    );
    for row in &rows {
        assert!(!row.id.is_empty(), "a row has no requirement id");
        assert!(
            row.statement.len() > 10,
            "{}: the requirement statement must actually say something",
            row.id
        );
        assert!(
            !row.tests.is_empty(),
            "{}: a requirement with no verification is the thing this check exists to catch",
            row.id
        );
    }
}

/// Requirement ids are unique — a duplicate silently overwrites its twin
/// in any downstream report.
#[test]
fn requirement_ids_are_unique() {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for row in requirements() {
        *seen.entry(row.id).or_default() += 1;
    }
    let duplicates: Vec<&String> = seen
        .iter()
        .filter(|(_, n)| **n > 1)
        .map(|(id, _)| id)
        .collect();
    assert!(
        duplicates.is_empty(),
        "duplicate requirement ids: {duplicates:?}"
    );
}

/// **The load-bearing check**: every test the matrix names really exists.
/// Renaming or deleting a test without updating the matrix fails here.
#[test]
fn every_named_test_exists() {
    let defined = defined_functions();
    let mut missing = Vec::new();
    for row in requirements() {
        for test in row.tests {
            if !defined.contains(&test) {
                missing.push(format!("{} → {test}", row.id));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "compliance/traceability.tsv names tests that do not exist \
         (rename them in the matrix, or restore the tests): {missing:#?}"
    );
}

/// The four control-driving frameworks each carry requirements, so the
/// matrix cannot quietly become a single-framework document.
#[test]
fn every_framework_is_represented() {
    let ids: Vec<String> = requirements().into_iter().map(|r| r.id).collect();
    for prefix in ["HIPAA-", "GDPR-", "EHDS-", "ONC-", "IEC-"] {
        assert!(
            ids.iter().any(|id| id.starts_with(prefix)),
            "no requirement is traced for {prefix}*"
        );
    }
}

/// The scanner finds functions it should — a self-check, so a broken
/// scanner cannot make [`every_named_test_exists`] pass vacuously.
#[test]
fn function_scanner_finds_known_functions() {
    let defined = defined_functions();
    assert!(defined.len() > 100, "scanner found only {}", defined.len());
    for known in [
        "row_hash",                            // a `pub fn` in src/
        "verify",                              // a `pub fn` in src/
        "matrix_is_well_formed",               // a test in this very file
        "intact_chain_verifies",               // a unit test in src/
        "merge_folds_duplicate_into_survivor", // a pre-existing unit test
    ] {
        assert!(defined.contains(known), "scanner missed {known}");
    }
    assert!(
        !defined.contains("definitely_not_a_real_function_name"),
        "scanner reports functions that do not exist"
    );
}
