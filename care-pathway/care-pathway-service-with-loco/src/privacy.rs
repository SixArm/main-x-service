//! Privacy: field masking and the GDPR right-of-access export.
//!
//! ## What is sensitive about a care pathway — and what is not
//!
//! A [`CarePathway`] is a **template**: a named, evidence-based plan of
//! care for a condition or patient group (`care_pathway_matcher`'s own
//! module docs). It names no patient. Its `name`, `condition_codes`,
//! `interventions`, and `keywords` are the clinical content the registry
//! exists to serve up, and its `identifiers` (DOI, Wikidata, guideline
//! id, provider-scoped pathway code) are institutional / bibliographic,
//! not personal — none of them is the fiscal-identifier-shaped field
//! organization's privacy module redacts. Masking any of that would
//! defeat the registry for no privacy gain, exactly as masking an LEI
//! would for organization.
//!
//! What *is* masked here is **`provider_name` / `provider_id`** — which
//! clinical team or organisation issues the pathway. That is
//! institutional rather than personal information, but it is still
//! worth an obligation-driven redaction: a cross-department reader of a
//! `sensitive_setting` pathway (mental health, palliative —
//! [`crate::auth::care_pathway_resource_attrs`]) may legitimately see
//! that the pathway exists and what it contains without learning
//! *which team* is delivering it.
//!
//! **The actually patient-identifying fact lives elsewhere.** A specific
//! person's enrolment on a pathway — `pathway_instances.subject_ref`, a
//! `person:<uuid>` reference — is the clinical fact analogous to the
//! `case ↔ person` `subject_of` edge
//! (`agents/share/cross-service-linking.md` §10), and is not a field on
//! this module's `CarePathway` at all. Masking/authorizing that linkage
//! is out of scope for this module and is tracked as a follow-up
//! (spec §16) rather than silently left undone.
//!
//! ## Consent
//!
//! As with organization, the shared privacy contract's consent model
//! ([`agents/share/privacy.md`]) is about a **data subject** granting a
//! purpose. A pathway *template* is not a data subject and names none;
//! this crate has no consent model for the same reason organization does
//! not — a second, unauthoritative home for a person's consent would be
//! worse than none. The subject of an *instance*'s consent, if any, is
//! the person service's to record.
//!
//! Everything here is pure; the endpoints live in
//! [`crate::controllers::care_pathways`].
//!
//! [`CarePathway`]: care_pathway_matcher::CarePathway
//! [`agents/share/privacy.md`]: ../../../agents/share/privacy.md

use care_pathway_matcher::CarePathway;

/// Return a copy of `pathway` with the sensitive fields masked (see the
/// module docs for what counts as sensitive here and why the set is
/// thin).
///
/// Masking keeps the last four characters of a value visible so an
/// operator can still tell two records apart on a review screen.
#[must_use]
pub fn mask_pathway(pathway: &CarePathway) -> CarePathway {
    let mut masked = pathway.clone();
    if let Some(ref name) = masked.provider_name {
        masked.provider_name = Some(mask_value(name, 4));
    }
    if let Some(ref id) = masked.provider_id {
        masked.provider_id = Some(mask_value(id, 4));
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

/// Build the GDPR right-of-access payload for one care pathway.
///
/// An **envelope**, not the bare record: an access request has to be
/// answerable as "here is everything held about this subject, as of
/// then", so the response states what was exported, when, and whether
/// any of it was redacted. `masked` is the honest part — a masked export
/// is a *partial* answer, and the caller must be able to tell which they
/// received.
#[must_use]
pub fn export_pathway(pathway: &CarePathway, pid: &str, masked: bool) -> serde_json::Value {
    let record = if masked {
        mask_pathway(pathway)
    } else {
        pathway.clone()
    };
    serde_json::json!({
        "entity": "care_pathway",
        "pid": pid,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "masked": masked,
        "record": serde_json::to_value(&record).unwrap_or(serde_json::Value::Null),
        "note": if masked {
            "Partial export: provider name and provider id are redacted for this caller."
        } else {
            "Complete export of the stored care pathway record."
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use care_pathway_matcher::{CodeSystem, ConditionCode};

    /// A fully-populated pathway to redact.
    fn pathway() -> CarePathway {
        CarePathway {
            provider_name: Some("Royal Infirmary".into()),
            provider_id: Some("organization:9a2f1e2a-0000".into()),
            pathway_code: Some("STROKE-01".into()),
            condition_codes: vec![ConditionCode {
                system: CodeSystem::Icd10,
                code: "I63".into(),
            }],
            interventions: vec!["thrombolysis".into()],
            keywords: vec!["hyperacute".into()],
            ..CarePathway::new("Acute Stroke Care Pathway")
        }
    }

    /// The provider name and id keep their last four characters and
    /// hide everything before that — enough to tell two records apart
    /// on a review screen, not enough to identify the team.
    #[test]
    fn provider_identity_is_masked_to_its_tail() {
        let masked = mask_pathway(&pathway());
        let name = masked.provider_name.expect("provider_name");
        assert!(name.ends_with("mary"), "keeps the last four chars: {name}");
        assert_ne!(name, "Royal Infirmary", "must actually be redacted");
        assert!(name.contains('*'), "hides the rest: {name}");

        let id = masked.provider_id.expect("provider_id");
        assert!(id.ends_with("0000"), "keeps the last four chars: {id}");
        assert_ne!(
            id, "organization:9a2f1e2a-0000",
            "must actually be redacted"
        );
        assert!(id.contains('*'), "hides the rest: {id}");
    }

    /// The clinical content is never masked — redacting it would defeat
    /// the registry for no privacy gain, since a pathway template names
    /// no patient.
    #[test]
    fn clinical_content_survives_masking() {
        let masked = mask_pathway(&pathway());
        assert_eq!(masked.name, "Acute Stroke Care Pathway");
        assert_eq!(masked.condition_codes.len(), 1);
        assert_eq!(masked.condition_codes[0].code, "I63");
        assert_eq!(masked.interventions, vec!["thrombolysis".to_string()]);
        assert_eq!(masked.keywords, vec!["hyperacute".to_string()]);
        assert_eq!(masked.pathway_code.as_deref(), Some("STROKE-01"));
    }

    /// Masking a pathway with no provider recorded is a no-op rather
    /// than an error or an invented value.
    #[test]
    fn a_bare_pathway_is_unchanged() {
        let bare = CarePathway::new("Bare Pathway");
        let masked = mask_pathway(&bare);
        assert_eq!(masked.name, "Bare Pathway");
        assert_eq!(masked.provider_name, None);
        assert_eq!(masked.provider_id, None);
    }

    /// Values no longer than the visible tail are left alone, and
    /// masking counts characters rather than bytes.
    #[test]
    fn mask_value_is_char_safe() {
        assert_eq!(mask_value("STROKE-01", 4), "*****E-01");
        assert_eq!(mask_value("abc", 4), "abc");
        assert_eq!(mask_value("naïve12", 4), "***ve12");
    }

    /// The export envelope names the subject, says when, and — the part
    /// that matters — says whether what it carries was redacted.
    #[test]
    fn export_envelope_declares_whether_it_is_partial() {
        let full = export_pathway(&pathway(), "pid-1", false);
        assert_eq!(full["entity"], "care_pathway");
        assert_eq!(full["pid"], "pid-1");
        assert_eq!(full["masked"], false);
        assert_eq!(full["record"]["provider_name"], "Royal Infirmary");
        assert!(full["exported_at"].as_str().is_some_and(|s| !s.is_empty()));

        let partial = export_pathway(&pathway(), "pid-1", true);
        assert_eq!(partial["masked"], true);
        let masked_name = partial["record"]["provider_name"].as_str().unwrap();
        assert!(masked_name.ends_with("mary") && masked_name.contains('*'));
        assert!(
            partial["note"]
                .as_str()
                .is_some_and(|n| n.contains("Partial")),
            "a masked export must say so"
        );
    }
}
