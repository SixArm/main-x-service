//! Backward-compatibility shims for the **2026-07-23 rename**
//! (human capital management / `HCM` → workforce planning management /
//! `WPM`).
//!
//! The rename changed two things a deployment cannot see coming:
//!
//! 1. the environment-variable prefix (`HCM_REQUIRE_AUTH` →
//!    `WPM_REQUIRE_AUTH`, and so on), and
//! 2. the ABAC resource entity (`"hcm"` → `"wpm"`), which a mounted
//!    policy may key its rules on.
//!
//! Renaming those without a shim would silently change behaviour on an
//! existing deployment — the worst failure mode available here, because
//! **both** of them fail *open-ish*: a `HCM_REQUIRE_AUTH=1` that stops
//! being read turns authentication **off**, and a policy rule keyed on
//! `entity: "hcm"` that stops matching falls through to the default
//! decision. A config that no longer applies must not look like a
//! config that says "allow".
//!
//! So both are accepted, once, with a loud deprecation warning:
//!
//! - [`env_var`] reads the `WPM_*` name and falls back to the legacy
//!   `HCM_*` name.
//! - [`migrate_policy_entity`] rewrites `entity: "hcm"` conditions in a
//!   mounted policy to `"wpm"` before it is parsed.
//!
//! Both are **transitional**. They exist so a deployment survives the
//! rename across one release, not forever; see the removal note on each.

use std::collections::HashSet;
use std::sync::Mutex;

/// The current environment-variable prefix.
pub const PREFIX: &str = "WPM_";

/// The pre-rename environment-variable prefix, still honoured as a
/// deprecated fallback.
pub const LEGACY_PREFIX: &str = "HCM_";

/// The current ABAC resource-entity name (see
/// [`crate::auth::ENTITY`]).
pub const ENTITY: &str = "wpm";

/// The pre-rename ABAC resource-entity name, still honoured in a
/// mounted policy.
pub const LEGACY_ENTITY: &str = "hcm";

/// The legacy spelling of a `WPM_*` variable, or `None` when `name` is
/// not one of ours (so an unrelated variable is never rewritten).
///
/// Pure, so the mapping is unit-tested without touching process env.
#[must_use]
pub fn legacy_name(name: &str) -> Option<String> {
    name.strip_prefix(PREFIX)
        .map(|rest| format!("{LEGACY_PREFIX}{rest}"))
}

/// Names already warned about, so a deprecation notice is logged once
/// per variable rather than on every read (several of these are read
/// per request behind a `OnceLock`, and a per-read warning would drown
/// the log it is trying to be noticed in).
static WARNED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Log the deprecation notice for `legacy`, at most once per process.
fn warn_once(legacy: &str, current: &str) {
    let mut guard = WARNED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let seen = guard.get_or_insert_with(HashSet::new);
    if seen.insert(legacy.to_string()) {
        tracing::warn!(
            legacy_variable = legacy,
            current_variable = current,
            "using the deprecated pre-rename environment variable; \
             rename it to the current one — the fallback will be removed"
        );
    }
}

/// Read a `WPM_*` environment variable, falling back to its legacy
/// `HCM_*` spelling.
///
/// Blank or whitespace-only counts as unset at both names, matching the
/// rest of the crate's env handling. Using the legacy name logs a
/// deprecation warning once per variable per process.
///
/// **Removal**: delete this indirection (and call `std::env::var`
/// directly) once no deployment sets `HCM_*`.
#[must_use]
pub fn env_var(name: &str) -> Option<String> {
    let nonblank = |v: &String| !v.trim().is_empty();

    if let Some(value) = std::env::var(name).ok().filter(nonblank) {
        return Some(value);
    }
    let legacy = legacy_name(name)?;
    let value = std::env::var(&legacy).ok().filter(nonblank)?;
    warn_once(&legacy, name);
    Some(value)
}

/// Rewrite legacy `entity: "hcm"` conditions in a mounted ABAC policy
/// to the current entity name, returning `true` when anything changed.
///
/// A policy rule may key on the `entity` pseudo-attribute
/// (`authorization-attributes.md` §2). After the rename, a rule written
/// as `{"when": {"entity": ["hcm"]}}` would simply stop matching — and
/// a rule that stops matching is *invisible*: the decision quietly
/// falls through to the default instead of erroring. Rewriting it (with
/// a warning) keeps a mounted policy meaning what its author intended.
///
/// Only the `entity` key is touched, and only where its value is
/// exactly the legacy name — a subject attribute that happens to hold
/// the string `"hcm"` is left alone.
///
/// Pure (operates on the parsed JSON), so the walk is unit-tested.
///
/// **Removal**: delete once no mounted policy names the old entity.
pub fn migrate_policy_entity(policy: &mut serde_json::Value) -> bool {
    let mut changed = false;
    let Some(rules) = policy.get_mut("rules").and_then(|r| r.as_array_mut()) else {
        return false;
    };
    for rule in rules {
        let Some(when) = rule.get_mut("when").and_then(|w| w.as_object_mut()) else {
            continue;
        };
        let Some(entity) = when.get_mut("entity") else {
            continue;
        };
        match entity {
            serde_json::Value::String(value) if value == LEGACY_ENTITY => {
                *value = ENTITY.to_string();
                changed = true;
            }
            serde_json::Value::Array(values) => {
                for value in values.iter_mut() {
                    if value.as_str() == Some(LEGACY_ENTITY) {
                        *value = serde_json::Value::String(ENTITY.to_string());
                        changed = true;
                    }
                }
            }
            _ => {}
        }
    }
    if changed {
        tracing::warn!(
            legacy_entity = LEGACY_ENTITY,
            current_entity = ENTITY,
            "the mounted ABAC policy names the pre-rename entity; it was accepted and \
             rewritten — update the policy, the fallback will be removed"
        );
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Only our own prefix is mapped; an unrelated variable is never
    /// given a legacy alias.
    #[test]
    fn legacy_names_map_only_our_prefix() {
        assert_eq!(
            legacy_name("WPM_REQUIRE_AUTH").as_deref(),
            Some("HCM_REQUIRE_AUTH")
        );
        assert_eq!(
            legacy_name("WPM_ABAC_POLICY_FILE").as_deref(),
            Some("HCM_ABAC_POLICY_FILE")
        );
        assert_eq!(legacy_name("WPM_"), Some("HCM_".to_string()));

        assert_eq!(legacy_name("DATABASE_URL"), None);
        assert_eq!(legacy_name("RUST_LOG"), None);
        assert_eq!(legacy_name("HCM_REQUIRE_AUTH"), None, "no reverse mapping");
        assert_eq!(legacy_name("XWPM_FOO"), None, "prefix must be at the start");
    }

    /// A rule keyed on the legacy entity is rewritten, in both the
    /// array and bare-string forms.
    #[test]
    fn policy_entity_is_migrated() {
        let mut policy = json!({
            "rules": [
                { "effect": "allow", "actions": ["read"], "when": { "entity": ["hcm"] } },
                { "effect": "deny", "actions": ["write"], "when": { "entity": "hcm" } },
            ]
        });
        assert!(migrate_policy_entity(&mut policy));
        assert_eq!(policy["rules"][0]["when"]["entity"], json!(["wpm"]));
        assert_eq!(policy["rules"][1]["when"]["entity"], json!("wpm"));
    }

    /// A mixed list keeps its other entries; a policy already on the
    /// new name is left untouched and reports no change.
    #[test]
    fn migration_is_narrow_and_idempotent() {
        let mut mixed = json!({
            "rules": [{ "when": { "entity": ["hcm", "person"] } }]
        });
        assert!(migrate_policy_entity(&mut mixed));
        assert_eq!(
            mixed["rules"][0]["when"]["entity"],
            json!(["wpm", "person"])
        );

        // Already migrated ⇒ no change, no warning.
        assert!(!migrate_policy_entity(&mut mixed));

        let mut current = json!({ "rules": [{ "when": { "entity": ["wpm"] } }] });
        assert!(!migrate_policy_entity(&mut current));
    }

    /// Only the `entity` key is rewritten — a *subject* attribute whose
    /// value happens to be `"hcm"` (a department, a tenant, a service
    /// name) must survive untouched.
    #[test]
    fn only_the_entity_key_is_touched() {
        let mut policy = json!({
            "rules": [{
                "effect": "allow",
                "when": { "dept": ["hcm"], "svc": ["hcm"], "entity": ["hcm"] }
            }]
        });
        assert!(migrate_policy_entity(&mut policy));
        let when = &policy["rules"][0]["when"];
        assert_eq!(when["entity"], json!(["wpm"]), "the entity is migrated");
        assert_eq!(when["dept"], json!(["hcm"]), "a subject attribute is not");
        assert_eq!(when["svc"], json!(["hcm"]), "a subject attribute is not");
    }

    /// Shapes that carry no entity condition are handled without
    /// panicking (the policy is operator-supplied JSON — invariant 2).
    #[test]
    fn malformed_or_empty_policies_are_safe() {
        for mut value in [
            json!({}),
            json!({ "rules": "not-an-array" }),
            json!({ "rules": [] }),
            json!({ "rules": [{}] }),
            json!({ "rules": [{ "when": "not-an-object" }] }),
            json!({ "rules": [{ "when": { "entity": 7 } }] }),
            json!(null),
        ] {
            assert!(!migrate_policy_entity(&mut value));
        }
    }
}
