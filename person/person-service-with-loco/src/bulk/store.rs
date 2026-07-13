//! Artifact storage for bulk jobs (`agents/share/bulk-import-export.md`
//! §3, §12).
//!
//! Bulk operations move opaque byte artifacts — the uploaded input file,
//! the export output, and the per-row error report — that are referenced
//! from `bulk_jobs.{input,result,error_report}_url`. Storage is behind
//! the [`ArtifactStore`] trait so the concrete backend is config-driven.
//!
//! Rollout step 1 ships the dev/test backend, [`LocalFsArtifactStore`],
//! which writes under a configurable base directory
//! (`PERSON_BULK_ARTIFACT_DIR`). The deployment backend is an
//! **S3-compatible object store** returning short-lived, access-controlled
//! URLs — that impl is a later step and is intentionally NOT built here.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// A place to `put` and `get` opaque bulk-job byte artifacts.
///
/// `put` stores `bytes` under a caller-chosen `key` and returns an opaque
/// **reference** string (persisted in `bulk_jobs.*_url`); `get` resolves
/// a previously-returned reference back to its bytes. References are
/// backend-specific and must not be interpreted by callers.
pub trait ArtifactStore: Send + Sync {
    /// Store `bytes` under `key`, returning an opaque reference.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Internal`] if the artifact cannot be written.
    fn put(&self, key: &str, bytes: &[u8]) -> Result<String>;

    /// Resolve a reference returned by [`put`](ArtifactStore::put) back to
    /// its stored bytes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Internal`] if the artifact is missing or unreadable.
    fn get(&self, reference: &str) -> Result<Vec<u8>>;
}

/// Local-filesystem [`ArtifactStore`] for development and tests.
///
/// Artifacts live under a base directory; the returned reference is a
/// `file://<absolute-path>` URL, which [`get`](ArtifactStore::get)
/// resolves back to a path.
pub struct LocalFsArtifactStore {
    /// Base directory under which artifacts are written.
    base: PathBuf,
}

impl LocalFsArtifactStore {
    /// Build a store rooted at `base`.
    #[must_use]
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    /// Build a store from the environment: `PERSON_BULK_ARTIFACT_DIR`, or
    /// a `person-bulk-artifacts` directory under the system temp dir when
    /// unset/blank.
    #[must_use]
    pub fn from_env() -> Self {
        let base = std::env::var("PERSON_BULK_ARTIFACT_DIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(
                || std::env::temp_dir().join("person-bulk-artifacts"),
                PathBuf::from,
            );
        Self::new(base)
    }

    /// Absolute path for `key` under the base directory.
    fn path_for(&self, key: &str) -> PathBuf {
        self.base.join(key)
    }

    /// Resolve a `get` reference to a **confined** path (SEC-B4). A
    /// `file://` reference must, once resolved, live under this store's base
    /// directory; anything else (`file:///etc/passwd`, a `..`-escape) is
    /// rejected rather than read. A bare (non-`file://`) reference is
    /// treated as a key and validated with [`is_safe_key`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::Internal`] if the reference is unsafe or escapes the
    /// base directory.
    fn resolve_get_path(&self, reference: &str) -> Result<PathBuf> {
        let candidate = if let Some(stripped) = reference.strip_prefix("file://") {
            Path::new(stripped).to_path_buf()
        } else {
            if !is_safe_key(reference) {
                return Err(Error::Internal(format!(
                    "refusing unsafe artifact reference: {reference}"
                )));
            }
            self.path_for(reference)
        };
        // Confine to the base: compare canonicalised absolute paths so a
        // symlink or `..` cannot escape. The base is canonicalised with a
        // raw fallback so a not-yet-created base still yields a usable root.
        let base_abs = std::fs::canonicalize(&self.base).unwrap_or_else(|_| self.base.clone());
        let resolved = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
        if resolved.starts_with(&base_abs) {
            Ok(resolved)
        } else {
            Err(Error::Internal(format!(
                "artifact reference escapes the store base: {reference}"
            )))
        }
    }
}

/// Whether `key` is a safe relative artifact key (SEC-B4): non-empty, not
/// absolute, no parent (`..`) component, and no Windows drive prefix or
/// backslash. Pure, so `put`/`get` and their tests share one definition.
#[must_use]
pub fn is_safe_key(key: &str) -> bool {
    use std::path::Component;
    if key.is_empty() || key.contains('\\') {
        return false;
    }
    let path = Path::new(key);
    path.components().all(|c| matches!(c, Component::Normal(_)))
}

impl ArtifactStore for LocalFsArtifactStore {
    fn put(&self, key: &str, bytes: &[u8]) -> Result<String> {
        if !is_safe_key(key) {
            return Err(Error::Internal(format!(
                "refusing unsafe artifact key: {key}"
            )));
        }
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Internal(format!("create artifact dir: {e}")))?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| Error::Internal(format!("write artifact {key}: {e}")))?;
        let abs = std::fs::canonicalize(&path).unwrap_or(path);
        Ok(format!("file://{}", abs.display()))
    }

    fn get(&self, reference: &str) -> Result<Vec<u8>> {
        let path = self.resolve_get_path(reference)?;
        std::fs::read(&path).map_err(|e| Error::Internal(format!("read artifact {reference}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactStore, LocalFsArtifactStore};

    #[test]
    fn put_then_get_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        let reference = store.put("jobs/abc/input.jsonl", b"hello bulk").unwrap();
        assert!(reference.starts_with("file://"));
        let bytes = store.get(&reference).unwrap();
        assert_eq!(bytes, b"hello bulk");
    }

    #[test]
    fn get_missing_artifact_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(store.get("file:///no/such/artifact").is_err());
    }

    #[test]
    fn is_safe_key_rejects_traversal_and_absolute() {
        use super::is_safe_key;
        assert!(is_safe_key("jobs/abc/input.jsonl"));
        assert!(is_safe_key("input.jsonl"));
        // Traversal, absolute, current-dir, and Windows-ish forms are unsafe.
        assert!(!is_safe_key("../secret"));
        assert!(!is_safe_key("jobs/../../etc/passwd"));
        assert!(!is_safe_key("/etc/passwd"));
        assert!(!is_safe_key("./x"));
        assert!(!is_safe_key(""));
        assert!(!is_safe_key("a\\b"));
    }

    #[test]
    fn put_rejects_an_unsafe_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(store.put("../escape.txt", b"x").is_err());
        assert!(store.put("/tmp/escape.txt", b"x").is_err());
    }

    #[test]
    fn get_refuses_a_reference_outside_the_base() {
        // A real file outside the store base must not be readable through a
        // crafted file:// reference (SEC-B4 arbitrary-read guard).
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"top secret").unwrap();
        let outside_ref = format!("file://{}", outside.path().display());

        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(
            store.get(&outside_ref).is_err(),
            "a file:// reference outside the base must be refused"
        );

        // A legitimately stored artifact still round-trips.
        let ok_ref = store.put("jobs/x/out.jsonl", b"mine").unwrap();
        assert_eq!(store.get(&ok_ref).unwrap(), b"mine");
    }
}
