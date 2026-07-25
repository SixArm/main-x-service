//! Payload validation → `422` (family convention). Bounded-input
//! rules (security invariant 3) plus the token vocabularies from
//! [`crate::flow::tokens`] and `EntityRef` URN shape checks.

use std::str::FromStr;

use entity_ref::{EntityRef, EntityType};

/// Maximum length of any free-text field (security invariant 3).
pub const MAX_TEXT_LEN: usize = 1024;
/// Maximum entries in any list field (e.g. `alerts`).
pub const MAX_ARRAY_LEN: usize = 256;
/// Maximum length of one list entry.
pub const MAX_ITEM_LEN: usize = 512;

/// Accumulates human-readable validation problems.
#[derive(Debug, Default)]
pub struct Problems(Vec<String>);

impl Problems {
    /// Start an empty problem list.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a problem.
    pub fn push(&mut self, problem: impl Into<String>) {
        self.0.push(problem.into());
    }

    /// `field` must be non-blank and within [`MAX_TEXT_LEN`].
    pub fn require_text(&mut self, field: &str, value: &str) {
        if value.trim().is_empty() {
            self.push(format!("{field} is required"));
        }
        self.cap_text(field, value);
    }

    /// `field`, when present, must be within [`MAX_TEXT_LEN`].
    pub fn cap_text(&mut self, field: &str, value: &str) {
        if value.len() > MAX_TEXT_LEN {
            self.push(format!("{field} exceeds {MAX_TEXT_LEN} characters"));
        }
    }

    /// Optional variant of [`Self::cap_text`].
    pub fn cap_opt(&mut self, field: &str, value: Option<&str>) {
        if let Some(v) = value {
            self.cap_text(field, v);
        }
    }

    /// `field` must be a member of the closed token `set`.
    pub fn require_token(&mut self, field: &str, set: &[&str], value: &str) {
        if !crate::flow::tokens::is_token(set, value) {
            self.push(format!("{field} must be one of {set:?}, got {value:?}"));
        }
    }

    /// Optional variant of [`Self::require_token`].
    pub fn token_opt(&mut self, field: &str, set: &[&str], value: Option<&str>) {
        if let Some(v) = value {
            self.require_token(field, set, v);
        }
    }

    /// `field` must be a well-formed `EntityRef` URN of `expected` type.
    pub fn require_ref(&mut self, field: &str, expected: EntityType, value: &str) {
        match EntityRef::from_str(value) {
            Ok(r) if r.entity_type == expected => {}
            Ok(r) => self.push(format!(
                "{field} must reference a {expected:?}, got {:?}",
                r.entity_type
            )),
            Err(e) => self.push(format!("{field} is not a valid EntityRef URN: {e}")),
        }
    }

    /// Optional variant of [`Self::require_ref`].
    pub fn ref_opt(&mut self, field: &str, expected: EntityType, value: Option<&str>) {
        if let Some(v) = value {
            self.require_ref(field, expected, v);
        }
    }

    /// A string list must respect the array and per-item caps, with no
    /// blank entries.
    pub fn cap_list(&mut self, field: &str, values: &[String]) {
        if values.len() > MAX_ARRAY_LEN {
            self.push(format!("{field} exceeds {MAX_ARRAY_LEN} entries"));
        }
        for v in values {
            if v.trim().is_empty() {
                self.push(format!("{field} entries must be non-blank"));
            }
            if v.len() > MAX_ITEM_LEN {
                self.push(format!("{field} entry exceeds {MAX_ITEM_LEN} characters"));
            }
        }
    }

    /// The collected problems (empty ⇒ valid).
    #[must_use]
    pub fn into_vec(self) -> Vec<String> {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_rules() {
        let mut p = Problems::new();
        p.require_text("name", "Ward 7");
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.require_text("name", "   ");
        p.cap_text("note", &"x".repeat(MAX_TEXT_LEN + 1));
        let v = p.into_vec();
        assert_eq!(v.len(), 2);
        assert!(v[0].contains("required"));
        assert!(v[1].contains("exceeds"));
    }

    #[test]
    fn token_rules() {
        let mut p = Problems::new();
        p.require_token("kind", crate::flow::tokens::WARD_KINDS, "virtual");
        p.token_opt("priority", crate::flow::tokens::REQUEST_PRIORITIES, None);
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.require_token("kind", crate::flow::tokens::WARD_KINDS, "icu");
        assert_eq!(p.into_vec().len(), 1);
    }

    #[test]
    fn ref_rules() {
        let good = format!("person:{}", uuid::Uuid::new_v4());
        let mut p = Problems::new();
        p.require_ref("person_ref", EntityType::Person, &good);
        assert!(p.into_vec().is_empty());
        // Wrong type, malformed, and bad uuid each fail.
        let mut p = Problems::new();
        p.require_ref(
            "person_ref",
            EntityType::Person,
            &format!("worker:{}", uuid::Uuid::new_v4()),
        );
        p.require_ref("person_ref", EntityType::Person, "not-a-urn");
        p.require_ref("person_ref", EntityType::Person, "person:nope");
        assert_eq!(p.into_vec().len(), 3);
    }

    #[test]
    fn list_rules() {
        let mut p = Problems::new();
        p.cap_list("alerts", &["falls risk".to_string()]);
        assert!(p.into_vec().is_empty());
        let mut p = Problems::new();
        p.cap_list("alerts", &[String::new()]);
        assert_eq!(p.into_vec().len(), 1);
    }
}
