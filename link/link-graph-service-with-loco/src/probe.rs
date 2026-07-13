//! Lazy verify-on-read (spec T-10 / design §5.1) — the **interim**
//! integrity path until the durable bus feeds presence via events.
//!
//! When a read surfaces an edge whose endpoint presence is still unknown
//! (no `created`/`deleted` event has been consumed for it), the aggregator
//! makes a one-shot `GET` to that endpoint's source service, caches the
//! verdict in `entity_presence`, and recomputes the incident edge status —
//! so `unverified` edges settle to `verified` / `dangling` on first read
//! even before the bus is live.
//!
//! The HTTP call sits behind the [`PresenceProbe`] trait so the read-path
//! logic ([`verify_unknown`]) is testable with a mock. The per-entity
//! probe **URL template** is configured via the environment (no hardcoded
//! service hosts or plural paths):
//! `LINK_GRAPH_PROBE_URL_<ENTITY>` = e.g.
//! `http://person-service/api/persons/{id}` (the `{id}` placeholder is
//! substituted with the record pid). An entity with no template is not
//! probed. All of it is gated by `LINK_GRAPH_LAZY_VERIFY` (off by default).

use async_trait::async_trait;
use loco_rs::prelude::*;
use sea_orm::ConnectionTrait;

use entity_ref::{EntityRef, EntityType};

use crate::models::{edges, entity_presence};

/// The outcome of probing a record's presence at its source service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The source service returned success — the record exists.
    Alive,
    /// The source service returned `404` — the record is gone.
    Absent,
    /// Presence could not be determined (no template, network error, or
    /// an unexpected status) — leave it unknown, retry on a later read.
    Unknown,
}

/// A source of truth for whether a record still exists at its owning
/// service. Injectable, so [`verify_unknown`] is testable with a mock.
#[async_trait]
pub trait PresenceProbe: Send + Sync {
    /// Probe the presence of `r` at its source service.
    async fn probe(&self, r: &EntityRef) -> ProbeOutcome;
}

/// The env var holding the by-id URL template for an entity type, e.g.
/// `LINK_GRAPH_PROBE_URL_CARE_PATHWAY`.
#[must_use]
pub fn probe_url_env(entity_type: EntityType) -> String {
    format!(
        "LINK_GRAPH_PROBE_URL_{}",
        entity_type.as_str().to_ascii_uppercase()
    )
}

/// Substitute a ref's id into a URL template's `{id}` placeholder. Pure,
/// so it is unit-testable without touching the environment.
#[must_use]
pub fn substitute_id(template: &str, r: &EntityRef) -> String {
    template.replace("{id}", &r.id.to_string())
}

/// Resolve the concrete probe URL for `r`: read its entity's URL template
/// from the environment and substitute `{id}`. `None` when no template is
/// configured (that entity is not probed).
#[must_use]
pub fn probe_url(r: &EntityRef) -> Option<String> {
    let template = std::env::var(probe_url_env(r.entity_type))
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    Some(substitute_id(&template, r))
}

/// Whether lazy verify-on-read is active (`LINK_GRAPH_LAZY_VERIFY`).
#[must_use]
pub fn enabled() -> bool {
    std::env::var("LINK_GRAPH_LAZY_VERIFY").is_ok_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

/// Map a probe response status to an outcome (SEC-B11): `2xx` ⇒
/// [`ProbeOutcome::Alive`], `404` ⇒ [`ProbeOutcome::Absent`], anything else
/// ⇒ [`ProbeOutcome::Unknown`]. Crucially a **3xx redirect** maps to
/// `Unknown` — the probe client does not follow redirects (see
/// [`no_redirect_client`]), so a source service cannot steer the probe to an
/// arbitrary host (SSRF-via-redirect). Pure, so it is unit-testable.
#[must_use]
fn outcome_from_status(status: reqwest::StatusCode) -> ProbeOutcome {
    if status.is_success() {
        ProbeOutcome::Alive
    } else if status == reqwest::StatusCode::NOT_FOUND {
        ProbeOutcome::Absent
    } else {
        // 3xx (redirects — NOT followed), 4xx (except 404), 5xx.
        ProbeOutcome::Unknown
    }
}

/// The shared, **non-redirecting** HTTP client for presence probes
/// (SEC-B11). Disabling redirects closes the SSRF-via-redirect vector: the
/// only host ever contacted is the one in the operator-configured
/// `LINK_GRAPH_PROBE_URL_<ENTITY>` template — a source service returning a
/// `3xx` to an internal address (cloud metadata, another service) can no
/// longer make the aggregator follow it. Built once and reused.
fn no_redirect_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("build the non-redirecting presence-probe client")
    })
}

/// The real HTTP probe: a one-shot, **non-redirecting** `GET` to the
/// configured URL — `2xx` ⇒ [`ProbeOutcome::Alive`], `404` ⇒
/// [`ProbeOutcome::Absent`], anything else (a `3xx` redirect, other status,
/// no template, or a network error) ⇒ [`ProbeOutcome::Unknown`].
pub struct HttpPresenceProbe;

#[async_trait]
impl PresenceProbe for HttpPresenceProbe {
    async fn probe(&self, r: &EntityRef) -> ProbeOutcome {
        let Some(url) = probe_url(r) else {
            return ProbeOutcome::Unknown;
        };
        match no_redirect_client().get(&url).send().await {
            Ok(resp) => outcome_from_status(resp.status()),
            Err(_) => ProbeOutcome::Unknown,
        }
    }
}

/// Lazily verify the presence of the given endpoints whose presence is
/// still **unknown**, caching each resolved verdict in `entity_presence`
/// and recomputing incident edge status (spec §6 FR-11). Endpoints already
/// known (alive or deleted) are skipped, as are those the probe cannot
/// resolve. Returns the number of endpoints newly resolved.
///
/// Deduplicates `refs` so each endpoint is probed at most once per call.
///
/// # Errors
///
/// When a presence lookup, cache write, or status recompute fails.
pub async fn verify_unknown<C: ConnectionTrait>(
    db: &C,
    probe: &dyn PresenceProbe,
    refs: &[EntityRef],
) -> ModelResult<u64> {
    let mut seen = std::collections::HashSet::new();
    let mut resolved = 0u64;
    for r in refs {
        if !seen.insert(*r) {
            continue;
        }
        if entity_presence::Model::alive(db, r).await?.is_some() {
            continue; // already observed — nothing to verify
        }
        let alive = match probe.probe(r).await {
            ProbeOutcome::Alive => true,
            ProbeOutcome::Absent => false,
            ProbeOutcome::Unknown => continue,
        };
        entity_presence::Model::mark(db, r, alive, 0).await?;
        edges::Model::recompute_status_for(db, r).await?;
        resolved += 1;
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn r(t: EntityType, n: u128) -> EntityRef {
        EntityRef::new(t, Uuid::from_u128(n))
    }

    #[test]
    fn probe_url_env_is_the_uppercased_entity_token() {
        assert_eq!(
            probe_url_env(EntityType::Person),
            "LINK_GRAPH_PROBE_URL_PERSON"
        );
        assert_eq!(
            probe_url_env(EntityType::CarePathway),
            "LINK_GRAPH_PROBE_URL_CARE_PATHWAY"
        );
    }

    #[test]
    fn substitute_id_fills_the_placeholder() {
        let want = format!("http://thing-svc/api/things/{}", Uuid::from_u128(42));
        assert_eq!(
            substitute_id(
                "http://thing-svc/api/things/{id}",
                &r(EntityType::Thing, 42)
            ),
            want
        );
        // A template without the placeholder is returned unchanged.
        assert_eq!(
            substitute_id("http://x/none", &r(EntityType::Thing, 1)),
            "http://x/none"
        );
    }

    /// SEC-B11: a `3xx` redirect maps to `Unknown` (the client does not
    /// follow it), closing SSRF-via-redirect; `2xx`⇒Alive, `404`⇒Absent, and
    /// other errors ⇒ Unknown.
    #[test]
    fn outcome_from_status_treats_redirects_as_unknown() {
        use super::outcome_from_status;
        use reqwest::StatusCode;
        assert_eq!(outcome_from_status(StatusCode::OK), ProbeOutcome::Alive);
        assert_eq!(
            outcome_from_status(StatusCode::NO_CONTENT),
            ProbeOutcome::Alive
        );
        assert_eq!(
            outcome_from_status(StatusCode::NOT_FOUND),
            ProbeOutcome::Absent
        );
        // Redirects are NOT followed → Unknown (the SSRF-via-redirect guard).
        assert_eq!(
            outcome_from_status(StatusCode::MOVED_PERMANENTLY),
            ProbeOutcome::Unknown
        );
        assert_eq!(
            outcome_from_status(StatusCode::FOUND),
            ProbeOutcome::Unknown
        );
        assert_eq!(
            outcome_from_status(StatusCode::TEMPORARY_REDIRECT),
            ProbeOutcome::Unknown
        );
        // Server / other client errors are inconclusive, not Absent.
        assert_eq!(
            outcome_from_status(StatusCode::INTERNAL_SERVER_ERROR),
            ProbeOutcome::Unknown
        );
        assert_eq!(
            outcome_from_status(StatusCode::UNAUTHORIZED),
            ProbeOutcome::Unknown
        );
    }

    /// The probe client builds and disables redirect-following.
    #[test]
    fn no_redirect_client_builds() {
        // Smoke: constructing the shared client must not panic.
        let _ = super::no_redirect_client();
    }
}
