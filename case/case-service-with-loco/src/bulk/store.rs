//! Artifact storage for bulk jobs (`agents/share/bulk-import-export.md`
//! §3, §12).
//!
//! Bulk operations move opaque byte artifacts — the uploaded input file,
//! the export output, and the per-row error report — that are referenced
//! from `bulk_jobs.{input,result,error_report}_url`. Storage is behind
//! the [`ArtifactStore`] trait so the concrete backend is config-driven.
//!
//! ## Scope decision (BLK-5)
//!
//! Only [`LocalFsArtifactStore`] is implemented in this rollout — no S3
//! object-store backend, matching the task's bound (S3 was a
//! person-specific extra, BLK-4, built after CSV/review-routing). The
//! trait itself is nonetheless written **async**, following
//! care-pathway's shape rather than person's original synchronous one:
//! an object store is inherently asynchronous, and bridging that under a
//! sync signature would mean blocking a Tokio worker thread on every
//! artifact write. Writing it async now means a future S3 rollout
//! ("BLK-6"-shaped) is an **additive** `impl ArtifactStore for
//! S3ArtifactStore`, not a breaking signature change to every call site
//! in [`super::pipeline`] and [`super::worker`] that already await it.
//! [`from_backend`] treats `s3` as a real, named-but-unimplemented
//! request (a clear error) rather than silently falling back to local
//! storage — the same "asking for a backend you can't have is an error,
//! not a silent substitution" posture care-pathway's own `s3`-without-the-feature
//! path uses, so a deployment that sets
//! `CASE_BULK_ARTIFACT_BACKEND=s3` learns immediately rather than
//! discovering, later, that its clinical/personal-data export artifacts
//! were written to an ephemeral container disk.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use loco_rs::Error;
use loco_rs::Result;

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
}

/// Build the configured artifact store.
///
/// `CASE_BULK_ARTIFACT_BACKEND` selects it: `local` (default) or `s3`
/// (not implemented in this rollout — see the module docs). An
/// unrecognised value falls back to `local` with a warning rather than
/// failing to boot, matching the family's posture elsewhere (the
/// ABAC-policy and PASETO-key loaders behave the same way).
///
/// # Errors
///
/// When `s3` is requested (not implemented — see the module docs).
pub async fn from_env() -> Result<Box<dyn ArtifactStore>> {
    let backend = std::env::var("CASE_BULK_ARTIFACT_BACKEND").unwrap_or_default();
    from_backend(&backend).await
}

/// Build the store named by `backend`, the value
/// `CASE_BULK_ARTIFACT_BACKEND` would hold.
///
/// Split out from [`from_env`] so the selection rules are testable
/// without mutating the process environment.
///
/// # Errors
///
/// When `s3` is requested — see the module docs.
// `async` for interface consistency with `from_env` (and any future
// backend whose construction genuinely needs to await, e.g. the S3 SDK's
// config loader) rather than because this particular body awaits
// anything today.
#[allow(clippy::unused_async)]
pub async fn from_backend(backend: &str) -> Result<Box<dyn ArtifactStore>> {
    match backend.trim().to_ascii_lowercase().as_str() {
        "" | "local" | "fs" | "file" => Ok(Box::new(LocalFsArtifactStore::from_env())),
        "s3" => Err(Error::Message(
            "CASE_BULK_ARTIFACT_BACKEND=s3 is not implemented by this crate's BLK-5 rollout \
             (local-filesystem storage only); falling back to local storage would silently \
             write case export artifacts to an ephemeral container disk"
                .to_string(),
        )),
        other => {
            tracing::warn!(
                backend = other,
                "unknown CASE_BULK_ARTIFACT_BACKEND; falling back to the local store"
            );
            Ok(Box::new(LocalFsArtifactStore::from_env()))
        }
    }
}

/// Local-filesystem [`ArtifactStore`] for development, test, and (until a
/// future S3 rollout) production.
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

    /// Build a store from the environment: `CASE_BULK_ARTIFACT_DIR`, or a
    /// `case-bulk-artifacts` directory under the system temp dir when
    /// unset/blank.
    #[must_use]
    pub fn from_env() -> Self {
        let base = std::env::var("CASE_BULK_ARTIFACT_DIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(
                || std::env::temp_dir().join("case-bulk-artifacts"),
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
    /// Returns [`Error::Message`] if the reference is unsafe or escapes the
    /// base directory.
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
/// backslash.
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
        let outside = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(outside.path(), b"top secret").unwrap();
        let outside_ref = format!("file://{}", outside.path().display());

        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(
            store.get(&outside_ref).await.is_err(),
            "a file:// reference outside the base must be refused"
        );

        let ok_ref = store.put("jobs/x/out.jsonl", b"mine").await.unwrap();
        assert_eq!(store.get(&ok_ref).await.unwrap(), b"mine");
    }

    /// Asking for `s3` is an error, not a silent fallback to local
    /// storage (see the module docs).
    #[tokio::test]
    async fn s3_backend_is_an_error_not_a_fallback() {
        let error = super::from_backend("s3")
            .await
            .err()
            .expect("s3 must fail in this rollout");
        let message = error.to_string();
        assert!(
            message.contains("not implemented"),
            "the error must explain why: {message}"
        );
    }

    /// An unknown backend name falls back to local with a warning, so a
    /// typo does not stop the service booting.
    #[tokio::test]
    async fn unknown_backend_falls_back_to_local() {
        for name in ["gcs", "azure", "  S3-ish "] {
            let store = super::from_backend(name)
                .await
                .expect("an unknown backend must not stop the service booting");
            assert!(
                store
                    .put("probe.txt", b"x")
                    .await
                    .is_ok_and(|r| r.starts_with("file://")),
                "{name} should have fallen back to the local store"
            );
        }
    }

    /// The names that select the local store, including the empty default.
    #[tokio::test]
    async fn local_backend_names_and_the_default_select_local() {
        for name in ["", "local", "LOCAL", " fs ", "file"] {
            let store = super::from_backend(name).await.expect("local store");
            assert!(
                store
                    .put("probe.txt", b"x")
                    .await
                    .is_ok_and(|r| r.starts_with("file://")),
                "{name:?} should select the local store"
            );
        }
    }
}
