//! Artifact storage for bulk jobs (`agents/share/bulk-import-export.md`
//! §3, §12).
//!
//! Bulk operations move opaque byte artifacts — the uploaded input file,
//! the export output, and the per-row error report — that are referenced
//! from `bulk_jobs.{input,result,error_report}_url`. Storage is behind
//! the [`ArtifactStore`] trait so the concrete backend is config-driven.
//!
//! Two backends ship:
//!
//! - [`LocalFsArtifactStore`] — dev and test. Writes under a configurable
//!   base directory (`CARE_PATHWAY_BULK_ARTIFACT_DIR`) and returns
//!   `file://` references, confined to that base (SEC-B4).
//! - [`S3ArtifactStore`] — deployment, behind the **`s3` cargo feature**.
//!   Any S3-compatible object store (AWS S3, MinIO, Ceph RGW, R2), with
//!   **presigned, short-lived GET URLs** as the shared doc §3 requires.
//!
//! Selected at runtime by [`from_env`]; see that function for the
//! environment variables.
//!
//! ## Why the trait is async
//!
//! It was originally synchronous, which was fine while the only backend
//! was the local filesystem. An object store is inherently asynchronous,
//! and bridging that under a sync signature means blocking a Tokio worker
//! thread on every artifact write — the failure mode being a stalled
//! runtime under load rather than an error anyone would see in a test. The
//! trait is therefore `async_trait`, and the one call site (the
//! `bulk_export` worker) was already async.
//!
//! ## Why the AWS SDK rather than hand-rolled SigV4
//!
//! Request signing is security-relevant code, and a hand-rolled
//! implementation could not be verified here: checking it needs either the
//! published AWS test vectors or a live endpoint, and shipping unverified
//! signing code that *looks* finished is worse than depending on a
//! maintained implementation. The SDK is optional so no deployment using
//! the local backend pays for it.

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

    /// A short-lived URL a client may fetch the artifact from directly,
    /// when the backend supports one.
    ///
    /// `None` by default. The local backend genuinely cannot issue one —
    /// a `file://` path is not fetchable by a remote client — and
    /// returning `None` says so rather than handing back a reference that
    /// looks like a URL and is not.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend supports presigning but fails.
    async fn presigned_get(&self, _reference: &str, _ttl_secs: u64) -> Result<Option<String>> {
        Ok(None)
    }
}

/// Build the configured artifact store.
///
/// `CARE_PATHWAY_BULK_ARTIFACT_BACKEND` selects it: `local` (default) or
/// `s3`. An unrecognised value falls back to `local` with a warning rather
/// than failing to boot, matching the family's posture elsewhere (the
/// ABAC-policy and PASETO-key loaders behave the same way).
///
/// Asking for `s3` in a binary built **without** the `s3` feature is the
/// one case that returns an error rather than falling back. Silently
/// writing a deployment's clinical export artifacts to the local disk of
/// an ephemeral container — when the operator asked for an object store —
/// would lose data and look like it worked.
///
/// # Errors
///
/// When `s3` is requested but the feature is not compiled in, or its
/// configuration is incomplete.
pub async fn from_env() -> Result<Box<dyn ArtifactStore>> {
    let backend = std::env::var("CARE_PATHWAY_BULK_ARTIFACT_BACKEND").unwrap_or_default();
    from_backend(&backend).await
}

/// Build the store named by `backend`, the value
/// `CARE_PATHWAY_BULK_ARTIFACT_BACKEND` would hold.
///
/// Split out from [`from_env`] so the selection rules are testable without
/// mutating the process environment: `std::env::set_var` is `unsafe` in
/// this edition, and this crate is `#![forbid(unsafe_code)]`. It is also
/// racy under a parallel test harness, so even where it compiles it is the
/// wrong tool.
///
/// # Errors
///
/// When `s3` is requested but unavailable — see [`from_env`].
pub async fn from_backend(backend: &str) -> Result<Box<dyn ArtifactStore>> {
    match backend.trim().to_ascii_lowercase().as_str() {
        "" | "local" | "fs" | "file" => Ok(Box::new(LocalFsArtifactStore::from_env())),
        "s3" => s3_from_env().await,
        other => {
            tracing::warn!(
                backend = other,
                "unknown CARE_PATHWAY_BULK_ARTIFACT_BACKEND; falling back to the local store"
            );
            Ok(Box::new(LocalFsArtifactStore::from_env()))
        }
    }
}

/// Build the S3 store, or explain why it is unavailable.
#[cfg(feature = "s3")]
async fn s3_from_env() -> Result<Box<dyn ArtifactStore>> {
    Ok(Box::new(S3ArtifactStore::from_env().await?))
}

/// The `s3` feature is not compiled in.
#[cfg(not(feature = "s3"))]
#[allow(clippy::unused_async)]
async fn s3_from_env() -> Result<Box<dyn ArtifactStore>> {
    Err(Error::Message(
        "CARE_PATHWAY_BULK_ARTIFACT_BACKEND=s3 but this binary was built without the `s3` \
         cargo feature; rebuild with `--features s3` (falling back to local storage would \
         silently write export artifacts to an ephemeral container disk)"
            .to_string(),
    ))
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

    /// Build a store from the environment: `CARE_PATHWAY_BULK_ARTIFACT_DIR`, or
    /// a `care-pathway-bulk-artifacts` directory under the system temp dir when
    /// unset/blank.
    #[must_use]
    pub fn from_env() -> Self {
        let base = std::env::var("CARE_PATHWAY_BULK_ARTIFACT_DIR")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(
                || std::env::temp_dir().join("care-pathway-bulk-artifacts"),
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
/// backslash. Pure, so `put`/`get` and their tests share one definition.
///
/// Applied by **both** backends. On the filesystem an unsafe key is a path
/// traversal; on an object store it is a key-namespace escape that could
/// overwrite another tenant's prefix. The rule is the same either way, so
/// there is one definition rather than two that can drift apart.
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

/// S3-compatible object-store [`ArtifactStore`].
///
/// Works against AWS S3 and any API-compatible store (`MinIO`, Ceph RGW,
/// Cloudflare R2) by pointing `CARE_PATHWAY_BULK_S3_ENDPOINT` at it.
/// References are `s3://<bucket>/<key>` URLs, and
/// [`presigned_get`](ArtifactStore::presigned_get) issues the short-lived,
/// access-controlled download URL the shared doc §3 asks for.
#[cfg(feature = "s3")]
pub struct S3ArtifactStore {
    /// The configured S3 client.
    client: aws_sdk_s3::Client,
    /// Bucket every artifact is written to.
    bucket: String,
}

#[cfg(feature = "s3")]
impl S3ArtifactStore {
    /// Build a store from the environment.
    ///
    /// | Variable | Meaning |
    /// |---|---|
    /// | `CARE_PATHWAY_BULK_S3_BUCKET` | **required** — the bucket |
    /// | `CARE_PATHWAY_BULK_S3_ENDPOINT` | custom endpoint for a non-AWS store |
    /// | `CARE_PATHWAY_BULK_S3_REGION` | region (default `us-east-1`) |
    /// | `CARE_PATHWAY_BULK_S3_FORCE_PATH_STYLE` | path-style addressing (default **on**) |
    ///
    /// Credentials come from the standard AWS chain (environment, profile,
    /// container/instance role) — deliberately not from bespoke variables,
    /// so a deployment's existing secret management applies unchanged and
    /// no long-lived key needs to be minted for this service alone.
    ///
    /// Path-style addressing defaults **on** because the common
    /// self-hosted targets (`MinIO`, Ceph RGW) require it, and
    /// virtual-hosted style against them fails with a DNS error that reads
    /// like a network fault rather than a configuration mistake.
    ///
    /// # Errors
    ///
    /// When the bucket is not configured.
    pub async fn from_env() -> Result<Self> {
        let bucket = std::env::var("CARE_PATHWAY_BULK_S3_BUCKET")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| {
                Error::Message(
                    "CARE_PATHWAY_BULK_ARTIFACT_BACKEND=s3 requires \
                     CARE_PATHWAY_BULK_S3_BUCKET"
                        .to_string(),
                )
            })?;

        let region = std::env::var("CARE_PATHWAY_BULK_S3_REGION")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "us-east-1".to_string());

        let shared = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region))
            .load()
            .await;

        let mut builder = aws_sdk_s3::config::Builder::from(&shared)
            .force_path_style(force_path_style_from_env());
        if let Some(endpoint) = std::env::var("CARE_PATHWAY_BULK_S3_ENDPOINT")
            .ok()
            .filter(|s| !s.trim().is_empty())
        {
            builder = builder.endpoint_url(endpoint);
        }

        Ok(Self {
            client: aws_sdk_s3::Client::from_conf(builder.build()),
            bucket,
        })
    }

    /// Split an `s3://bucket/key` reference into its parts.
    ///
    /// A reference naming a **different** bucket is refused rather than
    /// fetched: the reference comes from a `bulk_jobs` row, and honouring
    /// an arbitrary bucket in it would turn a row edit into a read of any
    /// bucket this service's credentials can reach.
    fn split_reference<'a>(&self, reference: &'a str) -> Result<(&'a str, &'a str)> {
        let rest = reference
            .strip_prefix("s3://")
            .ok_or_else(|| Error::Message(format!("not an s3 artifact reference: {reference}")))?;
        let (bucket, key) = rest
            .split_once('/')
            .ok_or_else(|| Error::Message(format!("malformed s3 reference: {reference}")))?;
        if bucket != self.bucket {
            return Err(Error::Message(format!(
                "artifact reference names a different bucket than this store: {reference}"
            )));
        }
        if key.is_empty() {
            return Err(Error::Message(format!("empty s3 key: {reference}")));
        }
        Ok((bucket, key))
    }
}

/// Whether to use path-style addressing; default **on** (see
/// [`S3ArtifactStore::from_env`]).
#[cfg(feature = "s3")]
fn force_path_style_from_env() -> bool {
    std::env::var("CARE_PATHWAY_BULK_S3_FORCE_PATH_STYLE")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_none_or(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

#[cfg(feature = "s3")]
#[async_trait]
impl ArtifactStore for S3ArtifactStore {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<String> {
        // The same key rule as the filesystem backend: on an object store
        // an unsafe key is a namespace escape rather than a path
        // traversal, but it is no less a defect.
        if !is_safe_key(key) {
            return Err(Error::Message(format!(
                "refusing unsafe artifact key: {key}"
            )));
        }
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(bytes.to_vec().into())
            .send()
            .await
            .map_err(|e| Error::Message(format!("put artifact {key}: {e}")))?;
        Ok(format!("s3://{}/{key}", self.bucket))
    }

    async fn get(&self, reference: &str) -> Result<Vec<u8>> {
        let (_, key) = self.split_reference(reference)?;
        let object = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| Error::Message(format!("get artifact {reference}: {e}")))?;
        let data = object
            .body
            .collect()
            .await
            .map_err(|e| Error::Message(format!("read artifact body {reference}: {e}")))?;
        Ok(data.into_bytes().to_vec())
    }

    async fn presigned_get(&self, reference: &str, ttl_secs: u64) -> Result<Option<String>> {
        let (_, key) = self.split_reference(reference)?;
        // Bounded: an unbounded TTL on a URL to a clinical export is a
        // long-lived unauthenticated read of personal data. One hour is the
        // ceiling, matching the retention window bulk jobs already use.
        let ttl = std::time::Duration::from_secs(ttl_secs.clamp(1, 3600));
        let config = aws_sdk_s3::presigning::PresigningConfig::expires_in(ttl)
            .map_err(|e| Error::Message(format!("presigning config: {e}")))?;
        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|e| Error::Message(format!("presign artifact {reference}: {e}")))?;
        Ok(Some(request.uri().to_string()))
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
        let reference = store.put("jobs/abc/out.ndjson", b"x").await.unwrap();
        assert_eq!(store.presigned_get(&reference, 300).await.unwrap(), None);
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

    #[tokio::test]
    async fn put_rejects_an_unsafe_key() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalFsArtifactStore::new(dir.path());
        assert!(store.put("../escape.txt", b"x").await.is_err());
        assert!(store.put("/tmp/escape.txt", b"x").await.is_err());
    }

    #[tokio::test]
    async fn get_refuses_a_reference_outside_the_base() {
        // A real file outside the store base must not be readable through a
        // crafted file:// reference (SEC-B4 arbitrary-read guard).
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

    /// Asking for `s3` without the feature is an error, not a silent
    /// fallback to local storage — which would write a deployment's
    /// clinical export artifacts to an ephemeral container disk while
    /// appearing to succeed.
    #[cfg(not(feature = "s3"))]
    #[tokio::test]
    async fn s3_backend_without_the_feature_is_an_error_not_a_fallback() {
        let error = super::from_backend("s3")
            .await
            .err()
            .expect("s3 without the feature must fail");
        let message = error.to_string();
        assert!(
            message.contains("`s3` cargo feature"),
            "the error must name the missing feature: {message}"
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

    /// Round-trip against a **real** S3-compatible endpoint.
    ///
    /// `#[ignore]`d and opt-in: it needs a live object store, which CI
    /// does not have. Run it against `MinIO` with
    ///
    /// ```sh
    /// CARE_PATHWAY_BULK_S3_ENDPOINT=http://127.0.0.1:9000 \
    /// CARE_PATHWAY_BULK_S3_BUCKET=care-pathway-test \
    /// AWS_ACCESS_KEY_ID=minioadmin AWS_SECRET_ACCESS_KEY=minioadmin \
    /// AWS_REGION=us-east-1 \
    ///   cargo test --features s3 -- --ignored s3_round_trip
    /// ```
    ///
    /// This test exists because everything else about the S3 backend is
    /// verified only by the compiler. The key-safety rule and the
    /// reference-parsing rules below are unit-tested, but whether a `put`
    /// followed by a `get` actually returns the same bytes through a
    /// signed request cannot be established without an endpoint — and
    /// saying "the S3 backend is tested" on the strength of the parsing
    /// tests alone would be false.
    #[cfg(feature = "s3")]
    #[tokio::test]
    #[ignore = "requires a live S3-compatible endpoint; see the doc comment"]
    async fn s3_round_trip_against_a_live_endpoint() {
        let store = super::S3ArtifactStore::from_env()
            .await
            .expect("S3 configuration from the environment");
        let key = "tests/round-trip.ndjson";
        let reference = store
            .put(key, b"{\"resourceType\":\"PlanDefinition\"}\n")
            .await
            .expect("put");
        assert!(reference.starts_with("s3://"));
        let bytes = store.get(&reference).await.expect("get");
        assert!(String::from_utf8_lossy(&bytes).contains("PlanDefinition"));

        let url = store
            .presigned_get(&reference, 300)
            .await
            .expect("presign")
            .expect("the S3 backend issues presigned URLs");
        assert!(url.starts_with("http"), "presigned URL: {url}");
    }

    /// A reference naming a different bucket is refused rather than
    /// fetched. The reference comes from a `bulk_jobs` row, so honouring
    /// an arbitrary bucket in it would turn a row edit into a read of any
    /// bucket this service's credentials can reach.
    #[cfg(feature = "s3")]
    #[test]
    fn s3_reference_parsing_refuses_a_foreign_bucket() {
        // Constructed directly rather than through `from_env`, so this
        // runs with no AWS configuration present.
        let store = super::S3ArtifactStore {
            client: aws_sdk_s3::Client::from_conf(
                aws_sdk_s3::config::Builder::new()
                    .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
                    .region(aws_sdk_s3::config::Region::new("us-east-1"))
                    .build(),
            ),
            bucket: "mine".to_string(),
        };
        assert!(store.split_reference("s3://mine/jobs/a/out.ndjson").is_ok());
        assert!(store.split_reference("s3://theirs/secrets").is_err());
        assert!(store.split_reference("file:///etc/passwd").is_err());
        assert!(store.split_reference("s3://mine").is_err());
        assert!(store.split_reference("s3://mine/").is_err());
    }

    /// Path-style addressing defaults on, because `MinIO` and Ceph RGW
    /// require it and virtual-hosted style against them fails with a DNS
    /// error that reads like a network fault rather than a misconfiguration.
    #[cfg(feature = "s3")]
    #[test]
    fn path_style_defaults_on() {
        // With the variable unset (the CI case) the default must be on.
        if std::env::var("CARE_PATHWAY_BULK_S3_FORCE_PATH_STYLE").is_err() {
            assert!(super::force_path_style_from_env());
        }
    }
}
