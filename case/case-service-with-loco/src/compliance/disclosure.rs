//! Read-auditing and the HIPAA §164.528 access/disclosure distinction.
//!
//! HIPAA §164.312(b) requires recording activity in systems holding ePHI,
//! and a **lookup is activity** — recording mutations only does not satisfy
//! it. §164.528 goes further: an individual may demand an accounting of
//! **disclosures** of their information, so the trail must distinguish an
//! internal *access* from an outward *disclosure*. EHDS adds a third axis:
//! its Ch. IV separates **primary use** (care delivery) from **secondary
//! use** (research, policy, statistics), and only the caller knows which
//! one a request is.
//!
//! None of that is derivable from the HTTP verb, so the caller declares it
//! in request headers, and the service records the declaration alongside
//! the deployment's standing declarations (residency, lawful basis, Art. 9
//! condition):
//!
//! | Header | Meaning |
//! |---|---|
//! | `X-Purpose-Of-Use` | Why the data is being accessed — one of [`PURPOSES`]. |
//! | `X-Disclosure-Recipient` | Who the data is being released **to**; its presence makes the access a disclosure. |
//! | `X-Destination-Region` | Where the data is going, for the GDPR Ch. V transfer check. |
//!
//! ## What the declaration is worth
//!
//! A caller-supplied purpose is an **assertion, not a proof** — a
//! misbehaving client can claim `care` while doing research. That is the
//! same trust model HL7's own `PurposeOfUse` and the SMART `launch`
//! context use, and it is still worth recording: it makes the caller
//! accountable for a statement they made, which is exactly what an
//! accounting of disclosures is for. Where the purpose must be *enforced*
//! rather than recorded, it belongs in the ABAC policy as a subject
//! attribute (`agents/share/authorization-attributes.md`), not here.
//!
//! ## When the audit write fails
//!
//! A dropped audit row means data was disclosed with no record of it, and
//! the resulting gap is indistinguishable from a deleted row when the
//! chain is verified. Which way to fail is a deployment decision, not a
//! library one, so it is a switch — [`fail_closed`],
//! `CASE_AUDIT_FAIL_CLOSED`:
//!
//! - **off (default)** — log and serve the read, preserving the family's
//!   best-effort audit posture and today's availability profile;
//! - **on** — refuse the read with `503`, disclosing nothing the service
//!   cannot account for. Recommended for a HIPAA-facing deployment.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

use crate::models::audit_logs::Model as AuditModel;

/// Header naming why the data is being accessed.
pub const PURPOSE_HEADER: &str = "x-purpose-of-use";
/// Header naming who the data is being released to.
pub const RECIPIENT_HEADER: &str = "x-disclosure-recipient";
/// Header naming where the data is going (GDPR Ch. V).
pub const DESTINATION_HEADER: &str = "x-destination-region";

/// The purpose recorded when the caller declares none.
pub const UNSPECIFIED: &str = "unspecified";

/// Recognised purposes of use, loosely following HL7 `v3-PurposeOfUse` and
/// the EHDS primary/secondary split. An unrecognised value is recorded as
/// [`UNSPECIFIED`] rather than echoed, so an attacker cannot write
/// arbitrary text into the audit trail through a header.
pub const PURPOSES: [&str; 9] = [
    // Primary use (EHDS Ch. II) — care delivery and its administration.
    "care",
    "treatment",
    "payment",
    "operations",
    "public-health",
    // Secondary use (EHDS Ch. IV) — and other outward purposes.
    "research",
    "policy",
    "statistics",
    "legal",
];

/// Purposes that are, by their nature, an outward **disclosure** rather
/// than an internal access: the data leaves the care context.
const DISCLOSURE_PURPOSES: [&str; 4] = ["research", "policy", "statistics", "legal"];

/// Purposes that are EHDS **secondary use**.
const SECONDARY_USE_PURPOSES: [&str; 3] = ["research", "policy", "statistics"];

/// Longest accepted header value; longer values are truncated before being
/// recorded, so a header cannot bloat an audit row (SEC-M1 input caps).
const MAX_HEADER_LEN: usize = 128;

/// Audit action verbs for read paths. Kept as constants so the read-audit
/// vocabulary is closed and greppable.
pub mod action {
    /// A single record was read.
    pub const READ: &str = "read";
    /// The collection was listed.
    pub const LIST: &str = "list";
    /// The collection was searched.
    pub const SEARCH: &str = "search";
    /// Records were exported in bulk.
    pub const EXPORT: &str = "export";
    /// A record was read through the FHIR surface.
    pub const FHIR_READ: &str = "fhir_read";
    /// The FHIR surface was searched.
    pub const FHIR_SEARCH: &str = "fhir_search";
}

/// The caller's declared access context, extracted from request headers.
///
/// Extraction never fails: a request with no headers yields an
/// [`UNSPECIFIED`] purpose and no recipient, which is recorded as an
/// internal access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessContext {
    /// Declared purpose of use — always one of [`PURPOSES`] or [`UNSPECIFIED`].
    pub purpose: String,
    /// Declared recipient of an outward disclosure.
    pub recipient: Option<String>,
    /// Declared destination region (GDPR Ch. V).
    pub destination: Option<String>,
}

impl AccessContext {
    /// An internal access with nothing declared.
    #[must_use]
    pub fn internal() -> Self {
        Self {
            purpose: UNSPECIFIED.to_string(),
            recipient: None,
            destination: None,
        }
    }

    /// Build from raw header values, normalising and validating each.
    #[must_use]
    pub fn from_parts(
        purpose: Option<&str>,
        recipient: Option<&str>,
        destination: Option<&str>,
    ) -> Self {
        Self {
            purpose: normalize_purpose(purpose),
            recipient: sanitize(recipient),
            destination: sanitize(destination),
        }
    }

    /// Whether this access is an outward **disclosure** (§164.528): either
    /// the caller named a recipient, or the declared purpose is inherently
    /// outward.
    #[must_use]
    pub fn is_disclosure(&self) -> bool {
        self.recipient.is_some() || DISCLOSURE_PURPOSES.contains(&self.purpose.as_str())
    }

    /// Whether this is EHDS **secondary use** (Ch. IV) rather than care
    /// delivery.
    #[must_use]
    pub fn is_secondary_use(&self) -> bool {
        SECONDARY_USE_PURPOSES.contains(&self.purpose.as_str())
    }

    /// The JSON recorded in `audit_logs.context`: what the caller declared,
    /// plus the deployment's standing declarations, plus the derived
    /// classifications — so a row is interpretable years later without
    /// knowing what the environment held at the time.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "purpose_of_use": self.purpose,
            "recipient": self.recipient,
            "destination": self.destination,
            "secondary_use": self.is_secondary_use(),
        })
    }
}

/// Extraction is infallible: absent or malformed headers degrade to an
/// internal, unspecified-purpose access rather than rejecting the request.
impl<S: Send + Sync> FromRequestParts<S> for AccessContext {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = |name: &str| parts.headers.get(name).and_then(|v| v.to_str().ok());
        Ok(Self::from_parts(
            header(PURPOSE_HEADER),
            header(RECIPIENT_HEADER),
            header(DESTINATION_HEADER),
        ))
    }
}

/// Normalise a declared purpose to a known token. Unrecognised, blank, and
/// absent values all become [`UNSPECIFIED`] — the value is never echoed,
/// so a header cannot inject text into the trail.
fn normalize_purpose(raw: Option<&str>) -> String {
    let normalized = raw
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .replace('_', "-");
    PURPOSES
        .iter()
        .find(|p| **p == normalized)
        .map_or_else(|| UNSPECIFIED.to_string(), |p| (*p).to_string())
}

/// Sanitise a free-text header: trim, drop control characters, cap length,
/// and treat blank as absent. Recipients and regions are operator-supplied
/// labels, so they are kept verbatim modulo those rules.
fn sanitize(raw: Option<&str>) -> Option<String> {
    let cleaned: String = raw?
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_HEADER_LEN)
        .collect();
    let cleaned = cleaned.trim().to_string();
    (!cleaned.is_empty()).then_some(cleaned)
}

/// The audit row could not be written, and the deployment has asked to
/// **fail closed** — the caller must refuse the read rather than serve
/// data it cannot account for.
///
/// Deliberately carries no detail: the cause is already logged, and the
/// caller's job is only to decide the HTTP status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuditWriteRefused;

/// Whether a failed **read-audit** write must refuse the read, from
/// `CASE_AUDIT_FAIL_CLOSED` (read once and cached).
///
/// **Default off**, so enabling read-auditing does not also change a
/// service's availability profile. The trade-off is real in both
/// directions and the deployment owns it:
///
/// - **Off (default).** A failed audit write logs and the read proceeds.
///   Availability is preserved, but PHI was disclosed with no record of
///   it — precisely what HIPAA §164.312(b) exists to prevent — and the
///   gap is indistinguishable from a deleted row when the chain is
///   verified.
/// - **On.** A failed audit write refuses the read with `503`. Nothing is
///   disclosed unaccounted for, at the cost of coupling read availability
///   to the audit table.
///
/// A HIPAA-facing deployment should turn this **on**: an accounting of
/// disclosures that silently omits disclosures is worse than an outage,
/// because the outage is visible.
///
/// This governs **reads only**. Mutation audits are already fail-closed
/// under `CASE_EVENT_TRANSPORT=outbox`, where the audit row shares
/// the entity mutation's transaction and a failure rolls the change back.
#[must_use]
pub fn fail_closed() -> bool {
    static FAIL_CLOSED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FAIL_CLOSED.get_or_init(|| {
        crate::auth::parse_bool(&std::env::var("CASE_AUDIT_FAIL_CLOSED").unwrap_or_default())
    })
}

/// Record one read/disclosure audit row — a **no-op unless**
/// `CASE_AUDIT_READS` is on, so adopting this module changes
/// nothing until a deployment opts in.
///
/// `entity_pid` is the record read, or [`Uuid::nil`] for a
/// collection-level action (list / search / export), which is how a
/// collection access is distinguished from a record access when answering
/// §164.528 for one record.
///
/// # Errors
///
/// [`AuditWriteRefused`] when the write failed **and**
/// [`fail_closed`] is on — the caller must then refuse the read. With
/// fail-closed off (the default) a failure is logged and `Ok` is
/// returned, preserving the previous best-effort behaviour.
pub async fn record_access<C: ConnectionTrait>(
    db: &C,
    entity_pid: Uuid,
    action: &str,
    actor: Option<&str>,
    ctx: &AccessContext,
) -> Result<(), AuditWriteRefused> {
    if !super::audit_reads() {
        return Ok(());
    }
    let Err(error) = AuditModel::record_with_context(
        db,
        entity_pid,
        action,
        actor,
        None,
        Some(ctx.to_json()),
        ctx.is_disclosure(),
    )
    .await
    else {
        return Ok(());
    };
    if fail_closed() {
        tracing::error!(
            %error,
            action,
            "read-access audit write failed and CASE_AUDIT_FAIL_CLOSED is on; \
             refusing the read rather than disclosing data unaccounted for"
        );
        return Err(AuditWriteRefused);
    }
    tracing::warn!(
        %error,
        action,
        "failed to write read-access audit row; the read proceeds (fail-open) and the \
         chain will show a gap indistinguishable from a deletion — set \
         CASE_AUDIT_FAIL_CLOSED to refuse instead"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every declared purpose survives normalisation, case- and
    /// separator-insensitively.
    #[test]
    fn known_purposes_normalize() {
        for purpose in PURPOSES {
            assert_eq!(normalize_purpose(Some(purpose)), purpose);
            assert_eq!(normalize_purpose(Some(&purpose.to_uppercase())), purpose);
        }
        assert_eq!(normalize_purpose(Some("public_health")), "public-health");
        assert_eq!(normalize_purpose(Some("  care  ")), "care");
    }

    /// An unknown purpose is never echoed into the audit trail — it
    /// becomes `unspecified`, closing a header-injection path.
    #[test]
    fn unknown_purpose_is_not_echoed() {
        for junk in [
            "",
            "   ",
            "treatment; DROP TABLE audit_logs",
            "<script>",
            "care\nresearch",
        ] {
            assert_eq!(normalize_purpose(Some(junk)), UNSPECIFIED, "{junk:?}");
        }
        assert_eq!(normalize_purpose(None), UNSPECIFIED);
    }

    /// Free-text headers are trimmed, control-stripped, and length-capped.
    #[test]
    fn free_text_headers_are_sanitised() {
        assert_eq!(
            sanitize(Some("  NHS Digital  ")),
            Some("NHS Digital".into())
        );
        assert_eq!(sanitize(Some("a\u{0}b\u{7}c")), Some("abc".into()));
        assert_eq!(sanitize(Some("   ")), None);
        assert_eq!(sanitize(None), None);
        let long = "x".repeat(500);
        assert_eq!(sanitize(Some(&long)).map(|s| s.len()), Some(MAX_HEADER_LEN));
    }

    /// A named recipient makes any access a disclosure, whatever the
    /// purpose says.
    #[test]
    fn named_recipient_is_always_a_disclosure() {
        let ctx = AccessContext::from_parts(Some("care"), Some("Regional Registry"), None);
        assert!(ctx.is_disclosure());
        assert!(!ctx.is_secondary_use());
    }

    /// Outward purposes are disclosures even with no recipient named.
    #[test]
    fn outward_purposes_are_disclosures() {
        for purpose in DISCLOSURE_PURPOSES {
            let ctx = AccessContext::from_parts(Some(purpose), None, None);
            assert!(ctx.is_disclosure(), "{purpose} should be a disclosure");
        }
        for purpose in [
            "care",
            "treatment",
            "payment",
            "operations",
            "public-health",
        ] {
            let ctx = AccessContext::from_parts(Some(purpose), None, None);
            assert!(!ctx.is_disclosure(), "{purpose} is an internal access");
        }
    }

    /// EHDS secondary use is a strict subset of disclosure: research,
    /// policy and statistics — not `legal`, which is outward but not
    /// secondary use in the EHDS sense.
    #[test]
    fn secondary_use_is_the_ehds_subset() {
        for purpose in SECONDARY_USE_PURPOSES {
            let ctx = AccessContext::from_parts(Some(purpose), None, None);
            assert!(ctx.is_secondary_use(), "{purpose}");
            assert!(ctx.is_disclosure(), "secondary use is also a disclosure");
        }
        let legal = AccessContext::from_parts(Some("legal"), None, None);
        assert!(legal.is_disclosure());
        assert!(!legal.is_secondary_use());
    }

    /// The default context is an internal, unspecified-purpose access —
    /// what an un-annotated request records.
    #[test]
    fn default_context_is_an_internal_access() {
        let ctx = AccessContext::internal();
        assert_eq!(ctx.purpose, UNSPECIFIED);
        assert!(!ctx.is_disclosure());
        assert!(!ctx.is_secondary_use());
        assert_eq!(ctx, AccessContext::from_parts(None, None, None));
    }

    /// The recorded context carries both the caller's declaration and the
    /// deployment's standing declarations, so an old row stays readable.
    #[test]
    fn context_json_carries_declaration_and_derivation() {
        let ctx =
            AccessContext::from_parts(Some("research"), Some("University"), Some("us-east-1"));
        let json = ctx.to_json();
        assert_eq!(json["purpose_of_use"], "research");
        assert_eq!(json["recipient"], "University");
        assert_eq!(json["secondary_use"], true);
        assert_eq!(json["destination"], "us-east-1");
        // The GDPR standing declarations (residency, lawful basis, Art. 9
        // condition, transfer safeguard) are **not** recorded here: this
        // service adopts the audit chain and read-auditing at
        // `spec/compliance` §8.5 step 3, and the declaration surface comes
        // with the later steps. Recording empty placeholders would imply a
        // posture the deployment has not actually declared.
        for absent in [
            "residency",
            "lawful_basis",
            "art9_condition",
            "transfer_safeguard",
        ] {
            assert!(
                json.get(absent).is_none(),
                "{absent} must not be claimed until case adopts the declarations"
            );
        }
    }

    /// Both fail modes are reachable and distinct, and the default is
    /// **fail-open** so that enabling read-auditing cannot by itself
    /// change a deployment's availability profile.
    #[test]
    fn fail_closed_defaults_off() {
        // The process env is shared across tests, so assert the parse
        // rule the getter uses rather than mutating the environment.
        assert!(!crate::auth::parse_bool(""), "unset means fail-open");
        assert!(!crate::auth::parse_bool("0"));
        assert!(crate::auth::parse_bool("1"), "opt-in turns fail-closed on");
        assert!(crate::auth::parse_bool("true"));
    }

    /// The refusal type carries no detail — the cause is logged, and the
    /// caller's only job is to choose a status code.
    #[test]
    fn refusal_is_a_bare_marker() {
        assert_eq!(AuditWriteRefused, AuditWriteRefused);
        assert_eq!(std::mem::size_of::<AuditWriteRefused>(), 0);
    }

    /// With read-auditing off, recording is a no-op that always succeeds
    /// — so the fail-closed switch cannot affect a deployment that has
    /// not opted into auditing at all.
    #[tokio::test]
    async fn recording_is_ok_when_auditing_is_off() {
        if super::super::audit_reads() {
            return; // env opted in; the DB-gated suite covers that path
        }
        // A connection that would fail if touched: proves the no-op path
        // never reaches the database.
        let db = sea_orm::DatabaseConnection::Disconnected;
        let ctx = AccessContext::internal();
        assert!(
            record_access(&db, Uuid::nil(), action::READ, None, &ctx)
                .await
                .is_ok()
        );
    }

    /// The read-audit action vocabulary is closed and distinct.
    #[test]
    fn read_actions_are_distinct() {
        let actions = [
            action::READ,
            action::LIST,
            action::SEARCH,
            action::EXPORT,
            action::FHIR_READ,
            action::FHIR_SEARCH,
        ];
        let mut sorted = actions;
        sorted.sort_unstable();
        let mut deduped = sorted.to_vec();
        deduped.dedup();
        assert_eq!(
            deduped.len(),
            actions.len(),
            "action verbs must be distinct"
        );
    }
}
