//! Artifact storage for bulk jobs (`agents/share/bulk-import-export.md`
//! §3, §12).
//!
//! Bulk operations move opaque byte artifacts — the uploaded input file,
//! the export output, and the per-row error report — that are referenced
//! from `bulk_jobs.{input,result,error_report}_url`. Storage is behind
//! the [`ArtifactStore`] trait so a future backend is a new
//! implementation, not a call-site rewrite.
//!
//! ## Scope: local-filesystem only (this rollout step)
//!
//! Unlike person's / care-pathway's bulk modules, this crate does **not**
//! ship an S3-compatible backend. BLK-5 is scoped to JSONL + CSV and
//! local storage; a deployment needing durable object storage is a
//! follow-up, not built here. [`LocalFsArtifactStore`] is a direct port
//! of care-pathway's (and person's, pre-S3) local backend.
//!
//! The trait is still **async**, even though its only implementation
//! today is synchronous filesystem I/O wrapped in async fns. This is a
//! deliberate inheritance decision: a future S3 rollout only has to add
//! a second `impl ArtifactStore`, not change every call site's
//! signature — the sync/async boundary is exactly where person's and
//! care-pathway's own S3 rollouts needed a breaking change when the
//! trait started synchronous. Paying that cost once, now, while the
//! surface is small, is cheaper than paying it later across a live
//! worker + handlers.
//!
//! [`ArtifactStore::presigned_get`] is kept (default `Ok(None)`) for the
//! same forward-compatibility reason: a local `file://` reference is
//! never fetchable by a remote client, so `None` is the honest answer
//! today, and a future S3 backend only has to override the default
//! rather than add a new trait method.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use loco_rs::{Error, Result};

/// A place to `put` and `get` opaque bulk-job byte artifacts.
///
/// `put` stores `bytes` under a caller-chosen `key` and returns an opaque
/// **reference** string (persisted in `bulk_jobs.*_url`); `get` resolves
/// a previously-returned reference back to its bytes. References are
/// backend-specific and must not be interpreted by callers.
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    /// Store `bytes` under `key`, returning an opaque reference.
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact cannot be written.
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<String>;

    /// Resolve a reference returned by [`put`](ArtifactStore::put) back to
    /// its stored bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the artifact is missing or unreadable.
    async fn get(&self, reference: &str) -> Result<Vec<u8>>;

    /// A short-lived URL a client may fetch the artifact from directly,
    /// when the backend supports one. `None` by default — see the module
    /// docs.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend supports presigning but fails.
    async fn presigned_get(&self, _reference: &str, _ttl_secs: u64) -> Result<Option<String>> {
        Ok(None)
    }
}

/// Build the configured artifact store. This rollout step ships only the
/// local-filesystem backend, so this always returns
/// [`LocalFsArtifactStore`] built from the environment
/// (`ORGANIZATION_BULK_ARTIFACT_DIR`) — kept `async` (and returning by
/// value rather than erroring) so a future backend-selection env var
/// (`ORGANIZATION_BULK_ARTIFACT_BACKEND`, mirroring the sibling
/// services) slots in without changing any call site. The local backend
/// itself needs no `.await` today, hence the explicit allow (matching
/// care-pathway's identically-motivated `s3_from_env` fallback).
#[allow(clippy::unused_async)]
pub async fn from_env() -> Box<dyn ArtifactStore> {
    Box::new(LocalFsArtifactStore::from_env())
}

/// Local-filesystem [`ArtifactStore`] for development, test, and (for
/// this rollout step) production.
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

    /// Build a store from the environment:
    /// `ORGANIZATION_BULK_ARTIFACT_DIR`, or an
    /// `organization-bulk-artifacts` directory under the system temp dir
    /// when unset/blank.
    #[must_use]
    pub fn from_env() -> Self {
        let base = std::env::var("ORGANIZATION_BULK_ARTIFACT_DIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(
                || std::env::temp_dir().join("organization-bulk-artifacts"),
                PathBuf::from,
            );
        Self::new(base)
    }

    /// Absolute path for `key` under the base directory.
    fn path_for(&self, key: &str) -> PathBuf {
        self.base.join(key)
    }

    /// Resolve a `get` reference to a **confined** path (SEC-B4). A
    /// `file://` reference must, once resolved, live under this store's
    /// base directory; anything else (`file:///etc/passwd`, a
    /// `..`-escape) is rejected rather than read. A bare (non-`file://`)
    /// reference is treated as a key and validated with [`is_safe_key`].
    ///
    /// # Errors
    ///
    /// Returns an error if the reference is unsafe or escapes the base
    /// directory.
    fn resolve_get_path(&self, reference: &str) -> Result<PathBuf> {
        let candidate = if let Some(stripped) = reference.strip_prefix("file://") {
            Path::new(stripped).to_path_buf()
        } else {
            if !is_safe_key(reference) {
                return Err(Error::Message(format!(
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
            Err(Error::Message(format!(
                "artifact reference escapes the store base: {reference}"
            )))
        }
    }
}

/// Whether `key` is a safe relative artifact key (SEC-B4): non-empty, not
/// absolute, no parent (`..`) component, and no Windows drive prefix or
/// backslash. Pure, so `put`/`get` and their tests share one definition.
/// A future object-store backend would need the same rule (a key-namespace
/// escape there, not a path traversal), so it stays a free function rather
/// than a method on this backend alone.
#[must_use]
pub fn is_safe_key(key: &str) -> bool {
    use std::path::Component;
    if key.is_empty() || key.contains('\\') {
        return false;
    }
    let path = Path::new(key);
    path.components().all(|c| matches!(c, Component::Normal(_)))
}

#[async_trait]
impl ArtifactStore for LocalFsArtifactStore {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<String> {
        if !is_safe_key(key) {
            return Err(Error::Message(format!(
                "refusing unsafe artifact key: {key}"
            )));
        }
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::Message(format!("create artifact dir: {e}")))?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| Error::Message(format!("write artifact {key}: {e}")))?;
        let abs = std::fs::canonicalize(&path).unwrap_or(path);
        Ok(format!("file://{}", abs.display()))
    }

    async fn get(&self, reference: &str) -> Result<Vec<u8>> {
        let path = self.resolve_get_path(reference)?;
        std::fs::read(&path).map_err(|e| Error::Message(format!("read artifact {reference}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactStore, LocalFsArtifactStore};

    #[tokio::test]
    async fn put_then_get_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        let reference = store
            .put("jobs/abc/input.jsonl", b"hello bulk")
            .await
            .unwrap();
        assert!(reference.starts_with("file://"));
        let bytes = store.get(&reference).await.unwrap();
        assert_eq!(bytes, b"hello bulk");
    }

    #[tokio::test]
    async fn get_missing_artifact_errors() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(store.get("file:///no/such/artifact").await.is_err());
    }

    /// The local backend cannot issue a fetchable URL, and says so with
    /// `None` rather than handing back a `file://` reference that looks
    /// like one.
    #[tokio::test]
    async fn local_store_offers_no_presigned_url() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        let reference = store.put("jobs/abc/out.jsonl", b"x").await.unwrap();
        assert_eq!(store.presigned_get(&reference, 300).await.unwrap(), None);
    }

    #[test]
    fn is_safe_key_rejects_traversal_and_absolute() {
        use super::is_safe_key;
        assert!(is_safe_key("jobs/abc/input.jsonl"));
        assert!(is_safe_key("input.jsonl"));
        assert!(!is_safe_key("../secret"));
        assert!(!is_safe_key("jobs/../../etc/passwd"));
        assert!(!is_safe_key("/etc/passwd"));
        assert!(!is_safe_key("./x"));
        assert!(!is_safe_key(""));
        assert!(!is_safe_key("a\\b"));
    }

    #[tokio::test]
    async fn put_rejects_an_unsafe_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(store.put("../escape.txt", b"x").await.is_err());
        assert!(store.put("/tmp/escape.txt", b"x").await.is_err());
    }

    #[tokio::test]
    async fn get_refuses_a_reference_outside_the_base() {
        // A real file outside the store base must not be readable through
        // a crafted file:// reference (SEC-B4 arbitrary-read guard).
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"top secret").unwrap();
        let outside_ref = format!("file://{}", outside.path().display());

        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(
            store.get(&outside_ref).await.is_err(),
            "a file:// reference outside the base must be refused"
        );

        // A legitimately stored artifact still round-trips.
        let ok_ref = store.put("jobs/x/out.jsonl", b"mine").await.unwrap();
        assert_eq!(store.get(&ok_ref).await.unwrap(), b"mine");
    }

    /// Every name selects the (only) local store — this rollout step has
    /// no backend-selection switch.
    #[tokio::test]
    async fn from_env_always_selects_the_local_store() {
        let store = super::from_env().await;
        assert!(
            store
                .put("probe.txt", b"x")
                .await
                .is_ok_and(|r| r.starts_with("file://"))
        );
    }
}
