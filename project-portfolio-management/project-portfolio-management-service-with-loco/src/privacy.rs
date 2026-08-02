//! Privacy: field masking and the GDPR right-of-access export.
//!
//! ## What is sensitive about a plan
//!
//! Most of a plan record is operational, not personal: `name`, `code`,
//! `goals`, `status`, dates, `keywords`, `tags`, `identifiers`,
//! `relationships`, and the containment `parent_ref` are exactly the
//! content the registry and the project-management tooling exist to
//! serve up, and masking them would defeat both for no privacy gain —
//! the same reasoning organization applies to its LEI and care-pathway
//! to its condition codes.
//!
//! Three fields are not operational, they are identity:
//!
//! - **`lead_ref`** — the plan lead, a `person:<id>` / `worker:<id>`
//!   [`EntityRef`]-shaped reference to a named individual. The most
//!   directly personal field on a `Plan`, so it is **dropped entirely**
//!   rather than partially shown: unlike a phone number or a provider
//!   name, a partial UUID has no legitimate "still recognisable" value
//!   to an operator — the plan is already identified by its `name` and
//!   `code`.
//! - **`owner_org_id` / `owner_org_name`** — the sponsoring
//!   organisation. Institutional rather than personal, but still worth
//!   an obligation-driven redaction from a caller who should see that a
//!   plan exists (and what it is) without learning who sponsors it —
//!   tail-preserving, so a review screen can still tell two records
//!   apart, the same treatment organization gives `telephone`/`email`
//!   and care-pathway gives `provider_name`/`provider_id`.
//!
//! Lower sensitivity than organization/case/care-pathway overall — a
//! plan names an organisation and (optionally) a lead, not a data
//! subject with contact details or a clinical/governmental history —
//! which is why this module is the thinnest of the four.
//!
//! ## Consent
//!
//! As with the other entity services, the shared privacy contract's
//! consent model ([`agents/share/privacy.md`]) is about a **data
//! subject** granting a purpose. A plan is not a data subject; the
//! natural person behind `lead_ref` is the person/worker service's to
//! record. This crate has no consent model for the same reason
//! organization does not.
//!
//! Everything here is pure; the endpoints live in
//! [`crate::controllers::plans`].
//!
//! [`EntityRef`]: ../../../link/entity-ref-rust-crate
//! [`agents/share/privacy.md`]: ../../../agents/share/privacy.md

use project_portfolio_management_matcher::Plan;

/// Return a copy of `plan` with the sensitive fields masked (see the
/// module docs for what counts as sensitive here and why the set is
/// thin).
#[must_use]
pub fn mask_plan(plan: &Plan) -> Plan {
    let mut masked = plan.clone();
    // The most directly personal field: dropped, not partially shown.
    masked.lead_ref = None;
    if let Some(ref name) = masked.owner_org_name {
        masked.owner_org_name = Some(mask_value(name, 4));
    }
    if let Some(ref id) = masked.owner_org_id {
        masked.owner_org_id = Some(mask_value(id, 4));
    }
    masked
}

/// Mask a string, keeping only its last `visible_chars` characters.
///
/// Alphanumerics in the hidden prefix become `*`; punctuation is kept so
/// the shape stays readable. Values no longer than `visible_chars` are
/// returned unchanged — there is nothing to hide behind.
///
/// Counts `char`s, not bytes, so a multibyte value is masked correctly
/// and can never be sliced across a UTF-8 boundary.
fn mask_value(value: &str, visible_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= visible_chars {
        return value.to_string();
    }
    let hidden = char_count - visible_chars;
    value
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i < hidden && c.is_alphanumeric() {
                '*'
            } else {
                c
            }
        })
        .collect()
}

/// Build the GDPR right-of-access payload for one plan.
///
/// An **envelope**, not the bare record: an access request has to be
/// answerable as "here is everything held about this subject, as of
/// then", so the response states what was exported, when, and whether
/// any of it was redacted. `masked` is the honest part — a masked
/// export is a *partial* answer, and the caller must be able to tell
/// which they received.
#[must_use]
pub fn export_plan(plan: &Plan, pid: &str, masked: bool) -> serde_json::Value {
    let record = if masked {
        mask_plan(plan)
    } else {
        plan.clone()
    };
    serde_json::json!({
        "entity": "plan",
        "pid": pid,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "masked": masked,
        "record": serde_json::to_value(&record).unwrap_or(serde_json::Value::Null),
        "note": if masked {
            "Partial export: lead_ref is dropped and owner_org_id/owner_org_name are \
             redacted for this caller."
        } else {
            "Complete export of the stored plan record."
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fully-populated plan to redact.
    fn apollo_plan() -> Plan {
        Plan {
            owner_org_name: Some("Acme Astronautics".into()),
            owner_org_id: Some("organization:9a2f1e2a-0000".into()),
            lead_ref: Some("person:0c4f1e2a-0000-4000-8000-000000000000".into()),
            code: Some("PROJ-2026".into()),
            ..Plan::new("Apollo platform migration")
        }
    }

    /// `lead_ref` — the most directly personal field — is dropped
    /// entirely, not partially shown.
    #[test]
    fn lead_ref_is_dropped_entirely() {
        assert_eq!(mask_plan(&apollo_plan()).lead_ref, None);
    }

    /// The owner org fields keep their last four characters and hide
    /// everything before that.
    #[test]
    fn owner_org_is_masked_to_its_tail() {
        let masked = mask_plan(&apollo_plan());
        let name = masked.owner_org_name.expect("owner_org_name");
        assert!(name.ends_with("tics"), "keeps the last four chars: {name}");
        assert_ne!(name, "Acme Astronautics", "must actually be redacted");
        assert!(name.contains('*'), "hides the rest: {name}");

        let id = masked.owner_org_id.expect("owner_org_id");
        assert!(id.ends_with("0000"), "keeps the last four chars: {id}");
        assert_ne!(
            id, "organization:9a2f1e2a-0000",
            "must actually be redacted"
        );
        assert!(id.contains('*'), "hides the rest: {id}");
    }

    /// Operational content is never masked — redacting it would defeat
    /// the registry for no privacy gain.
    #[test]
    fn operational_content_survives_masking() {
        let masked = mask_plan(&apollo_plan());
        assert_eq!(masked.name, "Apollo platform migration");
        assert_eq!(masked.code.as_deref(), Some("PROJ-2026"));
    }

    /// Masking a plan with no owner/lead recorded is a no-op rather
    /// than an error or an invented value.
    #[test]
    fn a_bare_plan_is_unchanged() {
        let bare = Plan::new("Bare Plan");
        let masked = mask_plan(&bare);
        assert_eq!(masked.name, "Bare Plan");
        assert_eq!(masked.owner_org_name, None);
        assert_eq!(masked.owner_org_id, None);
        assert_eq!(masked.lead_ref, None);
    }

    /// Values no longer than the visible tail are left alone, and
    /// masking counts characters rather than bytes.
    #[test]
    fn mask_value_is_char_safe() {
        assert_eq!(mask_value("PROJ-2026", 4), "****-2026");
        assert_eq!(mask_value("abc", 4), "abc");
        assert_eq!(mask_value("naïve12", 4), "***ve12");
    }

    /// The export envelope names the subject, says when, and — the part
    /// that matters — says whether what it carries was redacted.
    #[test]
    fn export_envelope_declares_whether_it_is_partial() {
        let full = export_plan(&apollo_plan(), "pid-1", false);
        assert_eq!(full["entity"], "plan");
        assert_eq!(full["pid"], "pid-1");
        assert_eq!(full["masked"], false);
        assert_eq!(full["record"]["owner_org_name"], "Acme Astronautics");
        assert!(full["exported_at"].as_str().is_some_and(|s| !s.is_empty()));

        let partial = export_plan(&apollo_plan(), "pid-1", true);
        assert_eq!(partial["masked"], true);
        assert!(partial["record"]["lead_ref"].is_null());
        assert!(
            partial["note"]
                .as_str()
                .is_some_and(|n| n.contains("Partial")),
            "a masked export must say so"
        );
    }
}
